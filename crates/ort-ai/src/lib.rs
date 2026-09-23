//! Direct-AI provider, catalog, accounting, monitoring, and guardrail boundary.

pub mod materials;

use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    sync::Mutex,
};
use uuid::Uuid;
use zeroize::Zeroize;

pub const ENABLED: bool = true;
pub const MAX_STREAM_BYTES: usize = 2 * 1_024 * 1_024;
pub const MAX_OUTPUT_TOKENS: u32 = 6_000;
pub const BUILTIN_CATALOG_SIGNATURE: &str =
    "avhdxt50Whqyy4VLxQookQGs6z0BYoEm6dqQCYj9X2ZJsVkfwIKmYghntYLktzEVLYPnryaHs+VmRGJLzOpDAA==";
pub const BUILTIN_CATALOG_PUBLIC_KEY: [u8; 32] = [
    0x88, 0x5b, 0x5c, 0x48, 0x56, 0x72, 0xf2, 0x53, 0x79, 0x77, 0x5b, 0xc2, 0x97, 0x5b, 0x8c, 0x5e,
    0x22, 0x24, 0x97, 0xf5, 0x48, 0xcf, 0xee, 0x5d, 0x09, 0x71, 0x50, 0xc1, 0xc5, 0x6d, 0x86, 0xbd,
];
pub const BUILTIN_CATALOG_BYTES: &[u8] = include_bytes!("../../../packages/catalog/direct-v1.json");

/// Opens the signed catalog shipped with this build.
/// # Errors
/// Uses the same trust and freshness checks as a downloaded content-only update.
pub fn builtin_catalog(now: &str, previous_id: Option<&str>) -> Result<Catalog, AiError> {
    Catalog::verify(
        BUILTIN_CATALOG_BYTES,
        BUILTIN_CATALOG_SIGNATURE,
        &BUILTIN_CATALOG_PUBLIC_KEY,
        now,
        env!("CARGO_PKG_VERSION"),
        previous_id,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    OpenAi,
    Anthropic,
    Gemini,
}
impl Provider {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Economy,
    Balanced,
    Quality,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    TailorResume,
    RefineResume,
    CoverLetter,
    AnswerQuestion,
    ImportMapping,
    CredentialTest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Reserved,
    Dispatching,
    Streaming,
    Succeeded,
    Failed,
    Cancelled,
    OutcomeUnknown,
}
impl AttemptStatus {
    const fn terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::OutcomeUnknown
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Usage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_write_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
}
impl Usage {
    fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            input_tokens: self.input_tokens.checked_add(other.input_tokens)?,
            cached_input_tokens: self
                .cached_input_tokens
                .checked_add(other.cached_input_tokens)?,
            cache_write_tokens: self
                .cache_write_tokens
                .checked_add(other.cache_write_tokens)?,
            output_tokens: self.output_tokens.checked_add(other.output_tokens)?,
            reasoning_tokens: self.reasoning_tokens.checked_add(other.reasoning_tokens)?,
        })
    }
}

/// Secret material whose formatting is always redacted and whose buffer clears on drop.
pub struct ApiKey(Vec<u8>);
impl ApiKey {
    /// # Errors
    /// Rejects empty, oversized, non-UTF-8, or control-containing credentials.
    pub fn new(value: &[u8]) -> Result<Self, AiError> {
        if value.is_empty()
            || value.len() > 8_192
            || std::str::from_utf8(value).is_err()
            || value.iter().any(u8::is_ascii_control)
        {
            return Err(AiError::InvalidCredential);
        }
        Ok(Self(value.to_vec()))
    }
    fn text(&self) -> Result<&str, AiError> {
        std::str::from_utf8(&self.0).map_err(|_| AiError::InvalidCredential)
    }
}
impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey([REDACTED])")
    }
}
impl Drop for ApiKey {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AiError {
    #[error("the provider credential is invalid")]
    InvalidCredential,
    #[error("the provider or model is not in the trusted catalog")]
    UnsupportedModel,
    #[error("the catalog is malformed, incompatible, expired, or untrusted")]
    InvalidCatalog,
    #[error("the provider response was malformed or exceeded its bound")]
    InvalidResponse,
    #[error("another remote operation is active")]
    OperationBusy,
    #[error("the requested record was not found")]
    NotFound,
    #[error("the operation state transition is invalid")]
    InvalidState,
    #[error("an enabled spending cap would be exceeded")]
    CapExceeded,
    #[error("cost cannot be bounded under an enabled cap")]
    CostUnavailable,
    #[error("accounting arithmetic overflowed")]
    Arithmetic,
    #[error("local accounting is unavailable")]
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceCategory {
    Input,
    CachedInput,
    CacheWrite,
    Output,
    Reasoning,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Price {
    pub micros_per_million: u64,
    pub category: PriceCategory,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogEntry {
    pub provider: Provider,
    pub model: String,
    pub preset: Preset,
    pub operations: Vec<OperationType>,
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub currency: String,
    pub prices: Vec<Price>,
    pub source: String,
    pub verified_at: String,
    pub effective_from: String,
    pub effective_to: Option<String>,
    pub disabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalog {
    pub format_version: u16,
    pub catalog_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub minimum_app_version: String,
    pub entries: Vec<CatalogEntry>,
}
impl Catalog {
    /// Verifies Ed25519 over the exact bytes and enforces freshness, compatibility, and rollback checks.
    /// # Errors
    /// Returns `InvalidCatalog` for any trust or semantic failure.
    pub fn verify(
        bytes: &[u8],
        signature_base64: &str,
        public_key: &[u8; 32],
        now: &str,
        app_version: &str,
        previous_id: Option<&str>,
    ) -> Result<Self, AiError> {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(signature_base64)
            .map_err(|_| AiError::InvalidCatalog)?;
        let signature = Signature::from_slice(&raw).map_err(|_| AiError::InvalidCatalog)?;
        VerifyingKey::from_bytes(public_key)
            .map_err(|_| AiError::InvalidCatalog)?
            .verify(bytes, &signature)
            .map_err(|_| AiError::InvalidCatalog)?;
        let catalog: Self = serde_json::from_slice(bytes).map_err(|_| AiError::InvalidCatalog)?;
        if catalog.format_version != 1
            || catalog.entries.is_empty()
            || catalog.catalog_id.is_empty()
            || catalog.issued_at.as_str() > now
            || catalog.expires_at.as_str() <= now
            || version_newer(&catalog.minimum_app_version, app_version)?
            || previous_id.is_some_and(|old| catalog.catalog_id.as_str() <= old)
        {
            return Err(AiError::InvalidCatalog);
        }
        for entry in &catalog.entries {
            if entry.model.is_empty()
                || entry.operations.is_empty()
                || entry.max_input_tokens == 0
                || entry.max_output_tokens == 0
                || entry.max_output_tokens > MAX_OUTPUT_TOKENS
                || entry.currency.len() != 3
                || entry.source.is_empty()
                || entry.verified_at.as_str() > now
                || entry.effective_from.as_str() > now
                || entry.effective_to.as_deref().is_some_and(|end| end <= now)
                || entry.prices.is_empty()
                || entry.prices.len() > 5
                || entry
                    .prices
                    .iter()
                    .any(|price| price.micros_per_million == 0)
                || entry.prices.iter().enumerate().any(|(index, price)| {
                    entry.prices[..index]
                        .iter()
                        .any(|other| other.category == price.category)
                })
            {
                return Err(AiError::InvalidCatalog);
            }
        }
        Ok(catalog)
    }
    /// # Errors
    /// Returns `UnsupportedModel`; selection never silently falls back.
    pub fn resolve(
        &self,
        provider: Provider,
        preset: Preset,
        operation: OperationType,
    ) -> Result<&CatalogEntry, AiError> {
        self.entries
            .iter()
            .find(|entry| {
                entry.provider == provider
                    && entry.preset == preset
                    && entry.operations.contains(&operation)
                    && !entry.disabled
            })
            .ok_or(AiError::UnsupportedModel)
    }
}
fn version_newer(required: &str, current: &str) -> Result<bool, AiError> {
    fn parse(value: &str) -> Result<[u64; 3], AiError> {
        value
            .split_once('-')
            .map_or(value, |p| p.0)
            .split('.')
            .map(str::parse)
            .collect::<Result<Vec<u64>, _>>()
            .map_err(|_| AiError::InvalidCatalog)?
            .try_into()
            .map_err(|_| AiError::InvalidCatalog)
    }
    Ok(parse(required)? > parse(current)?)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedRequest {
    pub operation: OperationType,
    pub model: String,
    pub system: String,
    pub input: Value,
    pub max_output_tokens: u32,
}
pub struct HttpRequest {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
impl fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpRequest")
            .field("url", &self.url)
            .field("headers", &"[REDACTED]")
            .field("body", &"[REDACTED]")
            .finish()
    }
}
impl Drop for HttpRequest {
    fn drop(&mut self) {
        for value in self.headers.values_mut() {
            value.zeroize();
        }
        self.body.zeroize();
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamEvent {
    Text(String),
    Usage(Usage),
    Model(String),
    ProviderFailure,
    Finished,
}
pub trait ProviderAdapter {
    fn provider(&self) -> Provider;
    /// # Errors
    /// Rejects invalid credentials, request bounds, or provider-specific model identifiers.
    fn build_request(
        &self,
        request: &NormalizedRequest,
        key: &ApiKey,
    ) -> Result<HttpRequest, AiError>;
    /// # Errors
    /// Rejects malformed, non-UTF-8, or oversized provider event streams.
    fn parse_stream(&self, bytes: &[u8]) -> Result<Vec<StreamEvent>, AiError>;
}
pub struct OpenAiAdapter;
pub struct AnthropicAdapter;
pub struct GeminiAdapter;
fn request_ok(r: &NormalizedRequest) -> Result<(), AiError> {
    if r.model.is_empty()
        || r.system.len() > 32_000
        || r.max_output_tokens == 0
        || r.max_output_tokens > MAX_OUTPUT_TOKENS
    {
        Err(AiError::InvalidResponse)
    } else {
        Ok(())
    }
}
fn encoded(value: &Value) -> Result<String, AiError> {
    serde_json::to_string(value).map_err(|_| AiError::InvalidResponse)
}
impl ProviderAdapter for OpenAiAdapter {
    fn provider(&self) -> Provider {
        Provider::OpenAi
    }
    fn build_request(&self, r: &NormalizedRequest, key: &ApiKey) -> Result<HttpRequest, AiError> {
        request_ok(r)?;
        Ok(HttpRequest { url: "https://api.openai.com/v1/responses".into(), headers: BTreeMap::from([("authorization".into(), format!("Bearer {}", key.text()?)), ("content-type".into(), "application/json".into())]), body: serde_json::to_vec(&json!({"model":r.model,"instructions":r.system,"input":encoded(&r.input)?,"max_output_tokens":r.max_output_tokens,"stream":true,"store":false,"tools":[],"tool_choice":"none","text":{"format":{"type":"json_object"}}})).map_err(|_| AiError::InvalidResponse)? })
    }
    fn parse_stream(&self, bytes: &[u8]) -> Result<Vec<StreamEvent>, AiError> {
        parse_sse(bytes, |v| match v.get("type").and_then(Value::as_str) {
            Some("response.created") => v
                .pointer("/response/model")
                .and_then(Value::as_str)
                .map(|model| Ok(vec![StreamEvent::Model(model.into())])),
            Some("response.output_text.delta") => v
                .get("delta")
                .and_then(Value::as_str)
                .map(|s| Ok(vec![StreamEvent::Text(s.into())])),
            Some("response.completed") => {
                let Some(usage) = v.pointer("/response/usage") else {
                    return Some(Err(AiError::InvalidResponse));
                };
                Some(
                    openai_usage(usage)
                        .map(|usage| vec![StreamEvent::Usage(usage), StreamEvent::Finished]),
                )
            }
            Some("response.failed" | "response.incomplete") => {
                Some(Ok(vec![StreamEvent::ProviderFailure]))
            }
            _ => None,
        })
    }
}
impl ProviderAdapter for AnthropicAdapter {
    fn provider(&self) -> Provider {
        Provider::Anthropic
    }
    fn build_request(&self, r: &NormalizedRequest, key: &ApiKey) -> Result<HttpRequest, AiError> {
        request_ok(r)?;
        Ok(HttpRequest { url: "https://api.anthropic.com/v1/messages".into(), headers: BTreeMap::from([("x-api-key".into(), key.text()?.into()), ("anthropic-version".into(), "2023-06-01".into()), ("content-type".into(), "application/json".into())]), body: serde_json::to_vec(&json!({"model":r.model,"system":r.system,"messages":[{"role":"user","content":encoded(&r.input)?}],"max_tokens":r.max_output_tokens,"stream":true})).map_err(|_| AiError::InvalidResponse)? })
    }
    fn parse_stream(&self, bytes: &[u8]) -> Result<Vec<StreamEvent>, AiError> {
        let events = parse_sse(bytes, |v| match v.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                let mut events = Vec::new();
                if let Some(model) = v.pointer("/message/model").and_then(Value::as_str) {
                    events.push(StreamEvent::Model(model.into()));
                }
                if let Some(usage) = v.pointer("/message/usage") {
                    match anthropic_usage(usage) {
                        Ok(usage) => events.push(StreamEvent::Usage(usage)),
                        Err(error) => return Some(Err(error)),
                    }
                }
                if events.is_empty() {
                    None
                } else {
                    Some(Ok(events))
                }
            }
            Some("content_block_delta") => v
                .pointer("/delta/text")
                .and_then(Value::as_str)
                .map(|s| Ok(vec![StreamEvent::Text(s.into())])),
            Some("message_delta") => v
                .get("usage")
                .map(anthropic_usage)
                .map(|r| r.map(|usage| vec![StreamEvent::Usage(usage)])),
            Some("message_stop") => Some(Ok(vec![StreamEvent::Finished])),
            Some("error") => Some(Ok(vec![StreamEvent::ProviderFailure])),
            _ => None,
        })?;
        let mut combined = Usage::default();
        let mut saw_usage = false;
        let mut normalized = Vec::new();
        for event in events {
            if let StreamEvent::Usage(usage) = event {
                saw_usage = true;
                combined.input_tokens = combined.input_tokens.max(usage.input_tokens);
                combined.cached_input_tokens =
                    combined.cached_input_tokens.max(usage.cached_input_tokens);
                combined.cache_write_tokens =
                    combined.cache_write_tokens.max(usage.cache_write_tokens);
                combined.output_tokens = combined.output_tokens.max(usage.output_tokens);
            } else {
                normalized.push(event);
            }
        }
        if saw_usage {
            let position = normalized
                .iter()
                .position(|event| matches!(event, StreamEvent::Finished))
                .unwrap_or(normalized.len());
            normalized.insert(position, StreamEvent::Usage(combined));
        }
        Ok(normalized)
    }
}
impl ProviderAdapter for GeminiAdapter {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }
    fn build_request(&self, r: &NormalizedRequest, key: &ApiKey) -> Result<HttpRequest, AiError> {
        request_ok(r)?;
        if !r
            .model
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(AiError::UnsupportedModel);
        }
        Ok(HttpRequest { url: format!("https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse", r.model), headers: BTreeMap::from([("x-goog-api-key".into(), key.text()?.into()), ("content-type".into(), "application/json".into())]), body: serde_json::to_vec(&json!({"systemInstruction":{"parts":[{"text":r.system}]},"contents":[{"role":"user","parts":[{"text":encoded(&r.input)?}]}],"generationConfig":{"maxOutputTokens":r.max_output_tokens,"responseMimeType":"application/json"}})).map_err(|_| AiError::InvalidResponse)? })
    }
    fn parse_stream(&self, bytes: &[u8]) -> Result<Vec<StreamEvent>, AiError> {
        parse_sse(bytes, |v| {
            let mut events = Vec::new();
            if let Some(s) = v
                .pointer("/candidates/0/content/parts/0/text")
                .and_then(Value::as_str)
            {
                events.push(StreamEvent::Text(s.into()));
            }
            if let Some(usage) = v.get("usageMetadata") {
                match gemini_usage(usage) {
                    Ok(usage) => events.push(StreamEvent::Usage(usage)),
                    Err(error) => return Some(Err(error)),
                }
            }
            if let Some(model) = v.get("modelVersion").and_then(Value::as_str) {
                events.push(StreamEvent::Model(model.into()));
            }
            if v.pointer("/promptFeedback/blockReason").is_some() {
                events.push(StreamEvent::ProviderFailure);
            }
            if events.is_empty() {
                None
            } else {
                Some(Ok(events))
            }
        })
    }
}
fn parse_sse(
    bytes: &[u8],
    mut parse: impl FnMut(&Value) -> Option<Result<Vec<StreamEvent>, AiError>>,
) -> Result<Vec<StreamEvent>, AiError> {
    if bytes.len() > MAX_STREAM_BYTES {
        return Err(AiError::InvalidResponse);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| AiError::InvalidResponse)?;
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            out.push(StreamEvent::Finished);
            continue;
        }
        let value = serde_json::from_str(data).map_err(|_| AiError::InvalidResponse)?;
        if let Some(event) = parse(&value) {
            out.extend(event?);
        }
    }
    Ok(out)
}
fn required(v: &Value, p: &str) -> Result<u64, AiError> {
    v.pointer(p)
        .and_then(Value::as_u64)
        .ok_or(AiError::InvalidResponse)
}
fn optional(v: &Value, p: &str) -> u64 {
    v.pointer(p).and_then(Value::as_u64).unwrap_or(0)
}
fn openai_usage(v: &Value) -> Result<Usage, AiError> {
    let total = required(v, "/input_tokens")?;
    let cached = optional(v, "/input_tokens_details/cached_tokens");
    let cache_write = optional(v, "/input_tokens_details/cache_write_tokens");
    let output = required(v, "/output_tokens")?;
    let reasoning = optional(v, "/output_tokens_details/reasoning_tokens");
    if reasoning > output {
        return Err(AiError::InvalidResponse);
    }
    Ok(Usage {
        input_tokens: total
            .checked_sub(cached)
            .and_then(|value| value.checked_sub(cache_write))
            .ok_or(AiError::InvalidResponse)?,
        cached_input_tokens: cached,
        cache_write_tokens: cache_write,
        output_tokens: output,
        reasoning_tokens: reasoning,
    })
}
fn anthropic_usage(v: &Value) -> Result<Usage, AiError> {
    Ok(Usage {
        input_tokens: optional(v, "/input_tokens"),
        cached_input_tokens: optional(v, "/cache_read_input_tokens"),
        cache_write_tokens: optional(v, "/cache_creation_input_tokens"),
        output_tokens: required(v, "/output_tokens")?,
        reasoning_tokens: 0,
    })
}
fn gemini_usage(v: &Value) -> Result<Usage, AiError> {
    let total = required(v, "/promptTokenCount")?;
    let cached = optional(v, "/cachedContentTokenCount");
    Ok(Usage {
        input_tokens: total.checked_sub(cached).ok_or(AiError::InvalidResponse)?,
        cached_input_tokens: cached,
        output_tokens: required(v, "/candidatesTokenCount")?,
        reasoning_tokens: optional(v, "/thoughtsTokenCount"),
        cache_write_tokens: 0,
    })
}

/// Integer-micro cost calculation, rounded up per category.
/// # Errors
/// Missing applicable prices fail closed instead of becoming zero.
pub fn estimate_cost(entry: &CatalogEntry, usage: Usage) -> Result<u64, AiError> {
    if entry.provider == Provider::OpenAi && usage.reasoning_tokens > usage.output_tokens {
        return Err(AiError::InvalidResponse);
    }
    let values = [
        (PriceCategory::Input, usage.input_tokens),
        (PriceCategory::CachedInput, usage.cached_input_tokens),
        (PriceCategory::CacheWrite, usage.cache_write_tokens),
        (PriceCategory::Output, usage.output_tokens),
        // OpenAI reports reasoning as a breakdown of output_tokens, which is
        // already priced in full. Gemini reports thinking tokens separately.
        (
            PriceCategory::Reasoning,
            if entry.provider == Provider::OpenAi {
                0
            } else {
                usage.reasoning_tokens
            },
        ),
    ];
    values
        .into_iter()
        .filter(|(_, q)| *q > 0)
        .try_fold(0_u64, |total, (category, quantity)| {
            let rate = entry
                .prices
                .iter()
                .find(|p| p.category == category)
                .ok_or(AiError::CostUnavailable)?
                .micros_per_million;
            let value = quantity
                .checked_mul(rate)
                .and_then(|n| n.checked_add(999_999))
                .ok_or(AiError::Arithmetic)?
                / 1_000_000;
            total.checked_add(value).ok_or(AiError::Arithmetic)
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CapPeriod {
    Week,
    Month,
    Year,
    AllTime,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cap {
    pub period: CapPeriod,
    pub limit_micros: u64,
    pub starts_at_unix: i64,
    pub ends_at_unix: Option<i64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCategory {
    Authentication,
    RateLimit,
    Transient,
    Safety,
    InvalidOutput,
    Timeout,
    Cancelled,
    Provider,
}
impl ErrorCategory {
    #[must_use]
    pub const fn retryable(self) -> bool {
        matches!(self, Self::RateLimit | Self::Transient | Self::Timeout)
    }
}
#[derive(Clone, Debug)]
pub struct Attempt {
    pub id: Uuid,
    pub operation_id: Uuid,
    pub provider: Provider,
    pub credential_id: Uuid,
    pub requested_model: String,
    pub effective_model: Option<String>,
    pub catalog_id: String,
    pub status: AttemptStatus,
    pub started_at_unix: i64,
    pub ended_at_unix: Option<i64>,
    pub reserved_micros: u64,
    pub settled_micros: Option<u64>,
    pub currency: String,
    pub usage: Option<Usage>,
    pub usage_complete: bool,
    pub retry_of: Option<Uuid>,
    pub error: Option<ErrorCategory>,
}
#[derive(Clone, Debug)]
pub struct Operation {
    pub id: Uuid,
    pub kind: OperationType,
    pub started_at_unix: i64,
    pub ended_at_unix: Option<i64>,
    pub cancelled: bool,
}
#[derive(Default)]
struct State {
    operations: HashMap<Uuid, Operation>,
    attempts: HashMap<Uuid, Attempt>,
    caps: HashMap<Uuid, Vec<Cap>>,
    guardrail_carry: HashMap<(Uuid, String), u64>,
}
#[derive(Default)]
pub struct AccountingLedger {
    state: Mutex<State>,
}
impl AccountingLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// # Errors
    /// Rejects malformed or duplicate-period policy.
    pub fn set_caps(&self, credential: Uuid, caps: Vec<Cap>) -> Result<(), AiError> {
        let mut periods = HashSet::new();
        if caps.iter().any(|c| {
            c.limit_micros == 0
                || !periods.insert(c.period)
                || c.ends_at_unix.is_some_and(|e| e <= c.starts_at_unix)
        }) {
            return Err(AiError::InvalidState);
        }
        self.state
            .lock()
            .map_err(|_| AiError::Unavailable)?
            .caps
            .insert(credential, caps);
        Ok(())
    }
    /// # Errors
    /// Enforces one active remote operation.
    pub fn begin_operation(&self, kind: OperationType, now: i64) -> Result<Uuid, AiError> {
        let mut s = self.state.lock().map_err(|_| AiError::Unavailable)?;
        if s.operations.values().any(|o| o.ended_at_unix.is_none()) {
            return Err(AiError::OperationBusy);
        }
        let id = Uuid::now_v7();
        s.operations.insert(
            id,
            Operation {
                id,
                kind,
                started_at_unix: now,
                ended_at_unix: None,
                cancelled: false,
            },
        );
        Ok(id)
    }
    /// Atomically records the reservation before dispatch.
    /// # Errors
    /// Enabled caps reject over-budget or unpriceable calls.
    #[allow(clippy::too_many_arguments)]
    pub fn reserve(
        &self,
        operation: Uuid,
        provider: Provider,
        credential: Uuid,
        model: &str,
        catalog: &str,
        currency: &str,
        maximum: u64,
        now: i64,
        retry_of: Option<Uuid>,
    ) -> Result<Uuid, AiError> {
        let mut s = self.state.lock().map_err(|_| AiError::Unavailable)?;
        if s.operations
            .get(&operation)
            .is_none_or(|o| o.ended_at_unix.is_some())
        {
            return Err(AiError::NotFound);
        }
        if model.is_empty() || catalog.is_empty() || currency.len() != 3 || maximum == 0 {
            return Err(AiError::CostUnavailable);
        }
        let mut counted = *s
            .guardrail_carry
            .get(&(credential, currency.into()))
            .unwrap_or(&0);
        for a in s
            .attempts
            .values()
            .filter(|a| a.credential_id == credential && a.currency == currency)
        {
            counted = counted
                .checked_add(a.settled_micros.unwrap_or(a.reserved_micros))
                .ok_or(AiError::Arithmetic)?;
        }
        for cap in s
            .caps
            .get(&credential)
            .into_iter()
            .flatten()
            .filter(|c| c.starts_at_unix <= now && c.ends_at_unix.is_none_or(|e| now < e))
        {
            if counted.checked_add(maximum).ok_or(AiError::Arithmetic)? > cap.limit_micros {
                return Err(AiError::CapExceeded);
            }
        }
        let id = Uuid::now_v7();
        s.attempts.insert(
            id,
            Attempt {
                id,
                operation_id: operation,
                provider,
                credential_id: credential,
                requested_model: model.into(),
                effective_model: None,
                catalog_id: catalog.into(),
                status: AttemptStatus::Reserved,
                started_at_unix: now,
                ended_at_unix: None,
                reserved_micros: maximum,
                settled_micros: None,
                currency: currency.into(),
                usage: None,
                usage_complete: false,
                retry_of,
                error: None,
            },
        );
        Ok(id)
    }
    /// # Errors
    /// Only reserved→dispatching→streaming transitions are accepted.
    pub fn transition(&self, id: Uuid, next: AttemptStatus) -> Result<(), AiError> {
        let mut s = self.state.lock().map_err(|_| AiError::Unavailable)?;
        let a = s.attempts.get_mut(&id).ok_or(AiError::NotFound)?;
        if !matches!(
            (a.status, next),
            (AttemptStatus::Reserved, AttemptStatus::Dispatching)
                | (AttemptStatus::Dispatching, AttemptStatus::Streaming)
        ) {
            return Err(AiError::InvalidState);
        }
        a.status = next;
        Ok(())
    }
    /// Unknown outcomes retain their full reservation.
    /// # Errors
    /// Terminal or pre-dispatch attempts cannot be settled twice.
    #[allow(clippy::too_many_arguments)]
    pub fn settle(
        &self,
        id: Uuid,
        status: AttemptStatus,
        effective: Option<&str>,
        usage: Option<Usage>,
        actual: Option<u64>,
        complete: bool,
        error: Option<ErrorCategory>,
        now: i64,
    ) -> Result<(), AiError> {
        if !status.terminal() {
            return Err(AiError::InvalidState);
        }
        let mut s = self.state.lock().map_err(|_| AiError::Unavailable)?;
        let a = s.attempts.get_mut(&id).ok_or(AiError::NotFound)?;
        if a.status.terminal()
            || a.status == AttemptStatus::Reserved
            || status == AttemptStatus::Succeeded && (effective.is_none() || usage.is_none())
        {
            return Err(AiError::InvalidState);
        }
        a.status = status;
        a.effective_model = effective.map(str::to_owned);
        a.usage = usage;
        a.usage_complete = complete;
        a.error = error;
        a.ended_at_unix = Some(now);
        if status != AttemptStatus::OutcomeUnknown {
            a.settled_micros = actual;
        }
        Ok(())
    }
    /// # Errors
    /// Missing operations cannot be cancelled.
    pub fn cancel(&self, id: Uuid) -> Result<(), AiError> {
        self.state
            .lock()
            .map_err(|_| AiError::Unavailable)?
            .operations
            .get_mut(&id)
            .ok_or(AiError::NotFound)?
            .cancelled = true;
        Ok(())
    }
    /// # Errors
    /// An operation with an in-flight attempt cannot finish.
    pub fn finish_operation(&self, id: Uuid, now: i64) -> Result<(), AiError> {
        let mut s = self.state.lock().map_err(|_| AiError::Unavailable)?;
        if s.attempts
            .values()
            .any(|a| a.operation_id == id && !a.status.terminal())
        {
            return Err(AiError::InvalidState);
        }
        s.operations
            .get_mut(&id)
            .ok_or(AiError::NotFound)?
            .ended_at_unix = Some(now);
        Ok(())
    }
    /// Converts crash-interrupted calls to conservative unknown outcomes.
    /// # Errors
    /// Returns `Unavailable` if the ledger lock is poisoned.
    pub fn recover_interrupted(&self, now: i64) -> Result<usize, AiError> {
        let mut s = self.state.lock().map_err(|_| AiError::Unavailable)?;
        let mut n = 0;
        for a in s.attempts.values_mut().filter(|a| {
            matches!(
                a.status,
                AttemptStatus::Dispatching | AttemptStatus::Streaming
            )
        }) {
            a.status = AttemptStatus::OutcomeUnknown;
            a.ended_at_unix = Some(now);
            a.error = Some(ErrorCategory::Provider);
            n += 1;
        }
        for o in s
            .operations
            .values_mut()
            .filter(|o| o.ended_at_unix.is_none())
        {
            o.ended_at_unix = Some(now);
        }
        Ok(n)
    }
    /// Clears monitoring history while carrying counted spend forward separately.
    /// # Errors
    /// Rejects an invalid range.
    pub fn clear_activity(&self, from: i64, to: i64) -> Result<usize, AiError> {
        if from >= to {
            return Err(AiError::InvalidState);
        }
        let mut s = self.state.lock().map_err(|_| AiError::Unavailable)?;
        let ids: Vec<_> = s
            .attempts
            .values()
            .filter(|a| a.started_at_unix >= from && a.started_at_unix < to && a.status.terminal())
            .map(|a| a.id)
            .collect();
        for id in &ids {
            if let Some(a) = s.attempts.remove(id) {
                let key = (a.credential_id, a.currency);
                let value = a.settled_micros.unwrap_or(a.reserved_micros);
                let old = s.guardrail_carry.get(&key).copied().unwrap_or(0);
                s.guardrail_carry
                    .insert(key, old.checked_add(value).ok_or(AiError::Arithmetic)?);
            }
        }
        Ok(ids.len())
    }
    /// # Errors
    /// Aggregation overflow or unavailable state is reported.
    pub fn summary(&self, from: i64, to: i64) -> Result<MonitoringSummary, AiError> {
        let s = self.state.lock().map_err(|_| AiError::Unavailable)?;
        let rows: Vec<_> = s
            .attempts
            .values()
            .filter(|a| a.started_at_unix >= from && a.started_at_unix < to)
            .collect();
        let usage = rows
            .iter()
            .filter_map(|a| a.usage)
            .try_fold(Usage::default(), Usage::checked_add)
            .ok_or(AiError::Arithmetic)?;
        let cost = rows
            .iter()
            .filter_map(|a| a.settled_micros)
            .try_fold(0_u64, u64::checked_add)
            .ok_or(AiError::Arithmetic)?;
        let unknown = rows
            .iter()
            .filter(|a| !a.usage_complete || a.settled_micros.is_none())
            .count();
        Ok(MonitoringSummary {
            logical_operations: rows
                .iter()
                .map(|a| a.operation_id)
                .collect::<HashSet<_>>()
                .len(),
            attempts: rows.len(),
            usage,
            estimated_micros: cost,
            partial: unknown > 0,
            unknown_count: unknown,
        })
    }
    /// Ordinary export contains aggregate metadata only.
    /// # Errors
    /// Propagates summary errors.
    pub fn export_csv(&self, from: i64, to: i64) -> Result<String, AiError> {
        let s = self.summary(from, to)?;
        Ok(format!(
            "from_unix,to_unix,logical_operations,attempts,input_tokens,cached_input_tokens,cache_write_tokens,output_tokens,reasoning_tokens,estimated_cost_micros,partial,unknown_count\n{from},{to},{},{},{},{},{},{},{},{},{},{}\n",
            s.logical_operations,
            s.attempts,
            s.usage.input_tokens,
            s.usage.cached_input_tokens,
            s.usage.cache_write_tokens,
            s.usage.output_tokens,
            s.usage.reasoning_tokens,
            s.estimated_micros,
            s.partial,
            s.unknown_count
        ))
    }
    #[must_use]
    pub fn should_retry(&self, id: Uuid) -> bool {
        let Ok(s) = self.state.lock() else {
            return false;
        };
        let Some(a) = s.attempts.get(&id) else {
            return false;
        };
        a.error.is_some_and(ErrorCategory::retryable) && a.retry_of.is_none()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitoringSummary {
    pub logical_operations: usize,
    pub attempts: usize,
    pub usage: Usage,
    pub estimated_micros: u64,
    pub partial: bool,
    pub unknown_count: usize,
}

#[cfg(test)]
mod tests;
