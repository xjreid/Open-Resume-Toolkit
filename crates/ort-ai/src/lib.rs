//! Direct-AI provider, catalog, accounting, monitoring, and guardrail boundary.

pub mod materials;
pub mod plan;

use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fmt};
use zeroize::Zeroize;

pub const ENABLED: bool = true;
pub const MAX_STREAM_BYTES: usize = 2 * 1_024 * 1_024;
pub const MAX_OUTPUT_TOKENS: u32 = 6_000;
pub const BUILTIN_CATALOG_SIGNATURE: &str =
    "d/oXmTBDjPacQmQHXERk3bw5eko5mOPfkgpABylLVZCJJ2t62SBDkd+Jc9xJvsOyq/UnrXEUtxWmDTOuoA7hBw==";
pub const BUILTIN_CATALOG_PUBLIC_KEY: [u8; 32] = [
    0xfa, 0x6f, 0x6a, 0x11, 0xf7, 0x4d, 0x1a, 0x75, 0xad, 0x18, 0x3a, 0xc4, 0x21, 0x18, 0x2f, 0xed,
    0x97, 0x64, 0x99, 0xb8, 0xbd, 0xc1, 0x54, 0xa6, 0xd4, 0x19, 0xe6, 0xfc, 0x70, 0x9d, 0xe7, 0x65,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Economy,
    Balanced,
    Quality,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    TailorResume,
    RefineResume,
    CoverLetter,
    AnswerQuestion,
    ImportMapping,
    CredentialTest,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Usage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_write_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PriceCategory {
    Input,
    CachedInput,
    CacheWrite,
    Output,
    Reasoning,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Price {
    pub micros_per_million: u64,
    pub category: PriceCategory,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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

    /// Resolves an explicit model without assigning it to a cost tier.
    /// # Errors
    /// Rejects unknown, disabled, wrong-provider, or unsupported-operation models.
    pub fn resolve_model(
        &self,
        provider: Provider,
        model: &str,
        operation: OperationType,
    ) -> Result<&CatalogEntry, AiError> {
        self.entries
            .iter()
            .find(|entry| {
                entry.provider == provider
                    && entry.model == model
                    && entry.operations.contains(&operation)
                    && !entry.disabled
            })
            .ok_or(AiError::UnsupportedModel)
    }

    /// All enabled choices for this provider and operation, in catalog order.
    /// Legacy tier metadata never limits the number of model choices.
    #[must_use]
    pub fn available_models(
        &self,
        provider: Provider,
        operation: OperationType,
    ) -> Vec<&CatalogEntry> {
        let mut seen = std::collections::HashSet::new();
        self.entries
            .iter()
            .filter(|entry| {
                entry.provider == provider
                    && !entry.disabled
                    && entry.operations.contains(&operation)
                    && seen.insert(entry.model.as_str())
            })
            .collect()
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
    Failure(StreamFailure),
    Finished,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamFailure {
    OutputLimit,
    Blocked(String),
    ProviderStatus(u16),
    Stopped(String),
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
        let format = if matches!(
            r.operation,
            OperationType::TailorResume | OperationType::RefineResume
        ) {
            json!({
                "type": "json_schema",
                "name": "resume_draft",
                "strict": true,
                "schema": materials::resume_output_schema(),
            })
        } else {
            json!({"type": "json_object"})
        };
        Ok(HttpRequest { url: "https://api.openai.com/v1/responses".into(), headers: BTreeMap::from([("authorization".into(), format!("Bearer {}", key.text()?)), ("content-type".into(), "application/json".into())]), body: serde_json::to_vec(&json!({"model":r.model,"instructions":r.system,"input":encoded(&r.input)?,"max_output_tokens":r.max_output_tokens,"stream":true,"store":false,"tools":[],"tool_choice":"none","text":{"format":format}})).map_err(|_| AiError::InvalidResponse)? })
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
        let mut generation_config = json!({
            "maxOutputTokens": r.max_output_tokens,
            "responseMimeType": "application/json"
        });
        if matches!(
            r.operation,
            OperationType::TailorResume | OperationType::RefineResume
        ) {
            // Send the actual body-only contract, rather than unconstrained JSON.
            generation_config
                .as_object_mut()
                .expect("generation config")
                .remove("responseMimeType");
            generation_config["responseFormat"] = json!({"text":{
                // This REST field is a protobuf enum, unlike responseMimeType.
                "mimeType":"APPLICATION_JSON",
                "schema":materials::gemini_resume_output_schema()
            }});
        }
        if r.model.starts_with("gemini-3.")
            && matches!(
                r.operation,
                OperationType::CredentialTest
                    | OperationType::TailorResume
                    | OperationType::RefineResume
            )
        {
            generation_config["thinkingConfig"] = json!({"thinkingLevel":"LOW"});
        }
        Ok(HttpRequest { url: format!("https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse", r.model), headers: BTreeMap::from([("x-goog-api-key".into(), key.text()?.into()), ("content-type".into(), "application/json".into())]), body: serde_json::to_vec(&json!({"systemInstruction":{"parts":[{"text":r.system}]},"contents":[{"role":"user","parts":[{"text":encoded(&r.input)?}]}],"generationConfig":generation_config})).map_err(|_| AiError::InvalidResponse)? })
    }
    fn parse_stream(&self, bytes: &[u8]) -> Result<Vec<StreamEvent>, AiError> {
        let mut completed = false;
        let mut events = parse_sse(bytes, |v| {
            let mut events = Vec::new();
            if let Some(parts) = v
                .pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
            {
                for part in parts {
                    if part.get("thought").and_then(Value::as_bool) == Some(true) {
                        continue;
                    }
                    if let Some(text) = part.get("text").and_then(Value::as_str) {
                        events.push(StreamEvent::Text(text.into()));
                    }
                }
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
            if let Some(reason) = v.pointer("/candidates/0/finishReason") {
                match reason.as_str() {
                    Some("STOP") => {
                        completed = true;
                        events.push(StreamEvent::Finished);
                    }
                    Some("MAX_TOKENS") => {
                        events.push(StreamEvent::Failure(StreamFailure::OutputLimit));
                    }
                    Some("SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII") => {
                        events.push(StreamEvent::Failure(StreamFailure::Blocked(
                            reason.as_str().expect("matched string").into(),
                        )));
                    }
                    Some(reason) if safe_provider_reason(reason) => {
                        events.push(StreamEvent::Failure(StreamFailure::Stopped(reason.into())));
                    }
                    _ => events.push(StreamEvent::ProviderFailure),
                }
            }
            if let Some(error) = v.get("error") {
                if let Some(code) = error
                    .get("code")
                    .and_then(Value::as_u64)
                    .and_then(|code| u16::try_from(code).ok())
                    .filter(|code| (400..=599).contains(code))
                {
                    events.push(StreamEvent::Failure(StreamFailure::ProviderStatus(code)));
                } else {
                    events.push(StreamEvent::ProviderFailure);
                }
            }
            if let Some(reason) = v.pointer("/promptFeedback/blockReason") {
                let reason = reason
                    .as_str()
                    .filter(|r| safe_provider_reason(r))
                    .unwrap_or("UNKNOWN_BLOCK_REASON");
                events.push(StreamEvent::Failure(StreamFailure::Blocked(reason.into())));
            }
            if events.is_empty() {
                None
            } else {
                Some(Ok(events))
            }
        })?;
        // EOF or a generic SSE sentinel is not a successful candidate completion.
        if !completed {
            events.retain(|event| !matches!(event, StreamEvent::Finished));
        }
        Ok(events)
    }
}
fn safe_provider_reason(reason: &str) -> bool {
    !reason.is_empty()
        && reason.len() <= 64
        && reason.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
}

/// Extract a bounded machine code, never the provider's free-text message.
#[must_use]
pub fn provider_error_reason(bytes: &[u8]) -> Option<String> {
    if bytes.len() > 16 * 1024 {
        return None;
    }
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let detail = value
        .pointer("/error/details")
        .and_then(Value::as_array)
        .and_then(|details| {
            details
                .iter()
                .find_map(|d| d.get("reason").and_then(Value::as_str))
        });
    detail
        .or_else(|| value.pointer("/error/status").and_then(Value::as_str))
        .or_else(|| value.pointer("/error/code").and_then(Value::as_str))
        .or_else(|| value.pointer("/error/type").and_then(Value::as_str))
        .filter(|r| {
            !r.is_empty()
                && r.len() <= 64
                && r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
        .map(str::to_owned)
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
        // Proto JSON may omit a zero count when thinking consumed the entire
        // output allowance and no visible candidate was produced.
        output_tokens: optional(v, "/candidatesTokenCount"),
        reasoning_tokens: optional(v, "/thoughtsTokenCount"),
        cache_write_tokens: 0,
    })
}

/// Integer-micro cost calculation, rounded up per category.
/// # Errors
/// Missing applicable prices fail closed instead of becoming zero.
pub fn estimate_cost(entry: &CatalogEntry, usage: Usage) -> Result<u64, AiError> {
    estimate_priced_cost(entry.provider, &entry.prices, usage)
}
/// Prices billing evidence independently of content acceptance.
/// # Errors
/// Rejects invalid usage, missing categories, and arithmetic overflow.
pub fn estimate_priced_cost(
    provider: Provider,
    prices: &[Price],
    usage: Usage,
) -> Result<u64, AiError> {
    if provider == Provider::OpenAi && usage.reasoning_tokens > usage.output_tokens {
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
            if provider == Provider::OpenAi {
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
            let rate = prices
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

#[cfg(test)]
mod tests;
