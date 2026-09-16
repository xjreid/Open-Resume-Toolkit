use ort_vault::{
    OsProviderCredentialVault, ProviderCredentialReference, ProviderCredentialVault, ProviderSecret,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{State, WebviewWindow};
use uuid::Uuid;

use crate::{DesktopState, ai_request::AiRequestGate, storage_unavailable, window_not_authorized};
use ort_domain::CommandResponse;
use ort_platform::{ExportDestination, ExportFileType, ExportWriteError};
use ort_storage::ai_activity::{AiBucketSize, AiMonitoringSummary};
use ort_storage::ai_activity::{AiCapPolicy, AiCapPolicySummary, AiPeriod, ai_calendar_bounds};
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::text_export::ExportState;

const SETTING: &str = "ai.connection.v1";
const RETENTION_SETTING: &str = "ai.history_retention.v1";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRetentionSummary {
    policy: String,
    removed_operations: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveAiCapRequest {
    period: AiPeriod,
    limit_micros: u64,
    time_zone: String,
    expected_revision: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiCapActionRequest {
    period: AiPeriod,
}

fn now_unix_ms() -> Option<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

fn retention_age_ms(policy: &str) -> Option<i64> {
    let days = match policy {
        "30_days" => 30,
        "90_days" => 90,
        "one_year" => 365,
        _ => return None,
    };
    Some(days * 24 * 60 * 60 * 1_000)
}

fn valid_retention(policy: &str) -> bool {
    matches!(
        policy,
        "30_days" | "90_days" | "one_year" | "retain_until_cleared"
    )
}

fn apply_retention(
    store: &ort_storage::EncryptedStore,
    policy: &str,
) -> Result<u64, ort_storage::StorageError> {
    let Some(age) = retention_age_ms(policy) else {
        return Ok(0);
    };
    let cutoff = now_unix_ms()
        .ok_or(ort_storage::StorageError::Unavailable)?
        .saturating_sub(age);
    if cutoff <= 0 {
        return Ok(0);
    }
    store.clear_ai_activity(0, cutoff)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_retention(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> CommandResponse<AiRetentionSummary> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        let policy = store
            .load_setting(RETENTION_SETTING)?
            .and_then(|setting| setting.value.as_str().map(ToOwned::to_owned))
            .filter(|value| valid_retention(value))
            .unwrap_or_else(|| "retain_until_cleared".into());
        let removed_operations = apply_retention(store, &policy)?;
        Ok(AiRetentionSummary {
            policy,
            removed_operations,
        })
    }) {
        Ok(summary) => CommandResponse::success(summary),
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn save_ai_retention(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    policy: String,
) -> CommandResponse<AiRetentionSummary> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if !valid_retention(&policy) {
        return CommandResponse::failure(
            "AI_RETENTION_INVALID",
            "errors.aiRetentionInvalid",
            false,
        );
    }
    match state.with_store(|store| {
        let revision = store
            .load_setting(RETENTION_SETTING)?
            .map(|setting| setting.revision);
        store.save_setting(RETENTION_SETTING, revision, &json!(policy))?;
        let removed_operations = apply_retention(store, &policy)?;
        Ok(AiRetentionSummary {
            policy,
            removed_operations,
        })
    }) {
        Ok(summary) => CommandResponse::success(summary),
        Err(ort_storage::StorageError::RevisionConflict) => {
            CommandResponse::failure("AI_BUSY", "errors.aiBusy", true)
        }
        Err(_) => storage_unavailable(),
    }
}

fn cap_context(
    store: &ort_storage::EncryptedStore,
) -> Result<(AiConnectionState, Uuid), ort_storage::StorageError> {
    let saved = store
        .load_setting(SETTING)?
        .ok_or(ort_storage::StorageError::NotFound)?;
    let connection: AiConnectionState =
        serde_json::from_value(saved.value).map_err(|_| ort_storage::StorageError::InvalidData)?;
    let credential = connection
        .credential_id
        .ok_or(ort_storage::StorageError::NotFound)?;
    Ok((connection, credential))
}

pub(crate) fn saved_credential_reference(
    store: &ort_storage::EncryptedStore,
) -> Result<Option<ProviderCredentialReference>, ort_storage::StorageError> {
    let Some(saved) = store.load_setting(SETTING)? else {
        return Ok(None);
    };
    let connection: AiConnectionState =
        serde_json::from_value(saved.value).map_err(|_| ort_storage::StorageError::InvalidData)?;
    let (Some(provider), Some(credential_id)) = (connection.provider, connection.credential_id)
    else {
        return Ok(None);
    };
    let (channel, install, profile) = store.vault_identity();
    ProviderCredentialReference::new(
        channel,
        &install.to_string(),
        &profile.to_string(),
        &provider,
        &credential_id.to_string(),
    )
    .map(Some)
    .map_err(|_| ort_storage::StorageError::InvalidData)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_caps(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> CommandResponse<Vec<AiCapPolicySummary>> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        let (_, credential) = cap_context(store)?;
        store.ai_cap_policies(credential)
    }) {
        Ok(policies) => CommandResponse::success(policies),
        Err(ort_storage::StorageError::NotFound) => CommandResponse::success(Vec::new()),
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn save_ai_cap(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: SaveAiCapRequest,
) -> CommandResponse<AiCapPolicySummary> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if request.limit_micros == 0
        || request.limit_micros > 9_007_199_254_740_991
        || request.time_zone.len() > 128
    {
        return CommandResponse::failure("AI_CAP_INVALID", "errors.aiCapInvalid", false);
    }
    let Some(now) = now_unix_ms() else {
        return storage_unavailable();
    };
    let result = state.with_store(|store| {
        let (connection, credential) = cap_context(store)?;
        if !matches!(connection.mode, ConnectionMode::DirectApi) {
            return Err(ort_storage::StorageError::InvalidData);
        }
        let provider = match connection.provider.as_deref() {
            Some("openai") => ort_ai::Provider::OpenAi,
            Some("anthropic") => ort_ai::Provider::Anthropic,
            Some("gemini") => ort_ai::Provider::Gemini,
            _ => return Err(ort_storage::StorageError::InvalidData),
        };
        let preset = match connection.preset.as_deref() {
            Some("economy") => ort_ai::Preset::Economy,
            Some("balanced") => ort_ai::Preset::Balanced,
            Some("quality") => ort_ai::Preset::Quality,
            _ => return Err(ort_storage::StorageError::InvalidData),
        };
        let catalog = ort_ai::builtin_catalog(&jiff::Timestamp::now().to_string(), None)
            .map_err(|_| ort_storage::StorageError::InvalidData)?;
        let currency = catalog
            .resolve(provider, preset, ort_ai::OperationType::CredentialTest)
            .map_err(|_| ort_storage::StorageError::InvalidData)?
            .currency
            .clone();
        let existing = store
            .ai_cap_policies(credential)?
            .into_iter()
            .find(|policy| policy.period == request.period);
        let (activated, start, end) = if let Some(existing) = &existing {
            (
                existing.activated_at_unix_ms,
                existing.period_start_unix_ms,
                existing.period_end_unix_ms,
            )
        } else {
            let (start, end) = ai_calendar_bounds(now, &request.time_zone, request.period)?;
            (now, start, end)
        };
        store.save_ai_cap_policy(&AiCapPolicy {
            credential_id: credential,
            period: request.period,
            currency,
            time_zone: request.time_zone,
            limit_micros: request.limit_micros,
            activated_at_unix_ms: activated,
            period_start_unix_ms: start,
            period_end_unix_ms: end,
            expected_revision: request.expected_revision,
        })?;
        store
            .ai_cap_policies(credential)?
            .into_iter()
            .find(|policy| policy.period == request.period)
            .ok_or(ort_storage::StorageError::NotFound)
    });
    match result {
        Ok(policy) => CommandResponse::success(policy),
        Err(ort_storage::StorageError::RevisionConflict) => {
            CommandResponse::failure("AI_CAP_CHANGED", "errors.aiCapChanged", true)
        }
        Err(ort_storage::StorageError::InvalidData) => {
            CommandResponse::failure("AI_CAP_INVALID", "errors.aiCapInvalid", false)
        }
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn disable_ai_cap(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: AiCapActionRequest,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        let (_, credential) = cap_context(store)?;
        store.disable_ai_cap(credential, request.period)
    }) {
        Ok(()) => CommandResponse::success(true),
        Err(ort_storage::StorageError::RevisionConflict) => {
            CommandResponse::failure("AI_BUSY", "errors.aiBusy", true)
        }
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn reset_ai_cap(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: AiCapActionRequest,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if request.period != AiPeriod::AllTime {
        return CommandResponse::failure("AI_CAP_INVALID", "errors.aiCapInvalid", false);
    }
    match state.with_store(|store| {
        let (_, credential) = cap_context(store)?;
        store.reset_ai_cap(
            credential,
            request.period,
            now_unix_ms().ok_or(ort_storage::StorageError::Unavailable)?,
        )
    }) {
        Ok(()) => CommandResponse::success(true),
        Err(ort_storage::StorageError::RevisionConflict) => {
            CommandResponse::failure("AI_BUSY", "errors.aiBusy", true)
        }
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_monitoring(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    from_unix_ms: i64,
    to_unix_ms: i64,
    time_zone: String,
    bucket_size: String,
) -> CommandResponse<AiMonitoringSummary> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if from_unix_ms < 0 || to_unix_ms <= from_unix_ms || to_unix_ms > 8_640_000_000_000_000 {
        return CommandResponse::failure("AI_PERIOD_INVALID", "errors.aiPeriodInvalid", false);
    }
    let bucket_size = match bucket_size.as_str() {
        "day" => AiBucketSize::Day,
        "month" => AiBucketSize::Month,
        _ => return CommandResponse::failure("AI_PERIOD_INVALID", "errors.aiPeriodInvalid", false),
    };
    match state.with_store(|store| {
        store.ai_monitoring_summary(from_unix_ms, to_unix_ms, &time_zone, bucket_size)
    }) {
        Ok(summary) => CommandResponse::success(summary),
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn clear_ai_monitoring(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    from_unix_ms: i64,
    to_unix_ms: i64,
) -> CommandResponse<u64> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if from_unix_ms < 0 || to_unix_ms <= from_unix_ms || to_unix_ms > 8_640_000_000_000_000 {
        return CommandResponse::failure("AI_PERIOD_INVALID", "errors.aiPeriodInvalid", false);
    }
    match state.with_store(|store| store.clear_ai_activity(from_unix_ms, to_unix_ms)) {
        Ok(cleared) => CommandResponse::success(cleared),
        Err(ort_storage::StorageError::RevisionConflict) => {
            CommandResponse::failure("AI_BUSY", "errors.aiBusy", true)
        }
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
pub async fn export_ai_monitoring(
    window: WebviewWindow,
    from_unix_ms: i64,
    to_unix_ms: i64,
    time_zone: String,
    bucket_size: String,
) -> CommandResponse<String> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if from_unix_ms < 0 || to_unix_ms <= from_unix_ms || to_unix_ms > 8_640_000_000_000_000 {
        return CommandResponse::failure("AI_PERIOD_INVALID", "errors.aiPeriodInvalid", false);
    }
    let app = window.app_handle().clone();
    let state = app.state::<DesktopState>();
    let exports = app.state::<ExportState>();
    let Some(lease) = exports.begin() else {
        return CommandResponse::failure("EXPORT_BUSY", "errors.aiExport", true);
    };
    let bucket_size = match bucket_size.as_str() {
        "day" => AiBucketSize::Day,
        "month" => AiBucketSize::Month,
        _ => return CommandResponse::failure("AI_PERIOD_INVALID", "errors.aiPeriodInvalid", false),
    };
    let Ok(bytes) = state.with_store(|store| {
        store.export_ai_monitoring_json(from_unix_ms, to_unix_ms, &time_zone, bucket_size)
    }) else {
        return storage_unavailable();
    };
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let selection = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Export unencrypted AI aggregate JSON — choose a new filename")
            .set_file_name("ort-ai-monitoring.json")
            .add_filter("JSON", &["json"])
            .blocking_save_file();
        let Some(selection) = selection else {
            return CommandResponse::success("cancelled".into());
        };
        let FilePath::Path(path) = selection else {
            return CommandResponse::failure(
                "EXPORT_INVALID_DESTINATION",
                "errors.aiExport",
                false,
            );
        };
        match ExportDestination::for_native_dialog(&path, ExportFileType::Json)
            .and_then(|destination| destination.write(&bytes))
        {
            Ok(_) => CommandResponse::success("exported".into()),
            Err(error) => CommandResponse::failure(
                match error {
                    ExportWriteError::AlreadyExists => "EXPORT_ALREADY_EXISTS",
                    ExportWriteError::InvalidDestination => "EXPORT_INVALID_DESTINATION",
                    ExportWriteError::InvalidContent => "EXPORT_INVALID_CONTENT",
                    ExportWriteError::Unavailable => "EXPORT_UNAVAILABLE",
                },
                "errors.aiExport",
                false,
            ),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) => CommandResponse::failure("EXPORT_OUTCOME_UNKNOWN", "errors.aiExport", true),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ConnectionMode {
    NoAi,
    DirectApi,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionState {
    mode: ConnectionMode,
    provider: Option<String>,
    preset: Option<String>,
    credential_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveAiConnectionRequest {
    provider: String,
    preset: String,
    api_key: String,
    #[serde(default)]
    copy_guardrails: bool,
}

fn no_ai() -> AiConnectionState {
    AiConnectionState {
        mode: ConnectionMode::NoAi,
        provider: None,
        preset: None,
        credential_id: None,
    }
}
fn valid_provider(value: &str) -> bool {
    matches!(value, "openai" | "anthropic" | "gemini")
}
fn valid_preset(value: &str) -> bool {
    matches!(value, "economy" | "balanced" | "quality")
}

fn catalog_selection(provider: &str, preset: &str) -> Option<(ort_ai::Provider, ort_ai::Preset)> {
    let provider = match provider {
        "openai" => ort_ai::Provider::OpenAi,
        "anthropic" => ort_ai::Provider::Anthropic,
        "gemini" => ort_ai::Provider::Gemini,
        _ => return None,
    };
    let preset = match preset {
        "economy" => ort_ai::Preset::Economy,
        "balanced" => ort_ai::Preset::Balanced,
        "quality" => ort_ai::Preset::Quality,
        _ => return None,
    };
    Some((provider, preset))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_catalog(window: WebviewWindow) -> CommandResponse<ort_ai::Catalog> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match ort_ai::builtin_catalog(&jiff::Timestamp::now().to_string(), None) {
        Ok(catalog) => CommandResponse::success(catalog),
        Err(_) => CommandResponse::failure(
            "AI_CATALOG_UNAVAILABLE",
            "errors.aiCatalogUnavailable",
            false,
        ),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_connection(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> CommandResponse<AiConnectionState> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| store.load_setting(SETTING)) {
        Ok(Some(value)) => match serde_json::from_value(value.value) {
            Ok(connection) => CommandResponse::success(connection),
            Err(_) => storage_unavailable(),
        },
        Ok(None) => CommandResponse::success(no_ai()),
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_lines)] // Keep credential write, cap rebinding, and rollback order together.
pub fn save_ai_connection(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
    request: SaveAiConnectionRequest,
) -> CommandResponse<AiConnectionState> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    if !valid_provider(&request.provider) || !valid_preset(&request.preset) {
        return CommandResponse::failure(
            "AI_CONFIGURATION_INVALID",
            "errors.aiConfigurationInvalid",
            false,
        );
    }
    let Some((provider, preset)) = catalog_selection(&request.provider, &request.preset) else {
        return CommandResponse::failure(
            "AI_CONFIGURATION_INVALID",
            "errors.aiConfigurationInvalid",
            false,
        );
    };
    let now = jiff::Timestamp::now().to_string();
    let Ok(catalog) = ort_ai::builtin_catalog(&now, None) else {
        return CommandResponse::failure(
            "AI_CATALOG_UNAVAILABLE",
            "errors.aiCatalogUnavailable",
            false,
        );
    };
    if catalog
        .resolve(provider, preset, ort_ai::OperationType::CredentialTest)
        .is_err()
    {
        return CommandResponse::failure(
            "AI_PRESET_UNAVAILABLE",
            "errors.aiPresetUnavailable",
            false,
        );
    }
    let Ok(secret) = ProviderSecret::from_bytes(request.api_key.into_bytes()) else {
        return CommandResponse::failure(
            "AI_CREDENTIAL_INVALID",
            "errors.aiCredentialInvalid",
            false,
        );
    };
    let credential_id = Uuid::now_v7();
    let prepared = state.with_store(|store| {
        let (channel, install, profile) = store.vault_identity();
        let reference = ProviderCredentialReference::new(
            channel,
            &install.to_string(),
            &profile.to_string(),
            &request.provider,
            &credential_id.to_string(),
        )
        .map_err(|_| ort_storage::StorageError::InvalidData)?;
        let current = store.load_setting(SETTING)?;
        let old_reference = saved_credential_reference(store)?;
        let old_credential = current
            .as_ref()
            .and_then(|value| serde_json::from_value::<AiConnectionState>(value.value.clone()).ok())
            .and_then(|value| value.credential_id);
        Ok((
            reference,
            current.as_ref().map(|value| value.revision),
            current.map(|value| value.value),
            old_reference,
            old_credential,
        ))
    });
    let Ok((reference, revision, old_value, old_reference, old_credential)) = prepared else {
        return storage_unavailable();
    };
    let vault = OsProviderCredentialVault::new();
    if vault.store_new(&reference, &secret).is_err() {
        return CommandResponse::failure("AI_VAULT_UNAVAILABLE", "errors.aiVaultUnavailable", true);
    }
    let connection = AiConnectionState {
        mode: ConnectionMode::DirectApi,
        provider: Some(request.provider),
        preset: Some(request.preset),
        credential_id: Some(credential_id),
    };
    let value = json!(connection);
    let saved = state.with_store(|store| store.save_setting(SETTING, revision, &value));
    let Ok(saved) = saved else {
        let _ = vault.delete(&reference);
        return storage_unavailable();
    };
    if let Some(old_credential) = old_credential {
        let transferred = state.with_store(|store| {
            store.replace_ai_cap_identity(
                old_credential,
                credential_id,
                request.copy_guardrails,
                now_unix_ms().ok_or(ort_storage::StorageError::Unavailable)?,
            )
        });
        if transferred.is_err() {
            if let Some(old_value) = old_value
                && state
                    .with_store(|store| {
                        store
                            .save_setting(SETTING, Some(saved.revision), &old_value)
                            .map(|_| ())
                    })
                    .is_err()
            {
                return CommandResponse::failure(
                    "AI_CREDENTIAL_CLEANUP_REQUIRED",
                    "errors.aiCredentialCleanupRequired",
                    false,
                );
            }
            let _ = vault.delete(&reference);
            return storage_unavailable();
        }
    }
    if old_reference
        .as_ref()
        .is_some_and(|old| vault.delete(old).is_err())
    {
        return CommandResponse::failure(
            "AI_OLD_CREDENTIAL_CLEANUP_REQUIRED",
            "errors.aiCredentialCleanupRequired",
            false,
        );
    }
    CommandResponse::success(connection)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn disable_ai(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
) -> CommandResponse<AiConnectionState> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    let result = state.with_store(|store| {
        let current = store.load_setting(SETTING)?;
        let Some(current) = current else {
            return Ok(no_ai());
        };
        let mut connection: AiConnectionState = serde_json::from_value(current.value)
            .map_err(|_| ort_storage::StorageError::InvalidData)?;
        connection.mode = ConnectionMode::NoAi;
        store
            .save_setting(SETTING, Some(current.revision), &json!(connection))
            .map(|_| connection)
    });
    match result {
        Ok(connection) => CommandResponse::success(connection),
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn activate_saved_ai(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
) -> CommandResponse<AiConnectionState> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    let result = state.with_store(|store| {
        let current = store
            .load_setting(SETTING)?
            .ok_or(ort_storage::StorageError::NotFound)?;
        let mut connection: AiConnectionState = serde_json::from_value(current.value)
            .map_err(|_| ort_storage::StorageError::InvalidData)?;
        if connection.provider.is_none()
            || connection.preset.is_none()
            || connection.credential_id.is_none()
        {
            return Err(ort_storage::StorageError::InvalidData);
        }
        let reference =
            saved_credential_reference(store)?.ok_or(ort_storage::StorageError::NotFound)?;
        OsProviderCredentialVault::new()
            .use_secret(&reference, |_| ())
            .map_err(|_| ort_storage::StorageError::VaultKeyUnavailable)?;
        connection.mode = ConnectionMode::DirectApi;
        store
            .save_setting(SETTING, Some(current.revision), &json!(connection))
            .map(|_| connection)
    });
    match result {
        Ok(connection) => CommandResponse::success(connection),
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn remove_ai_credential(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
) -> CommandResponse<AiConnectionState> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    let prepared = state.with_store(|store| {
        let current = store
            .load_setting(SETTING)?
            .ok_or(ort_storage::StorageError::NotFound)?;
        let connection: AiConnectionState = serde_json::from_value(current.value.clone())
            .map_err(|_| ort_storage::StorageError::InvalidData)?;
        let provider = connection
            .provider
            .ok_or(ort_storage::StorageError::InvalidData)?;
        let credential = connection
            .credential_id
            .ok_or(ort_storage::StorageError::InvalidData)?;
        let (channel, install, profile) = store.vault_identity();
        let reference = ProviderCredentialReference::new(
            channel,
            &install.to_string(),
            &profile.to_string(),
            &provider,
            &credential.to_string(),
        )
        .map_err(|_| ort_storage::StorageError::InvalidData)?;
        Ok((reference, credential, current.revision, current.value))
    });
    let Ok((reference, credential, revision, old_value)) = prepared else {
        return storage_unavailable();
    };
    let saved =
        state.with_store(|store| store.save_setting(SETTING, Some(revision), &json!(no_ai())));
    let Ok(saved) = saved else {
        return storage_unavailable();
    };
    if OsProviderCredentialVault::new().delete(&reference).is_err() {
        if state
            .with_store(|store| {
                store
                    .save_setting(SETTING, Some(saved.revision), &old_value)
                    .map(|_| ())
            })
            .is_err()
        {
            return CommandResponse::failure(
                "AI_CREDENTIAL_CLEANUP_REQUIRED",
                "errors.aiCredentialCleanupRequired",
                false,
            );
        }
        return CommandResponse::failure("AI_VAULT_UNAVAILABLE", "errors.aiVaultUnavailable", true);
    }
    if state
        .with_store(|store| store.remove_ai_cap_identity(credential))
        .is_err()
    {
        return CommandResponse::failure(
            "AI_CREDENTIAL_CLEANUP_REQUIRED",
            "errors.aiCredentialCleanupRequired",
            false,
        );
    }
    CommandResponse::success(no_ai())
}
