//! Frozen provider selection and provider-specific dispatch/accounting. Stage
//! advancement and logical operation ownership live in `ai_material_request`.
use crate::{
    DesktopState,
    ai_request::{
        CancelSignal, MaterialFailure, adapter, execution, material_schema_bytes, now_unix_ms,
        pinned_url, provider,
    },
};
use ort_ai::{
    ApiKey, CatalogEntry, NormalizedRequest, OperationType, Provider, Usage, builtin_catalog,
    estimate_cost,
    plan::{PlanSettings, ReasoningEffort},
};
use ort_storage::ai_activity::AiAttemptPreflight;
use ort_vault::{OsProviderCredentialVault, ProviderCredentialReference, ProviderCredentialVault};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tauri::WebviewWindow;
use uuid::Uuid;

pub(crate) struct CompletedPass {
    pub attempt_id: Uuid,
    pub text: String,
    pub started_at_unix_ms: i64,
    pub provider: Provider,
    pub model: String,
    pub reasoning: Option<ReasoningEffort>,
    pub reported_retries: u32,
}
impl CompletedPass {
    pub(crate) fn failure(
        &self,
        code: &'static str,
        details: &ort_domain::AiFailureDetails,
        operation_id: Uuid,
        operation: OperationType,
        call: u8,
        maximum: u8,
    ) -> MaterialFailure {
        let failure = MaterialFailure::attempt(
            code,
            details,
            operation_id,
            self.attempt_id,
            self.provider,
            operation,
            &self.model,
            call,
            maximum,
            self.started_at_unix_ms,
        );
        match self.reasoning {
            Some(reasoning) => failure.with_plan(reasoning, self.reported_retries),
            None => failure,
        }
    }
}
pub(crate) struct PassContext<'a> {
    pub window: &'a WebviewWindow,
    pub state: &'a DesktopState,
    pub profile: Uuid,
    pub operation_id: Uuid,
    pub operation: OperationType,
    pub system: &'a str,
    pub input: Value,
    pub call: u8,
    pub maximum: u8,
    pub previous: Option<Uuid>,
    pub signal: Arc<CancelSignal>,
}
pub(crate) enum MaterialProvider {
    Codex(PlanSettings),
    Api(Box<ApiProvider>),
}
impl MaterialProvider {
    pub(crate) fn load(
        state: &DesktopState,
        operation: OperationType,
    ) -> Result<(Self, Uuid), MaterialFailure> {
        let (connection, channel, install, profile) = state
            .with_store(|store| {
                let connection = crate::ai_keys::request_connection(store, None)?;
                let (channel, install, profile) = store.vault_identity();
                Ok((connection, channel.to_owned(), install, profile))
            })
            .map_err(|_| "STORAGE_UNAVAILABLE")?;
        let selected = match connection.mode.as_str() {
            "chatgpt_plan" => Self::Codex(
                state
                    .with_store(|store| {
                        crate::chatgpt_plan::load_settings(store).map(|value| value.0)
                    })
                    .map_err(|_| "STORAGE_UNAVAILABLE")?,
            ),
            "direct_api" => {
                let provider = connection
                    .provider
                    .as_deref()
                    .and_then(provider)
                    .ok_or("AI_CONFIGURATION_INVALID")?;
                let model = connection
                    .model
                    .as_deref()
                    .ok_or("AI_CONFIGURATION_INVALID")?;
                let credential_id = connection.credential_id.ok_or("AI_CREDENTIAL_MISSING")?;
                let catalog = builtin_catalog(&jiff::Timestamp::now().to_string(), None)
                    .map_err(|_| "AI_CATALOG_UNAVAILABLE")?;
                let entry = catalog
                    .resolve_model(provider, model, operation)
                    .map_err(|_| "AI_MODEL_UNAVAILABLE")?
                    .clone();
                Self::Api(Box::new(ApiProvider {
                    provider,
                    credential_id,
                    entry,
                    catalog_id: catalog.catalog_id,
                    channel,
                    install,
                }))
            }
            _ => return Err("AI_DISABLED".into()),
        };
        Ok((selected, profile))
    }
    pub(crate) fn source(&self) -> &'static str {
        match self {
            Self::Codex(_) => "chatgpt_plan",
            Self::Api(_) => "direct_api",
        }
    }
    pub(crate) async fn execute(
        &self,
        context: PassContext<'_>,
    ) -> Result<CompletedPass, MaterialFailure> {
        match self {
            Self::Codex(settings) => {
                crate::codex_pass::execute(
                    context.window.clone(),
                    settings.clone(),
                    context.profile,
                    context.operation_id,
                    context.operation,
                    context.system.into(),
                    context.input,
                    context.call,
                    context.maximum,
                    context.previous,
                    context.signal,
                )
                .await
            }
            Self::Api(provider) => provider.execute(context).await,
        }
    }
}
pub(crate) struct ApiProvider {
    provider: Provider,
    credential_id: Uuid,
    entry: CatalogEntry,
    catalog_id: String,
    channel: String,
    install: Uuid,
}
impl ApiProvider {
    #[allow(clippy::too_many_lines)]
    async fn execute(&self, context: PassContext<'_>) -> Result<CompletedPass, MaterialFailure> {
        let PassContext {
            state,
            profile,
            operation_id,
            operation,
            system,
            input,
            call,
            maximum,
            previous,
            signal,
            ..
        } = context;
        let provider = self.provider;
        let credential_id = self.credential_id;
        let entry = &self.entry;
        let channel = &self.channel;
        let install = self.install;
        let attempt_id = Uuid::now_v7();
        let input_bytes = serde_json::to_vec(&input).map_err(|_| "AI_INPUT_INVALID")?;
        if input_bytes.len() > 100_000 || system.len() > 12_000 {
            return Err("AI_INPUT_INVALID".into());
        }
        // Include provider output schemas in the spending-cap reservation.
        let schema_bytes = material_schema_bytes(provider, operation)?;
        let input_bound = u32::try_from(
            input_bytes
                .len()
                .checked_add(system.len())
                .and_then(|value| value.checked_add(schema_bytes))
                .and_then(|value| value.checked_add(2_048))
                .ok_or("AI_INPUT_INVALID")?,
        )
        .map_err(|_| "AI_INPUT_INVALID")?;
        let output_bound = entry.max_output_tokens.min(ort_ai::MAX_OUTPUT_TOKENS);
        if input_bound > entry.max_input_tokens {
            return Err("AI_INPUT_TOO_LARGE".into());
        }
        let mut max_usage = Usage {
            input_tokens: u64::from(input_bound),
            output_tokens: u64::from(output_bound),
            ..Usage::default()
        };
        for price in &entry.prices {
            match price.category {
                ort_ai::PriceCategory::CachedInput => {
                    max_usage.cached_input_tokens = u64::from(input_bound);
                }
                ort_ai::PriceCategory::CacheWrite => {
                    max_usage.cache_write_tokens = u64::from(input_bound);
                }
                ort_ai::PriceCategory::Reasoning => {
                    max_usage.reasoning_tokens = u64::from(output_bound);
                }
                _ => {}
            }
        }
        let maximum_cost_micros =
            estimate_cost(entry, max_usage).map_err(|_| "AI_PRICE_UNAVAILABLE")?;
        let request = NormalizedRequest {
            operation,
            model: entry.model.clone(),
            system: system.into(),
            input: input.clone(),
            max_output_tokens: output_bound,
        };
        let reference = ProviderCredentialReference::new(
            channel,
            &install.to_string(),
            &profile.to_string(),
            provider.as_str(),
            &credential_id.to_string(),
        )
        .map_err(|_| "AI_CREDENTIAL_MISSING")?;
        let provider_adapter = adapter(provider);
        let built = OsProviderCredentialVault::new().use_secret(&reference, |secret| {
            secret.expose_for(|bytes| {
                let key = ApiKey::new(bytes)?;
                provider_adapter.build_request(&request, &key)
            })
        });
        let http_request = built
            .map_err(|_| "AI_CREDENTIAL_MISSING")?
            .map_err(|_| "AI_PROVIDER_INVALID")?;
        let url = pinned_url(provider, &entry.model, &http_request).ok_or("AI_PROVIDER_INVALID")?;
        let started = now_unix_ms().ok_or("STORAGE_UNAVAILABLE")?;
        let preflight = AiAttemptPreflight {
            operation_id,
            attempt_id,
            operation_type: operation,
            provider,
            credential_id,
            requested_model: entry.model.clone(),
            preset_version: "explicit-model@direct-v2".into(),
            catalog_id: self.catalog_id.clone(),
            catalog_effective_from: entry.effective_from.clone(),
            pricing_components: entry.prices.clone(),
            started_at_unix_ms: started,
            estimated_input_tokens: u64::from(input_bound),
            maximum_cost_micros,
            currency: entry.currency.clone(),
            retry_of: previous,
        };
        let completed = execution::execute(
            state,
            &signal,
            preflight,
            http_request,
            url,
            &*provider_adapter,
            execution::Policy {
                timeout: Duration::from_secs(120),
                retry_server_error: false,
                keep_success_active: maximum > 1,
            },
            |_, _| {},
            |output| {
                if let Some(code) = output.material_error(&entry.model) {
                    return Err(code);
                }
                if maximum > 1 && output.usage.is_none() {
                    return Err("AI_USAGE_UNKNOWN");
                }
                Ok(output.text.clone())
            },
        )
        .await
        .map_err(|error| {
            MaterialFailure::attempt(
                error.code,
                &error.details,
                operation_id,
                attempt_id,
                provider,
                operation,
                &entry.model,
                call,
                maximum,
                started,
            )
        })?;
        Ok(CompletedPass {
            attempt_id: completed.attempt_id,
            text: completed.value,
            started_at_unix_ms: started,
            provider,
            model: entry.model.clone(),
            reasoning: None,
            reported_retries: 0,
        })
    }
}
