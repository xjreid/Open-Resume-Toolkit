use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{State, WebviewWindow};
use uuid::Uuid;

use crate::{DesktopState, ai_keys::AiConnectionState, storage_unavailable, window_not_authorized};
use ort_domain::CommandResponse;
use ort_platform::{ExportDestination, ExportFileType, ExportWriteError};
use ort_storage::ai_activity::AI_GENERAL_CAP_CREDENTIAL_ID;
use ort_storage::ai_activity::{AiBucketSize, AiMonitoringSummary};
use ort_storage::ai_activity::{AiCapPolicy, AiCapPolicySummary, AiPeriod, ai_calendar_bounds};
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::text_export::ExportState;

const RETENTION_SETTING: &str = "ai.history_retention.v1";
const MAX_UNIX_MS: i64 = 8_640_000_000_000_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiActivityMonth {
    label: String,
    from_unix_ms: i64,
    to_unix_ms: i64,
}

fn normalize_activity_months(mut months: Vec<AiActivityMonth>) -> Option<Vec<AiActivityMonth>> {
    if months.is_empty() || months.len() > 1_200 {
        return None;
    }
    months.sort_by_key(|month| month.from_unix_ms);
    let valid_label = |label: &str| {
        let bytes = label.as_bytes();
        bytes.len() == 7
            && bytes[4] == b'-'
            && bytes[..4].iter().all(u8::is_ascii_digit)
            && bytes[5..].iter().all(u8::is_ascii_digit)
            && matches!(
                &label[5..],
                "01" | "02" | "03" | "04" | "05" | "06" | "07" | "08" | "09" | "10" | "11" | "12"
            )
    };
    if months.iter().any(|month| {
        !valid_label(&month.label)
            || month.from_unix_ms < 0
            || month.to_unix_ms <= month.from_unix_ms
            || month.to_unix_ms > MAX_UNIX_MS
    }) || months
        .windows(2)
        .any(|pair| pair[0].to_unix_ms > pair[1].from_unix_ms || pair[0].label == pair[1].label)
    {
        return None;
    }
    Some(months)
}

fn normalize_credential_ids(credential_ids: Option<Vec<Uuid>>) -> Result<Option<Vec<Uuid>>, ()> {
    let Some(ids) = credential_ids else {
        return Ok(None);
    };
    let unique = ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    if ids.is_empty() || ids.len() > 1_000 || unique.len() != ids.len() {
        return Err(());
    }
    Ok(Some(ids))
}

#[cfg(test)]
mod key_budget_tests {
    use super::*;
    use ort_vault::testing::MemoryDatabaseKeyVault;

    #[test]
    fn paused_non_primary_keys_can_be_customized_and_calendar_caps_consolidate_once() {
        let temp = tempfile::tempdir().unwrap();
        let store = ort_storage::EncryptedStore::open_or_initialize(
            temp.path(),
            "test",
            &MemoryDatabaseKeyVault::new(),
        )
        .unwrap();
        let id = Uuid::from_u128(1);
        let other = Uuid::from_u128(2);
        store.save_setting("ai.connection.v1", None, &json!({
            "keys":[{"credentialId":id,"identificationNumber":1,"provider":"openai","preset":"balanced","paused":true,"removed":false,"cleanupRequired":false}],
            "primaryCredentialId":null,"nextIdentificationNumber":2,
        })).unwrap();
        assert_eq!(cap_context(&store, id).unwrap().0.mode, "no_ai");
        assert!(cap_context(&store, other).is_err());
        for (credential, period, limit) in [
            (id, AiPeriod::Week, 100),
            (id, AiPeriod::Month, 200),
            (other, AiPeriod::Month, 300),
        ] {
            store
                .save_ai_cap_policy(&AiCapPolicy {
                    credential_id: credential,
                    period,
                    currency: "USD".into(),
                    time_zone: "UTC".into(),
                    limit_micros: limit,
                    activated_at_unix_ms: 1_000,
                    period_start_unix_ms: 1_000,
                    period_end_unix_ms: Some(100_000),
                    expected_revision: None,
                })
                .unwrap();
        }
        let cap = unified_cap(&store, id).unwrap().unwrap();
        assert_eq!(cap.period, AiPeriod::AllTime);
        assert_eq!(cap.limit_micros, 100);
        assert_eq!(store.ai_cap_policies(id).unwrap().len(), 1);
        assert_eq!(
            store.ai_cap_policies(other).unwrap()[0].period,
            AiPeriod::Month
        );
        assert_eq!(
            unified_cap(&store, id).unwrap().unwrap().revision,
            cap.revision
        );
        store
            .reset_ai_cap(id, AiPeriod::AllTime, cap.activated_at_unix_ms + 1)
            .unwrap();
        assert_eq!(
            crate::ai_keys::load_registry(&store)
                .unwrap()
                .0
                .primary_credential_id,
            None
        );
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRetentionSummary {
    policy: String,
    removed_operations: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveAiCapRequest {
    credential_id: Uuid,
    period: AiPeriod,
    limit_micros: u64,
    time_zone: String,
    expected_revision: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiCapActionRequest {
    credential_id: Uuid,
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
    credential: Uuid,
) -> Result<(AiConnectionState, Uuid), ort_storage::StorageError> {
    let connection = crate::ai_keys::request_connection(store, Some(credential))?;
    let registry = crate::ai_keys::load_registry(store)?.0;
    if !registry
        .keys
        .iter()
        .any(|key| key.credential_id == credential && !key.removed && !key.cleanup_required)
    {
        return Err(ort_storage::StorageError::NotFound);
    }
    Ok((connection, credential))
}

pub(crate) fn unified_cap(
    store: &ort_storage::EncryptedStore,
    credential: Uuid,
) -> Result<Option<AiCapPolicySummary>, ort_storage::StorageError> {
    let policies = store.ai_cap_policies(credential)?;
    if policies
        .iter()
        .any(|policy| policy.period != AiPeriod::AllTime)
    {
        let existing = policies
            .iter()
            .find(|policy| policy.period == AiPeriod::AllTime);
        let policy = existing
            .or_else(|| policies.iter().min_by_key(|policy| policy.limit_micros))
            .ok_or(ort_storage::StorageError::NotFound)?;
        let now = now_unix_ms().ok_or(ort_storage::StorageError::Unavailable)?;
        store.save_ai_unified_cap(&AiCapPolicy {
            credential_id: credential,
            period: AiPeriod::AllTime,
            currency: policy.currency.clone(),
            time_zone: policy.time_zone.clone(),
            limit_micros: policy.limit_micros,
            activated_at_unix_ms: existing.map_or(now, |cap| cap.activated_at_unix_ms),
            period_start_unix_ms: existing.map_or(now, |cap| cap.period_start_unix_ms),
            period_end_unix_ms: None,
            expected_revision: existing.map(|cap| cap.revision),
        })?;
    }
    Ok(store
        .ai_cap_policies(credential)?
        .into_iter()
        .find(|cap| cap.period == AiPeriod::AllTime))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiKeySettings {
    cap: Option<AiCapPolicySummary>,
    lifetime_spend_by_currency_micros: std::collections::BTreeMap<String, u64>,
    lifetime_spend_partial: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGeneralSettings {
    cap: Option<AiCapPolicySummary>,
    lifetime_spend_by_currency_micros: std::collections::BTreeMap<String, u64>,
    lifetime_spend_partial: bool,
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_general_settings(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> CommandResponse<AiGeneralSettings> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        Ok(AiGeneralSettings {
            cap: unified_cap(store, AI_GENERAL_CAP_CREDENTIAL_ID)?,
            lifetime_spend_by_currency_micros: store.ai_lifetime_spend_all()?,
            lifetime_spend_partial: store.ai_lifetime_spend_all_is_partial()?,
        })
    }) {
        Ok(value) => CommandResponse::success(value),
        Err(_) => storage_unavailable(),
    }
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_key_settings(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    credential_id: Uuid,
) -> CommandResponse<AiKeySettings> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        cap_context(store, credential_id)?;
        Ok(AiKeySettings {
            cap: unified_cap(store, credential_id)?,
            lifetime_spend_by_currency_micros: store.ai_lifetime_spend(credential_id)?,
            lifetime_spend_partial: store.ai_lifetime_spend_is_partial(credential_id)?,
        })
    }) {
        Ok(value) => CommandResponse::success(value),
        Err(ort_storage::StorageError::RevisionConflict) => {
            CommandResponse::failure("AI_BUSY", "errors.aiBusy", true)
        }
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_caps(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    credential_id: Option<Uuid>,
) -> CommandResponse<Vec<AiCapPolicySummary>> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        let credential = credential_id
            .or(crate::ai_keys::load_registry(store)?
                .0
                .primary_credential_id)
            .ok_or(ort_storage::StorageError::NotFound)?;
        cap_context(store, credential)?;
        Ok(unified_cap(store, credential)?.into_iter().collect())
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
    if request.period != AiPeriod::AllTime
        || request.limit_micros == 0
        || request.limit_micros > 9_007_199_254_740_991
        || request.time_zone.len() > 128
    {
        return CommandResponse::failure("AI_CAP_INVALID", "errors.aiCapInvalid", false);
    }
    let Some(now) = now_unix_ms() else {
        return storage_unavailable();
    };
    let result = state.with_store(|store| {
        let (connection, credential) = cap_context(store, request.credential_id)?;
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
        store.save_ai_unified_cap(&AiCapPolicy {
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
pub fn save_ai_general_cap(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    limit_micros: u64,
    time_zone: String,
    expected_revision: Option<u64>,
) -> CommandResponse<AiCapPolicySummary> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if limit_micros == 0 || limit_micros > 9_007_199_254_740_991 || time_zone.len() > 128 {
        return CommandResponse::failure("AI_CAP_INVALID", "errors.aiCapInvalid", false);
    }
    let Some(now) = now_unix_ms() else {
        return storage_unavailable();
    };
    let result = state.with_store(|store| {
        let existing = unified_cap(store, AI_GENERAL_CAP_CREDENTIAL_ID)?;
        let (activated, start) = existing.as_ref().map_or((now, now), |cap| {
            (cap.activated_at_unix_ms, cap.period_start_unix_ms)
        });
        store.save_ai_unified_cap(&AiCapPolicy {
            credential_id: AI_GENERAL_CAP_CREDENTIAL_ID,
            period: AiPeriod::AllTime,
            currency: existing
                .as_ref()
                .map_or_else(|| "USD".into(), |cap| cap.currency.clone()),
            time_zone: existing
                .as_ref()
                .map_or_else(|| time_zone.clone(), |cap| cap.time_zone.clone()),
            limit_micros,
            activated_at_unix_ms: activated,
            period_start_unix_ms: start,
            period_end_unix_ms: None,
            expected_revision,
        })?;
        unified_cap(store, AI_GENERAL_CAP_CREDENTIAL_ID)?.ok_or(ort_storage::StorageError::NotFound)
    });
    match result {
        Ok(value) => CommandResponse::success(value),
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
pub fn disable_ai_general_cap(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state
        .with_store(|store| store.disable_ai_cap(AI_GENERAL_CAP_CREDENTIAL_ID, AiPeriod::AllTime))
    {
        Ok(()) => CommandResponse::success(true),
        Err(ort_storage::StorageError::RevisionConflict) => {
            CommandResponse::failure("AI_BUSY", "errors.aiBusy", true)
        }
        Err(_) => storage_unavailable(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn reset_ai_general_cap(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        store.reset_ai_cap(
            AI_GENERAL_CAP_CREDENTIAL_ID,
            AiPeriod::AllTime,
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
pub fn disable_ai_cap(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: AiCapActionRequest,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        let (_, credential) = cap_context(store, request.credential_id)?;
        if request.period != AiPeriod::AllTime {
            return Err(ort_storage::StorageError::InvalidData);
        }
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
        let (_, credential) = cap_context(store, request.credential_id)?;
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
    credential_id: Option<Uuid>,
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
        store.ai_monitoring_summary_for_key(
            from_unix_ms,
            to_unix_ms,
            &time_zone,
            bucket_size,
            credential_id,
        )
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
    months: Vec<AiActivityMonth>,
    credential_ids: Option<Vec<Uuid>>,
) -> CommandResponse<u64> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(months) = normalize_activity_months(months) else {
        return CommandResponse::failure("AI_PERIOD_INVALID", "errors.aiPeriodInvalid", false);
    };
    let Ok(credential_ids) = normalize_credential_ids(credential_ids) else {
        return CommandResponse::failure(
            "AI_KEY_SELECTION_INVALID",
            "errors.aiPeriodInvalid",
            false,
        );
    };
    let ranges = months
        .iter()
        .map(|month| (month.from_unix_ms, month.to_unix_ms))
        .collect::<Vec<_>>();
    match state.with_store(|store| {
        store.clear_ai_activity_ranges_for_keys(&ranges, credential_ids.as_deref())
    }) {
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
    months: Vec<AiActivityMonth>,
    time_zone: String,
    credential_ids: Option<Vec<Uuid>>,
) -> CommandResponse<String> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(months) = normalize_activity_months(months) else {
        return CommandResponse::failure("AI_PERIOD_INVALID", "errors.aiPeriodInvalid", false);
    };
    let Ok(credential_ids) = normalize_credential_ids(credential_ids) else {
        return CommandResponse::failure(
            "AI_KEY_SELECTION_INVALID",
            "errors.aiPeriodInvalid",
            false,
        );
    };
    let app = window.app_handle().clone();
    let state = app.state::<DesktopState>();
    let exports = app.state::<ExportState>();
    let Some(lease) = exports.begin() else {
        return CommandResponse::failure("EXPORT_BUSY", "errors.aiExport", true);
    };
    let Ok(bytes) = state.with_store(|store| {
        let month_values = months
            .iter()
            .map(|month| {
                let summary = store.ai_monitoring_summary_for_keys(
                    month.from_unix_ms,
                    month.to_unix_ms,
                    &time_zone,
                    AiBucketSize::Day,
                    credential_ids.as_deref(),
                )?;
                Ok(json!({ "month": month.label, "summary": summary }))
            })
            .collect::<Result<Vec<_>, ort_storage::StorageError>>()?;
        serde_json::to_vec(&json!({
            "schemaVersion": 2,
            "selectedCredentialIds": credential_ids,
            "timeZone": time_zone,
            "selectedMonths": months.iter().map(|month| &month.label).collect::<Vec<_>>(),
            "months": month_values,
        }))
        .map_err(|_| ort_storage::StorageError::InvalidData)
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
