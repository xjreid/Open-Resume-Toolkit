//! User-confirmed, provider-direct synthetic request. Provider content is
//! returned only to the initiating main webview; accounting stores no prompt
//! or response bytes.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ort_ai::{
    AnthropicAdapter, ApiKey, GeminiAdapter, HttpRequest, MAX_STREAM_BYTES, NormalizedRequest,
    OpenAiAdapter, OperationType, Preset, Provider, ProviderAdapter, StreamEvent, Usage,
    builtin_catalog, estimate_cost,
};
use ort_domain::CommandResponse;
use ort_storage::{
    StorageError,
    ai_activity::{AiAttemptPreflight, AiAttemptSettlement, AiTerminalStatus},
};
use ort_vault::{OsProviderCredentialVault, ProviderCredentialReference, ProviderCredentialVault};
use reqwest::{Client, Method, Url, redirect::Policy};
use serde::Serialize;
use serde_json::{Value, json};
use tauri::{Manager, State, WebviewWindow, ipc::Channel};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::{DesktopState, storage_unavailable, window_not_authorized};

const OUTPUT_LIMIT: u32 = 64;
const TEST_INPUT_ESTIMATE: u32 = 512;
const TEST_INPUT_RESERVATION_BOUND: u32 = 4_096;

#[derive(Default)]
pub(crate) struct AiRequestGate(Mutex<Option<(Uuid, Arc<CancelSignal>)>>);

#[derive(Default)]
struct CancelSignal {
    cancelled: AtomicBool,
    notify: Notify,
}
impl CancelSignal {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.notify.notify_one();
    }
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    async fn wait(&self) {
        let notified = self.notify.notified();
        if !self.is_cancelled() {
            notified.await;
        }
    }
}

pub(crate) struct AiRequestLease<'a> {
    gate: &'a AiRequestGate,
    id: Uuid,
    signal: Arc<CancelSignal>,
}
impl Drop for AiRequestLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut current) = self.gate.0.lock()
            && current.as_ref().is_some_and(|(id, _)| *id == self.id)
        {
            *current = None;
        }
    }
}
impl AiRequestGate {
    pub(crate) fn begin(&self, id: Uuid) -> Option<AiRequestLease<'_>> {
        let mut current = self.0.lock().ok()?;
        if current.is_some() {
            return None;
        }
        let signal = Arc::new(CancelSignal::default());
        *current = Some((id, Arc::clone(&signal)));
        Some(AiRequestLease {
            gate: self,
            id,
            signal,
        })
    }
    fn cancel(&self) -> bool {
        let Ok(current) = self.0.lock() else {
            return false;
        };
        let Some((_, signal)) = &*current else {
            return false;
        };
        signal.cancel();
        true
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTestResult {
    pub attempt_id: Uuid,
    pub effective_model: String,
    pub usage: Usage,
    pub estimated_cost_micros: Option<u64>,
    pub usage_complete: bool,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTestPreview {
    pub credential_id: Uuid,
    pub provider: String,
    pub model: String,
    pub currency: String,
    pub estimated_input_tokens: u32,
    pub maximum_cost_micros: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProgress {
    pub kind: &'static str,
    pub text: String,
}

fn now_unix_ms() -> Option<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

fn provider(value: &str) -> Option<Provider> {
    match value {
        "openai" => Some(Provider::OpenAi),
        "anthropic" => Some(Provider::Anthropic),
        "gemini" => Some(Provider::Gemini),
        _ => None,
    }
}
fn preset(value: &str) -> Option<Preset> {
    match value {
        "economy" => Some(Preset::Economy),
        "balanced" => Some(Preset::Balanced),
        "quality" => Some(Preset::Quality),
        _ => None,
    }
}
fn adapter(provider: Provider) -> Box<dyn ProviderAdapter + Send + Sync> {
    match provider {
        Provider::OpenAi => Box::new(OpenAiAdapter),
        Provider::Anthropic => Box::new(AnthropicAdapter),
        Provider::Gemini => Box::new(GeminiAdapter),
    }
}

fn pinned_url(provider: Provider, requested_model: &str, request: &HttpRequest) -> Option<Url> {
    let url = Url::parse(&request.url).ok()?;
    if url.scheme() != "https"
        || url.port().is_some()
        || url.username() != ""
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let expected = match provider {
        Provider::OpenAi => "https://api.openai.com/v1/responses".to_owned(),
        Provider::Anthropic => "https://api.anthropic.com/v1/messages".to_owned(),
        Provider::Gemini => format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{requested_model}:streamGenerateContent?alt=sse"
        ),
    };
    (url.as_str() == expected).then_some(url)
}

fn http_client() -> Result<Client, ()> {
    Client::builder()
        .https_only(true)
        .redirect(Policy::none())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| ())
}

fn outbound(
    client: &Client,
    url: Url,
    request: &HttpRequest,
) -> Result<reqwest::RequestBuilder, ()> {
    let mut builder = client.request(Method::POST, url);
    for (name, value) in &request.headers {
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| ())?;
        let mut value = reqwest::header::HeaderValue::from_str(value).map_err(|_| ())?;
        if matches!(
            name.as_str(),
            "authorization" | "x-api-key" | "x-goog-api-key"
        ) {
            value.set_sensitive(true);
        }
        builder = builder.header(name, value);
    }
    Ok(builder.body(request.body.clone()))
}

fn category(status: reqwest::StatusCode) -> &'static str {
    match status.as_u16() {
        401 | 403 => "authentication",
        429 => "rate_limit",
        500..=599 => "transient",
        _ => "provider",
    }
}

fn maximum_cost(entry: &ort_ai::CatalogEntry, provider: Provider) -> Option<u64> {
    if entry.max_input_tokens < TEST_INPUT_RESERVATION_BOUND {
        return None;
    }
    let input = u64::from(TEST_INPUT_RESERVATION_BOUND);
    let output = u64::from(OUTPUT_LIMIT);
    let mut usage = Usage::default();
    for price in &entry.prices {
        match price.category {
            ort_ai::PriceCategory::Input => usage.input_tokens = input,
            ort_ai::PriceCategory::CachedInput => usage.cached_input_tokens = input,
            ort_ai::PriceCategory::CacheWrite => usage.cache_write_tokens = input,
            ort_ai::PriceCategory::Output => usage.output_tokens = output,
            ort_ai::PriceCategory::Reasoning => usage.reasoning_tokens = output,
        }
    }
    if provider == Provider::Gemini && usage.reasoning_tokens == 0 {
        return None;
    }
    estimate_cost(entry, usage).ok()
}

#[derive(Default)]
struct SyntheticStreamState {
    text: String,
    usage: Option<Usage>,
    effective_model: Option<String>,
    failed: bool,
    finished: bool,
}
impl SyntheticStreamState {
    fn from_events(events: Vec<StreamEvent>) -> Self {
        let mut state = Self::default();
        for event in events {
            match event {
                StreamEvent::Text(delta) => state.text.push_str(&delta),
                StreamEvent::Usage(value) => state.usage = Some(value),
                StreamEvent::Model(model) => state.effective_model = Some(model),
                StreamEvent::ProviderFailure => state.failed = true,
                StreamEvent::Finished => state.finished = true,
            }
        }
        state
    }
    fn valid(&self, provider: Provider, requested_model: &str) -> bool {
        let json_ok = serde_json::from_str::<Value>(&self.text)
            .ok()
            .and_then(|value| value.get("ok").and_then(Value::as_bool))
            .is_some_and(|ok| ok);
        !self.failed
            && self.effective_model.as_deref() == Some(requested_model)
            && json_ok
            && (self.finished || provider == Provider::Gemini)
    }
}

#[allow(clippy::needless_pass_by_value)]
fn settle(state: &DesktopState, result: AiAttemptSettlement) -> bool {
    state
        .with_store(|store| store.settle_ai_attempt(&result))
        .is_ok()
}

fn fail_after_settlement(
    state: &DesktopState,
    result: AiAttemptSettlement,
    code: &'static str,
    message_key: &'static str,
    retryable: bool,
) -> CommandResponse<AiTestResult> {
    if settle(state, result) {
        CommandResponse::failure(code, message_key, retryable)
    } else {
        storage_unavailable()
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn preview_ai_test(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    credential_id: Uuid,
) -> CommandResponse<AiTestPreview> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Ok(connection) =
        state.with_store(|store| crate::ai_keys::test_connection(store, credential_id))
    else {
        return CommandResponse::failure(
            "AI_CONFIGURATION_INVALID",
            "errors.aiConfigurationInvalid",
            false,
        );
    };
    if connection.mode != "direct_api" {
        return CommandResponse::failure("AI_DISABLED", "errors.aiDisabled", false);
    }
    let Some(provider) = connection.provider.as_deref().and_then(provider) else {
        return CommandResponse::failure(
            "AI_CONFIGURATION_INVALID",
            "errors.aiConfigurationInvalid",
            false,
        );
    };
    let Some(preset) = connection.preset.as_deref().and_then(preset) else {
        return CommandResponse::failure(
            "AI_CONFIGURATION_INVALID",
            "errors.aiConfigurationInvalid",
            false,
        );
    };
    let Ok(catalog) = builtin_catalog(&jiff::Timestamp::now().to_string(), None) else {
        return CommandResponse::failure(
            "AI_CATALOG_UNAVAILABLE",
            "errors.aiCatalogUnavailable",
            false,
        );
    };
    let Ok(entry) = catalog.resolve(provider, preset, OperationType::CredentialTest) else {
        return CommandResponse::failure(
            "AI_PRESET_UNAVAILABLE",
            "errors.aiPresetUnavailable",
            false,
        );
    };
    let Some(maximum_cost_micros) = maximum_cost(entry, provider) else {
        return CommandResponse::failure(
            "AI_PRICE_UNAVAILABLE",
            "errors.aiPriceUnavailable",
            false,
        );
    };
    CommandResponse::success(AiTestPreview {
        credential_id,
        provider: provider.as_str().into(),
        model: entry.model.clone(),
        currency: entry.currency.clone(),
        estimated_input_tokens: TEST_INPUT_ESTIMATE,
        maximum_cost_micros,
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cancel_ai_test(
    window: WebviewWindow,
    gate: State<'_, AiRequestGate>,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    CommandResponse::success(gate.cancel())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
pub async fn test_ai_connection(
    window: WebviewWindow,
    on_progress: Channel<AiProgress>,
    credential_id: Uuid,
    expected_model: String,
    expected_maximum_cost_micros: u64,
) -> CommandResponse<AiTestResult> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let app = window.app_handle().clone();
    let state = app.state::<DesktopState>();
    let gate = app.state::<AiRequestGate>();
    let mut attempt_id = Uuid::now_v7();
    let operation_id = Uuid::now_v7();
    let Some(lease) = gate.begin(operation_id) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    let prepared = state.with_store(|store| {
        let connection = crate::ai_keys::test_connection(store, credential_id)?;
        let (channel, install, profile) = store.vault_identity();
        Ok((connection, channel.to_owned(), install, profile))
    });
    let Ok((connection, channel, install, profile)) = prepared else {
        return storage_unavailable();
    };
    if connection.mode != "direct_api" {
        return CommandResponse::failure("AI_DISABLED", "errors.aiDisabled", false);
    }
    let Some(provider) = connection.provider.as_deref().and_then(provider) else {
        return CommandResponse::failure(
            "AI_CONFIGURATION_INVALID",
            "errors.aiConfigurationInvalid",
            false,
        );
    };
    let Some(preset) = connection.preset.as_deref().and_then(preset) else {
        return CommandResponse::failure(
            "AI_CONFIGURATION_INVALID",
            "errors.aiConfigurationInvalid",
            false,
        );
    };
    let Some(credential_id) = connection.credential_id else {
        return CommandResponse::failure(
            "AI_CREDENTIAL_MISSING",
            "errors.aiCredentialMissing",
            false,
        );
    };
    let now = jiff::Timestamp::now().to_string();
    let Ok(catalog) = builtin_catalog(&now, None) else {
        return CommandResponse::failure(
            "AI_CATALOG_UNAVAILABLE",
            "errors.aiCatalogUnavailable",
            false,
        );
    };
    let Ok(entry) = catalog.resolve(provider, preset, OperationType::CredentialTest) else {
        return CommandResponse::failure(
            "AI_PRESET_UNAVAILABLE",
            "errors.aiPresetUnavailable",
            false,
        );
    };
    let entry = entry.clone();
    let request = NormalizedRequest {
        operation: OperationType::CredentialTest,
        model: entry.model.clone(),
        system: "Return only a JSON object with the boolean field ok set to true. No tools, files, user content, or additional text.".into(),
        input: json!({"test":"Open Resume Toolkit synthetic connection test"}),
        max_output_tokens: OUTPUT_LIMIT,
    };
    let Ok(reference) = ProviderCredentialReference::new(
        &channel,
        &install.to_string(),
        &profile.to_string(),
        provider.as_str(),
        &credential_id.to_string(),
    ) else {
        return CommandResponse::failure(
            "AI_CREDENTIAL_MISSING",
            "errors.aiCredentialMissing",
            false,
        );
    };
    let provider_adapter = adapter(provider);
    let built = OsProviderCredentialVault::new().use_secret(&reference, |secret| {
        secret.expose_for(|bytes| {
            let key = ApiKey::new(bytes)?;
            provider_adapter.build_request(&request, &key)
        })
    });
    let Ok(Ok(http_request)) = built else {
        return CommandResponse::failure(
            "AI_CREDENTIAL_MISSING",
            "errors.aiCredentialMissing",
            false,
        );
    };
    let Some(url) = pinned_url(provider, &entry.model, &http_request) else {
        return CommandResponse::failure("AI_PROVIDER_INVALID", "errors.aiProviderInvalid", false);
    };
    let Some(maximum_cost_micros) = maximum_cost(&entry, provider) else {
        return CommandResponse::failure(
            "AI_PRICE_UNAVAILABLE",
            "errors.aiPriceUnavailable",
            false,
        );
    };
    if entry.model != expected_model || maximum_cost_micros != expected_maximum_cost_micros {
        return CommandResponse::failure(
            "AI_CONFIRMATION_STALE",
            "errors.aiConfirmationStale",
            false,
        );
    }
    let Some(mut started) = now_unix_ms() else {
        return storage_unavailable();
    };
    let preflight = AiAttemptPreflight {
        operation_id,
        attempt_id,
        operation_type: OperationType::CredentialTest,
        provider,
        credential_id,
        requested_model: entry.model.clone(),
        preset_version: format!(
            "{}@direct-v1",
            match preset {
                Preset::Economy => "economy",
                Preset::Balanced => "balanced",
                Preset::Quality => "quality",
            }
        ),
        catalog_id: catalog.catalog_id,
        catalog_effective_from: entry.effective_from.clone(),
        pricing_components: entry.prices.clone(),
        started_at_unix_ms: started,
        estimated_input_tokens: u64::from(TEST_INPUT_ESTIMATE),
        maximum_cost_micros,
        currency: entry.currency.clone(),
        retry_of: None,
    };
    if let Err(error) = state.with_store(|store| store.reserve_ai_attempt(&preflight)) {
        return match error {
            StorageError::RevisionConflict => {
                CommandResponse::failure("AI_BUSY", "errors.aiBusy", true)
            }
            StorageError::InvalidData => {
                CommandResponse::failure("AI_CAP_REJECTED", "errors.aiCapRejected", false)
            }
            _ => storage_unavailable(),
        };
    }
    if lease.signal.is_cancelled() {
        if state
            .with_store(|store| {
                store.cancel_reserved_ai_attempt(attempt_id, now_unix_ms().unwrap_or(started))
            })
            .is_err()
        {
            return storage_unavailable();
        }
        return CommandResponse::failure("AI_CANCELLED", "errors.aiCancelled", false);
    }
    let Ok(client) = http_client() else {
        if state
            .with_store(|store| {
                store.cancel_reserved_ai_attempt(attempt_id, now_unix_ms().unwrap_or(started))
            })
            .is_err()
        {
            return storage_unavailable();
        }
        return CommandResponse::failure(
            "AI_PROVIDER_UNAVAILABLE",
            "errors.aiProviderUnavailable",
            true,
        );
    };
    let Ok(outbound) = outbound(&client, url, &http_request) else {
        if state
            .with_store(|store| {
                store.cancel_reserved_ai_attempt(attempt_id, now_unix_ms().unwrap_or(started))
            })
            .is_err()
        {
            return storage_unavailable();
        }
        return CommandResponse::failure("AI_PROVIDER_INVALID", "errors.aiProviderInvalid", false);
    };
    drop(http_request);
    let retry_outbound = outbound.try_clone();
    if state
        .with_store(|store| store.mark_ai_dispatching(attempt_id))
        .is_err()
    {
        let _ = state.with_store(|store| {
            store.cancel_reserved_ai_attempt(attempt_id, now_unix_ms().unwrap_or(started))
        });
        return storage_unavailable();
    }
    let response = tokio::select! {
        response = outbound.send() => response,
        () = lease.signal.wait() => {
            let ended = now_unix_ms().unwrap_or(started);
            return fail_after_settlement(&state, AiAttemptSettlement { attempt_id, status: AiTerminalStatus::Cancelled,
                effective_model: None, usage: None, settled_cost_micros: None, usage_complete: false,
                error_category: Some("cancelled".into()), ended_at_unix_ms: ended,
                keep_operation_active: false },
                "AI_CANCELLED", "errors.aiCancelled", false);
        }
    };
    let Ok(mut response) = response else {
        let ended = now_unix_ms().unwrap_or(started);
        return fail_after_settlement(
            &state,
            AiAttemptSettlement {
                attempt_id,
                status: AiTerminalStatus::OutcomeUnknown,
                effective_model: None,
                usage: None,
                settled_cost_micros: None,
                usage_complete: false,
                error_category: Some("transient".into()),
                ended_at_unix_ms: ended,
                keep_operation_active: false,
            },
            "AI_PROVIDER_UNAVAILABLE",
            "errors.aiProviderUnavailable",
            true,
        );
    };
    if response.status().is_server_error()
        && let Some(retry_outbound) = retry_outbound
    {
        let ended = now_unix_ms().unwrap_or(started);
        if !settle(
            &state,
            AiAttemptSettlement {
                attempt_id,
                status: AiTerminalStatus::Failed,
                effective_model: None,
                usage: None,
                settled_cost_micros: None,
                usage_complete: false,
                error_category: Some("transient".into()),
                ended_at_unix_ms: ended,
                keep_operation_active: true,
            },
        ) {
            return storage_unavailable();
        }
        if lease.signal.is_cancelled() {
            if state
                .with_store(|store| {
                    store.finish_ai_operation(operation_id, AiTerminalStatus::Cancelled, ended)
                })
                .is_err()
            {
                return storage_unavailable();
            }
            return CommandResponse::failure("AI_CANCELLED", "errors.aiCancelled", false);
        }
        let previous_attempt = attempt_id;
        attempt_id = Uuid::now_v7();
        started = now_unix_ms().unwrap_or(ended);
        let mut retry_preflight = preflight.clone();
        retry_preflight.attempt_id = attempt_id;
        retry_preflight.started_at_unix_ms = started;
        retry_preflight.retry_of = Some(previous_attempt);
        if state
            .with_store(|store| store.reserve_ai_attempt(&retry_preflight))
            .is_err()
        {
            if state
                .with_store(|store| {
                    store.finish_ai_operation(operation_id, AiTerminalStatus::Failed, started)
                })
                .is_err()
            {
                return storage_unavailable();
            }
            return CommandResponse::failure("AI_RETRY_BLOCKED", "errors.aiRetryBlocked", false);
        }
        if lease.signal.is_cancelled() {
            if state
                .with_store(|store| {
                    store.cancel_reserved_ai_attempt(attempt_id, now_unix_ms().unwrap_or(started))
                })
                .is_err()
            {
                return storage_unavailable();
            }
            return CommandResponse::failure("AI_CANCELLED", "errors.aiCancelled", false);
        }
        if state
            .with_store(|store| store.mark_ai_dispatching(attempt_id))
            .is_err()
        {
            let _ = state.with_store(|store| {
                store.cancel_reserved_ai_attempt(attempt_id, now_unix_ms().unwrap_or(started))
            });
            return storage_unavailable();
        }
        let _ = on_progress.send(AiProgress {
            kind: "retry",
            text: String::new(),
        });
        let second = tokio::select! {
            response = retry_outbound.send() => response,
            () = lease.signal.wait() => {
                let ended = now_unix_ms().unwrap_or(started);
                return fail_after_settlement(&state, AiAttemptSettlement {
                    attempt_id, status: AiTerminalStatus::Cancelled, effective_model: None,
                    usage: None, settled_cost_micros: None, usage_complete: false,
                    error_category: Some("cancelled".into()), ended_at_unix_ms: ended,
                    keep_operation_active: false,
                }, "AI_CANCELLED", "errors.aiCancelled", false);
            }
        };
        let Ok(second) = second else {
            let ended = now_unix_ms().unwrap_or(started);
            return fail_after_settlement(
                &state,
                AiAttemptSettlement {
                    attempt_id,
                    status: AiTerminalStatus::OutcomeUnknown,
                    effective_model: None,
                    usage: None,
                    settled_cost_micros: None,
                    usage_complete: false,
                    error_category: Some("transient".into()),
                    ended_at_unix_ms: ended,
                    keep_operation_active: false,
                },
                "AI_PROVIDER_UNAVAILABLE",
                "errors.aiProviderUnavailable",
                true,
            );
        };
        response = second;
    }
    if !response.status().is_success() {
        let error = category(response.status());
        let ended = now_unix_ms().unwrap_or(started);
        return fail_after_settlement(
            &state,
            AiAttemptSettlement {
                attempt_id,
                status: AiTerminalStatus::Failed,
                effective_model: None,
                usage: None,
                settled_cost_micros: None,
                usage_complete: false,
                error_category: Some(error.into()),
                ended_at_unix_ms: ended,
                keep_operation_active: false,
            },
            match error {
                "authentication" => "AI_AUTHENTICATION_FAILED",
                "rate_limit" => "AI_RATE_LIMITED",
                _ => "AI_PROVIDER_FAILED",
            },
            "errors.aiProviderFailed",
            true,
        );
    }
    if state
        .with_store(|store| store.mark_ai_streaming(attempt_id))
        .is_err()
    {
        let ended = now_unix_ms().unwrap_or(started);
        return fail_after_settlement(
            &state,
            AiAttemptSettlement {
                attempt_id,
                status: AiTerminalStatus::OutcomeUnknown,
                effective_model: None,
                usage: None,
                settled_cost_micros: None,
                usage_complete: false,
                error_category: Some("provider".into()),
                ended_at_unix_ms: ended,
                keep_operation_active: false,
            },
            "AI_STORAGE_UNAVAILABLE",
            "errors.storageUnavailable",
            true,
        );
    }
    let mut raw = Vec::new();
    let mut scan = 0_usize;
    loop {
        let next = tokio::select! { next = response.chunk() => next,
            () = lease.signal.wait() => {
                let ended = now_unix_ms().unwrap_or(started);
                return fail_after_settlement(&state, AiAttemptSettlement { attempt_id, status: AiTerminalStatus::Cancelled,
                    effective_model: None, usage: None, settled_cost_micros: None, usage_complete: false,
                    error_category: Some("cancelled".into()), ended_at_unix_ms: ended,
                    keep_operation_active: false },
                    "AI_CANCELLED", "errors.aiCancelled", false);
            }
        };
        let chunk = match next {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(_) => {
                let ended = now_unix_ms().unwrap_or(started);
                return fail_after_settlement(
                    &state,
                    AiAttemptSettlement {
                        attempt_id,
                        status: AiTerminalStatus::OutcomeUnknown,
                        effective_model: None,
                        usage: None,
                        settled_cost_micros: None,
                        usage_complete: false,
                        error_category: Some("transient".into()),
                        ended_at_unix_ms: ended,
                        keep_operation_active: false,
                    },
                    "AI_PROVIDER_UNAVAILABLE",
                    "errors.aiProviderUnavailable",
                    true,
                );
            }
        };
        if raw
            .len()
            .checked_add(chunk.len())
            .is_none_or(|size| size > MAX_STREAM_BYTES)
        {
            let ended = now_unix_ms().unwrap_or(started);
            return fail_after_settlement(
                &state,
                AiAttemptSettlement {
                    attempt_id,
                    status: AiTerminalStatus::Failed,
                    effective_model: None,
                    usage: None,
                    settled_cost_micros: None,
                    usage_complete: false,
                    error_category: Some("invalid_output".into()),
                    ended_at_unix_ms: ended,
                    keep_operation_active: false,
                },
                "AI_OUTPUT_INVALID",
                "errors.aiOutputInvalid",
                false,
            );
        }
        raw.extend_from_slice(&chunk);
        while let Some(relative) = raw[scan..].iter().position(|byte| *byte == b'\n') {
            let end = scan + relative + 1;
            if let Ok(events) = provider_adapter.parse_stream(&raw[scan..end]) {
                for event in events {
                    if let StreamEvent::Text(text) = event {
                        let _ = on_progress.send(AiProgress {
                            kind: "delta",
                            text,
                        });
                    }
                }
            }
            scan = end;
        }
    }
    let parsed = provider_adapter.parse_stream(&raw);
    let ended = now_unix_ms().unwrap_or(started);
    let Ok(events) = parsed else {
        return fail_after_settlement(
            &state,
            AiAttemptSettlement {
                attempt_id,
                status: AiTerminalStatus::Failed,
                effective_model: None,
                usage: None,
                settled_cost_micros: None,
                usage_complete: false,
                error_category: Some("invalid_output".into()),
                ended_at_unix_ms: ended,
                keep_operation_active: false,
            },
            "AI_OUTPUT_INVALID",
            "errors.aiOutputInvalid",
            false,
        );
    };
    let synthetic = SyntheticStreamState::from_events(events);
    let valid = synthetic.valid(provider, &entry.model);
    let usage = synthetic.usage;
    let effective_model = synthetic.effective_model;
    if !valid {
        return fail_after_settlement(
            &state,
            AiAttemptSettlement {
                attempt_id,
                status: AiTerminalStatus::Failed,
                effective_model,
                usage,
                settled_cost_micros: None,
                usage_complete: false,
                error_category: Some("invalid_output".into()),
                ended_at_unix_ms: ended,
                keep_operation_active: false,
            },
            "AI_OUTPUT_INVALID",
            "errors.aiOutputInvalid",
            false,
        );
    }
    let Some(usage) = usage else {
        return fail_after_settlement(
            &state,
            AiAttemptSettlement {
                attempt_id,
                status: AiTerminalStatus::OutcomeUnknown,
                effective_model,
                usage: None,
                settled_cost_micros: None,
                usage_complete: false,
                error_category: Some("provider".into()),
                ended_at_unix_ms: ended,
                keep_operation_active: false,
            },
            "AI_USAGE_UNKNOWN",
            "errors.aiUsageUnknown",
            true,
        );
    };
    // Never release a guardrail reservation into a larger counted amount.
    // Keep unexpected provider usage unresolved for reconciliation instead.
    let cost = estimate_cost(&entry, usage)
        .ok()
        .filter(|amount| *amount <= maximum_cost_micros);
    let model = effective_model.unwrap_or(entry.model);
    let saved = settle(
        &state,
        AiAttemptSettlement {
            attempt_id,
            status: AiTerminalStatus::Succeeded,
            effective_model: Some(model.clone()),
            usage: Some(usage),
            settled_cost_micros: cost,
            usage_complete: cost.is_some(),
            error_category: None,
            ended_at_unix_ms: ended,
            keep_operation_active: false,
        },
    );
    if !saved {
        return storage_unavailable();
    }
    let _ = on_progress.send(AiProgress {
        kind: "finished",
        text: String::new(),
    });
    CommandResponse::success(AiTestResult {
        attempt_id,
        effective_model: model,
        usage,
        estimated_cost_micros: cost,
        usage_complete: cost.is_some(),
        confirmed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_gate_can_cancel_and_reuse_without_cross_request_signal() {
        let gate = AiRequestGate::default();
        let first = gate.begin(Uuid::now_v7()).unwrap();
        assert!(gate.begin(Uuid::now_v7()).is_none());
        assert!(gate.cancel());
        assert!(first.signal.is_cancelled());
        drop(first);
        let second = gate.begin(Uuid::now_v7()).unwrap();
        assert!(!second.signal.is_cancelled());
    }

    #[test]
    fn provider_url_and_secret_headers_are_pinned_and_redacted() {
        let key = ApiKey::new(b"SYNTHETIC-TEST-SECRET").unwrap();
        let adapter = OpenAiAdapter;
        let request = NormalizedRequest {
            operation: OperationType::CredentialTest,
            model: "fixture-model".into(),
            system: "Return JSON".into(),
            input: json!({"fixture":true}),
            max_output_tokens: 10,
        };
        let mut built = adapter.build_request(&request, &key).unwrap();
        assert!(pinned_url(Provider::OpenAi, "fixture-model", &built).is_some());
        let client = http_client().unwrap();
        let outbound = outbound(
            &client,
            pinned_url(Provider::OpenAi, "fixture-model", &built).unwrap(),
            &built,
        )
        .unwrap()
        .build()
        .unwrap();
        assert!(
            outbound
                .headers()
                .get("authorization")
                .unwrap()
                .is_sensitive()
        );
        assert!(!format!("{outbound:?}").contains("SYNTHETIC-TEST-SECRET"));
        built.url = "https://api.openai.com.attacker.invalid/v1/responses".into();
        assert!(pinned_url(Provider::OpenAi, "fixture-model", &built).is_none());
        built.url = "https://api.openai.com/v1/responses?redirect=https://attacker.invalid".into();
        assert!(pinned_url(Provider::OpenAi, "fixture-model", &built).is_none());
    }

    #[test]
    fn bundled_balanced_test_has_a_nonzero_conservative_reservation() {
        let catalog = builtin_catalog("2026-09-15T00:00:00Z", None).unwrap();
        for provider in [Provider::OpenAi, Provider::Anthropic, Provider::Gemini] {
            let entry = catalog
                .resolve(provider, Preset::Balanced, OperationType::CredentialTest)
                .unwrap();
            let reserved = maximum_cost(entry, provider).unwrap();
            assert!(reserved > 0);
            for price in &entry.prices {
                let mut usage = Usage::default();
                match price.category {
                    ort_ai::PriceCategory::Input => {
                        usage.input_tokens = u64::from(TEST_INPUT_RESERVATION_BOUND);
                    }
                    ort_ai::PriceCategory::CachedInput => {
                        usage.cached_input_tokens = u64::from(TEST_INPUT_RESERVATION_BOUND);
                    }
                    ort_ai::PriceCategory::CacheWrite => {
                        usage.cache_write_tokens = u64::from(TEST_INPUT_RESERVATION_BOUND);
                    }
                    ort_ai::PriceCategory::Output => usage.output_tokens = u64::from(OUTPUT_LIMIT),
                    ort_ai::PriceCategory::Reasoning => {
                        usage.reasoning_tokens = u64::from(OUTPUT_LIMIT);
                    }
                }
                assert!(reserved >= estimate_cost(entry, usage).unwrap());
            }
        }
    }

    #[test]
    fn synthetic_stream_requires_exact_serving_model_json_and_terminal_event() {
        let raw = br#"data: {"type":"response.created","response":{"model":"fixture-model"}}
data: {"type":"response.output_text.delta","delta":"{\"ok\":true}"}
data: {"type":"response.completed","response":{"usage":{"input_tokens":10,"output_tokens":3}}}
"#;
        let parsed = SyntheticStreamState::from_events(OpenAiAdapter.parse_stream(raw).unwrap());
        assert!(parsed.valid(Provider::OpenAi, "fixture-model"));
        assert_eq!(parsed.usage.unwrap().output_tokens, 3);
        assert!(!parsed.valid(Provider::OpenAi, "other-model"));
        let incomplete = SyntheticStreamState::from_events(vec![
            StreamEvent::Model("fixture-model".into()),
            StreamEvent::Text("{\"ok\":true}".into()),
        ]);
        assert!(!incomplete.valid(Provider::OpenAi, "fixture-model"));
        let failed = SyntheticStreamState::from_events(vec![
            StreamEvent::Model("fixture-model".into()),
            StreamEvent::Text("{\"ok\":true}".into()),
            StreamEvent::ProviderFailure,
            StreamEvent::Finished,
        ]);
        assert!(!failed.valid(Provider::OpenAi, "fixture-model"));
    }
}
