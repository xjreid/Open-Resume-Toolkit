//! User-confirmed, provider-direct synthetic request. Provider content is
//! returned only to the initiating main webview; accounting stores no prompt
//! or response bytes.

#[path = "ai_execution.rs"]
mod execution;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ort_ai::{
    AnthropicAdapter, ApiKey, GeminiAdapter, HttpRequest, NormalizedRequest, OpenAiAdapter,
    OperationType, Provider, ProviderAdapter, StreamEvent, Usage, builtin_catalog, estimate_cost,
};
use ort_domain::CommandResponse;
use ort_storage::ai_activity::AiAttemptPreflight;
use ort_vault::{OsProviderCredentialVault, ProviderCredentialReference, ProviderCredentialVault};
use reqwest::{Client, Method, Url, redirect::Policy};
use serde::Serialize;
use serde_json::{Value, json};
use tauri::{Manager, State, WebviewWindow, ipc::Channel};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::{DesktopState, storage_unavailable, window_not_authorized};

const OUTPUT_LIMIT: u32 = 64;
const GEMINI_TEST_OUTPUT_LIMIT: u32 = 512;
const TEST_INPUT_ESTIMATE: u32 = 512;
const TEST_INPUT_RESERVATION_BOUND: u32 = 4_096;

#[derive(Default)]
pub(crate) struct AiRequestGate(Mutex<Option<(Uuid, bool, Arc<CancelSignal>)>>);

#[derive(Default)]
pub(crate) struct CancelSignal {
    cancelled: AtomicBool,
    notify: Notify,
}
impl CancelSignal {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.notify.notify_one();
    }
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    pub(crate) async fn wait(&self) {
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
impl AiRequestLease<'_> {
    pub(crate) fn cancellation_signal(&self) -> Arc<CancelSignal> {
        Arc::clone(&self.signal)
    }
}
impl Drop for AiRequestLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut current) = self.gate.0.lock()
            && current.as_ref().is_some_and(|(id, _, _)| *id == self.id)
        {
            *current = None;
        }
    }
}
impl AiRequestGate {
    pub(crate) fn is_busy(&self) -> bool {
        self.0.lock().is_ok_and(|current| current.is_some())
    }

    pub(crate) fn begin(&self, id: Uuid) -> Option<AiRequestLease<'_>> {
        self.begin_owned(id, false)
    }
    pub(crate) fn begin_overlay(&self, id: Uuid) -> Option<AiRequestLease<'_>> {
        self.begin_owned(id, true)
    }
    fn begin_owned(&self, id: Uuid, overlay: bool) -> Option<AiRequestLease<'_>> {
        let mut current = self.0.lock().ok()?;
        if current.is_some() {
            return None;
        }
        let signal = Arc::new(CancelSignal::default());
        *current = Some((id, overlay, Arc::clone(&signal)));
        Some(AiRequestLease {
            gate: self,
            id,
            signal,
        })
    }
    pub(crate) fn cancel(&self) -> bool {
        self.cancel_owned(false)
    }
    pub(crate) fn cancel_overlay(&self) -> bool {
        self.cancel_owned(true)
    }
    fn cancel_owned(&self, overlay: bool) -> bool {
        let Ok(current) = self.0.lock() else {
            return false;
        };
        let Some((_, current_overlay, signal)) = &*current else {
            return false;
        };
        if *current_overlay != overlay {
            return false;
        }
        signal.cancel();
        true
    }
}

#[derive(Clone, Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiTestResult {
    pub attempt_id: Uuid,
    pub effective_model: String,
    pub usage: Usage,
    pub estimated_cost_micros: Option<u64>,
    pub usage_complete: bool,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AiTestPreview {
    pub credential_id: Uuid,
    pub provider: String,
    pub model: String,
    pub currency: String,
    pub estimated_input_tokens: u32,
    pub maximum_cost_micros: u64,
}

#[derive(Clone, Debug, Serialize, schemars::JsonSchema)]
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

fn http_client_with_timeout(timeout: Duration) -> Result<Client, ()> {
    Client::builder()
        .https_only(true)
        .redirect(Policy::none())
        .timeout(timeout)
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
        504 => "timeout",
        500..=599 => "transient",
        _ => "provider",
    }
}

fn provider_failure(status: reqwest::StatusCode) -> (&'static str, bool) {
    match status.as_u16() {
        400 => ("AI_PROVIDER_BAD_REQUEST", false),
        401 | 403 => ("AI_AUTHENTICATION_FAILED", false),
        404 => ("AI_MODEL_UNAVAILABLE", false),
        429 => ("AI_RATE_LIMITED", true),
        503 => ("AI_PROVIDER_SERVICE_UNAVAILABLE", true),
        504 => ("AI_PROVIDER_TIMEOUT", true),
        500..=599 => ("AI_PROVIDER_TEMPORARY", true),
        _ => ("AI_PROVIDER_FAILED", false),
    }
}

fn maximum_cost(entry: &ort_ai::CatalogEntry, provider: Provider) -> Option<u64> {
    if entry.max_input_tokens < TEST_INPUT_RESERVATION_BOUND {
        return None;
    }
    let input = u64::from(TEST_INPUT_RESERVATION_BOUND);
    let output = u64::from(test_output_limit(provider));
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

const fn test_output_limit(provider: Provider) -> u32 {
    if matches!(provider, Provider::Gemini) {
        GEMINI_TEST_OUTPUT_LIMIT
    } else {
        OUTPUT_LIMIT
    }
}

#[derive(Default)]
struct SyntheticStreamState {
    text: String,
    usage: Option<Usage>,
    effective_model: Option<String>,
    failed: bool,
    failure: Option<ort_ai::StreamFailure>,
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
                StreamEvent::Failure(failure) => state.failure = Some(failure),
                StreamEvent::Finished => state.finished = true,
            }
        }
        state
    }
    fn material_error(&self, requested_model: &str) -> Option<&'static str> {
        if let Some(failure) = &self.failure {
            Some(match failure {
                ort_ai::StreamFailure::OutputLimit => "AI_OUTPUT_LIMIT",
                ort_ai::StreamFailure::Blocked(_) => "AI_OUTPUT_BLOCKED",
                ort_ai::StreamFailure::Stopped(_) => "AI_OUTPUT_INVALID",
                ort_ai::StreamFailure::ProviderStatus(code) => reqwest::StatusCode::from_u16(*code)
                    .map_or("AI_PROVIDER_FAILED", |status| provider_failure(status).0),
            })
        } else if self.failed || self.text.len() > 512 * 1024 {
            Some("AI_OUTPUT_INVALID")
        } else if !self.finished {
            Some("AI_OUTPUT_INCOMPLETE")
        } else if self.effective_model.as_deref() != Some(requested_model) {
            Some("AI_MODEL_MISMATCH")
        } else {
            None
        }
    }
    fn valid(&self, requested_model: &str) -> bool {
        let json_ok = serde_json::from_str::<Value>(&self.text)
            .ok()
            .and_then(|value| value.get("ok").and_then(Value::as_bool))
            .is_some_and(|ok| ok);
        !self.failed
            && self.failure.is_none()
            && self.effective_model.as_deref() == Some(requested_model)
            && json_ok
            && self.finished
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
    if window
        .state::<DesktopState>()
        .with_store(|s| crate::chatgpt_plan::load_settings(s).map(|v| v.0.enabled))
        .unwrap_or(true)
    {
        return CommandResponse::failure("AI_KEY_PAUSED_BY_PLAN", "errors.aiKey", true);
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
    let Some(model) = connection.model.as_deref() else {
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
    let Ok(entry) = catalog.resolve_model(provider, model, OperationType::CredentialTest) else {
        return CommandResponse::failure(
            "AI_MODEL_UNAVAILABLE",
            "errors.aiModelUnavailable",
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
    if window
        .state::<DesktopState>()
        .with_store(|s| crate::chatgpt_plan::load_settings(s).map(|v| v.0.enabled))
        .unwrap_or(true)
    {
        return CommandResponse::failure("AI_KEY_PAUSED_BY_PLAN", "errors.aiKey", true);
    }
    let app = window.app_handle().clone();
    let state = app.state::<DesktopState>();
    let gate = app.state::<AiRequestGate>();
    let attempt_id = Uuid::now_v7();
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
    let Some(model) = connection.model.as_deref() else {
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
    let Ok(entry) = catalog.resolve_model(provider, model, OperationType::CredentialTest) else {
        return CommandResponse::failure(
            "AI_MODEL_UNAVAILABLE",
            "errors.aiModelUnavailable",
            false,
        );
    };
    let entry = entry.clone();
    let request = NormalizedRequest {
        operation: OperationType::CredentialTest,
        model: entry.model.clone(),
        system: "Return only a JSON object with the boolean field ok set to true. No tools, files, user content, or additional text.".into(),
        input: json!({"test":"Open Resume Toolkit synthetic connection test"}),
        max_output_tokens: test_output_limit(provider),
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
    let Some(started) = now_unix_ms() else {
        return storage_unavailable();
    };
    let preflight = AiAttemptPreflight {
        operation_id,
        attempt_id,
        operation_type: OperationType::CredentialTest,
        provider,
        credential_id,
        requested_model: entry.model.clone(),
        preset_version: "explicit-model@direct-v2".into(),
        catalog_id: catalog.catalog_id.clone(),
        catalog_effective_from: entry.effective_from.clone(),
        pricing_components: entry.prices.clone(),
        started_at_unix_ms: started,
        estimated_input_tokens: u64::from(TEST_INPUT_ESTIMATE),
        maximum_cost_micros,
        currency: entry.currency.clone(),
        retry_of: None,
    };
    match execution::execute(
        &state,
        &lease.signal,
        preflight,
        http_request,
        url,
        &*provider_adapter,
        execution::Policy {
            timeout: Duration::from_secs(60),
            retry_server_error: true,
            keep_success_active: false,
        },
        |kind, text| {
            let _ = on_progress.send(AiProgress { kind, text });
        },
        |output| {
            if let Some(error) = output.material_error(&entry.model) {
                return Err(error);
            }
            if !output.valid(&entry.model) {
                return Err("AI_OUTPUT_INVALID");
            }
            output.usage.ok_or("AI_USAGE_UNKNOWN")
        },
    )
    .await
    {
        Ok(completed) => CommandResponse::success(AiTestResult {
            attempt_id: completed.attempt_id,
            effective_model: entry.model,
            usage: completed.value,
            estimated_cost_micros: completed.cost,
            usage_complete: completed.cost.is_some(),
            confirmed: true,
        }),
        Err(error) => {
            CommandResponse::failure(error.code, "errors.aiProviderFailed", error.retryable)
        }
    }
}

fn material_schema_bytes(
    provider: Provider,
    operation: OperationType,
) -> Result<usize, &'static str> {
    if !matches!(
        operation,
        OperationType::TailorResume | OperationType::RefineResume
    ) {
        return Ok(0);
    }
    let schema = match provider {
        Provider::OpenAi => ort_ai::materials::resume_output_schema(),
        Provider::Gemini => ort_ai::materials::gemini_resume_output_schema(),
        Provider::Anthropic => return Ok(0),
    };
    serde_json::to_vec(&schema)
        .map(|bytes| bytes.len())
        .map_err(|_| "AI_INPUT_INVALID")
}

#[cfg(test)]
#[path = "ai_request_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) struct MaterialFailure {
    pub code: &'static str,
    pub details: serde_json::Map<String, Value>,
    pub attempt_id: Option<Uuid>,
}
impl From<&'static str> for MaterialFailure {
    fn from(code: &'static str) -> Self {
        Self {
            code,
            details: json!({"diagnostic": {"code": code}})
                .as_object()
                .expect("diagnostic object")
                .clone(),
            attempt_id: None,
        }
    }
}
impl MaterialFailure {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn attempt(
        code: &'static str,
        diagnostic: &ort_domain::AiFailureDetails,
        operation_id: Uuid,
        attempt_id: Uuid,
        provider: Provider,
        operation: OperationType,
        model: &str,
        call: u8,
        maximum: u8,
        started: i64,
    ) -> Self {
        let details = json!({"diagnostic":diagnostic,"operationId":operation_id,"attemptId":attempt_id,"provider":provider.as_str(),"operationType":operation,"category":execution::failure_category(code),"model":model,"callNumber":call,"maximumCalls":maximum,"durationMs":now_unix_ms().unwrap_or(started).saturating_sub(started)}).as_object().expect("diagnostic object").clone();
        Self {
            code,
            details,
            attempt_id: Some(attempt_id),
        }
    }
    pub(crate) fn with_plan(
        mut self,
        reasoning: ort_ai::plan::ReasoningEffort,
        retries: u32,
    ) -> Self {
        self.details
            .insert("connectionSource".into(), json!("chatgpt_plan"));
        self.details.insert("reasoning".into(), json!(reasoning));
        self.details
            .insert("monetaryCostTracking".into(), json!("not_tracked"));
        self.details
            .insert("reportedRetries".into(), json!(retries));
        self
    }
}

#[path = "ai_material_provider.rs"]
pub(crate) mod material_provider;
#[path = "ai_material_request.rs"]
mod materials;
pub(crate) use materials::{MaterialDecision, execute_material, execute_material_sequence};
