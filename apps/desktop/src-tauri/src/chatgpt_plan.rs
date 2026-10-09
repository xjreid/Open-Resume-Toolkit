//! ORT-owned authorization boundary and persistent, non-secret plan policy.
use crate::plan_runtime::SessionAccess;
use crate::{
    DesktopState,
    ai_request::AiRequestGate,
    codex_runtime::{self, Session},
};
use ort_ai::plan::{PlanModel, PlanSettings, QuotaSnapshot, ReasoningEffort};
use ort_domain::CommandResponse;
use ort_storage::{EncryptedStore, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, WebviewWindow};
use uuid::Uuid;

pub(crate) const SETTING: &str = "ai.chatgpt_plan.v1";
pub(crate) use crate::plan_runtime::PlanRuntime;

fn settings_for_session(
    previous: &PlanSettings,
    connection_id: Uuid,
    models: &[PlanModel],
) -> PlanSettings {
    let mut next = previous.clone();
    next.connection_id = Some(connection_id);
    if next.model.is_none()
        && models.iter().any(|m| {
            m.id == "gpt-6.1-sol"
                && m.supported
                && m.reasoning_efforts.contains(&ReasoningEffort::Medium)
        })
    {
        next.model = Some("gpt-6.1-sol".into());
        next.reasoning = ReasoningEffort::Medium;
    }
    next
}
pub(crate) fn load_settings(
    store: &EncryptedStore,
) -> Result<(PlanSettings, Option<i64>), StorageError> {
    let Some(saved) = store.load_setting(SETTING)? else {
        return Ok((PlanSettings::default(), None));
    };
    let settings: PlanSettings =
        serde_json::from_value(saved.value).map_err(|_| StorageError::InvalidData)?;
    if !settings.valid() {
        return Err(StorageError::InvalidData);
    }
    Ok((settings, Some(saved.revision)))
}
pub(crate) fn auth_root(window: &WebviewWindow, profile: Uuid) -> Result<PathBuf, &'static str> {
    let root = window
        .app_handle()
        .path()
        .app_data_dir()
        .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
    std::fs::create_dir_all(&root).map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
    let root = root
        .canonicalize()
        .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
    let managed = root.join("codex-managed");
    for path in [&managed, &managed.join(profile.to_string())] {
        if path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink() || !m.is_dir())
        {
            return Err("PLAN_RUNTIME_UNTRUSTED");
        }
        std::fs::create_dir_all(path).map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        }
    }
    Ok(managed.join(profile.to_string()))
}
fn with_session<T>(
    window: &WebviewWindow,
    profile: Uuid,
    f: impl FnOnce(&mut Session) -> Result<T, &'static str>,
) -> Result<T, &'static str> {
    with_session_until(
        window,
        profile,
        Instant::now() + Duration::from_secs(60),
        &|| false,
        f,
    )
}
fn with_session_until<T>(
    window: &WebviewWindow,
    profile: Uuid,
    end: Instant,
    cancel: &dyn Fn() -> bool,
    f: impl FnOnce(&mut Session) -> Result<T, &'static str>,
) -> Result<T, &'static str> {
    window
        .state::<PlanRuntime>()
        .with_session(
            profile,
            end,
            SessionAccess {
                cancel,
                skip_busy: false,
                permit: &|| runtime_permission(window, profile),
            },
            |cancel| Session::start(&auth_root(window, profile)?, end, cancel),
            f,
        )
        .and_then(|value| value.ok_or("AI_BUSY"))
}

fn runtime_permission(window: &WebviewWindow, profile: Uuid) -> Result<(), &'static str> {
    window
        .state::<DesktopState>()
        .with_store(|store| {
            ort_application::application_workspace::ensure_profile(store, profile)?;
            load_settings(store).map(|v| v.0)
        })
        .map_err(|_| "STORAGE_UNAVAILABLE")?
        .runtime_permission()
}

pub(crate) fn connected(
    session: &mut Session,
    cancel: &dyn Fn() -> bool,
) -> Result<Option<String>, &'static str> {
    let value = session.rpc(
        "account/read",
        json!({"refreshToken":false}),
        Duration::from_secs(10),
        cancel,
    )?;
    let account = value.get("account").filter(|v| !v.is_null());
    if account
        .and_then(|a| a.get("type"))
        .and_then(serde_json::Value::as_str)
        != Some("chatgpt")
    {
        session.connection_id = None;
        session.account_plan = None;
        session.quota = None;
        return Ok(None);
    }
    session.account_plan = Some(
        account
            .and_then(|a| a.get("planType"))
            .and_then(serde_json::Value::as_str)
            .filter(|s| {
                matches!(
                    *s,
                    "plus" | "pro" | "team" | "business" | "enterprise" | "edu" | "free"
                )
            })
            .unwrap_or("ChatGPT")
            .to_owned(),
    );
    Ok(session.account_plan.clone())
}
#[derive(Clone, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanStatus {
    pub settings: PlanSettings,
    pub revision: Option<i64>,
    pub connected: bool,
    pub account_plan: Option<String>,
    pub login_pending: bool,
    pub operation_active: bool,
    pub runtime_version: Option<String>,
    pub models: Vec<PlanModel>,
    pub quota: Option<QuotaSnapshot>,
    pub error_code: Option<String>,
}
fn status(window: &WebviewWindow, refresh_usage: bool) -> Result<PlanStatus, &'static str> {
    let state = window.state::<DesktopState>();
    let (mut settings, revision, profile) = state
        .with_store(|store| {
            let (settings, revision) = load_settings(store)?;
            Ok((settings, revision, store.manifest().profile_id))
        })
        .map_err(|_| "STORAGE_UNAVAILABLE")?;
    let busy = window.state::<AiRequestGate>().is_busy();
    let manager = window.state::<PlanRuntime>();
    let mut value = manager.status(profile, settings.clone(), revision, busy);
    if busy || !settings.enabled || settings.cleanup_required {
        return Ok(value);
    }
    let end = Instant::now() + Duration::from_secs(30);
    let result = manager.with_session(
        profile,
        end,
        SessionAccess {
            cancel: &|| false,
            skip_busy: true,
            permit: &|| runtime_permission(window, profile),
        },
        |cancel| Session::start(&auth_root(window, profile)?, end, cancel),
        |session| {
            session.poll_login()?;
            value.login_pending = session.login_id.is_some();
            value.models.clone_from(&session.models);
            value.runtime_version = Some(codex_runtime::qualified_version().into());
            value.error_code = session.login_error.map(str::to_owned);
            value.account_plan = connected(session, &|| manager.interrupted())?;
            value.connected = value.account_plan.is_some() && !settings.cleanup_required;
            if settings.cleanup_required {
                value.error_code = Some("PLAN_CREDENTIAL_CLEANUP_REQUIRED".into());
            }
            if value.connected
                && (session.connection_id.is_none()
                    || session.connection_id != settings.connection_id)
            {
                let identity = *session.connection_id.get_or_insert_with(Uuid::now_v7);
                settings = settings_for_session(&settings, identity, &value.models);
                let saved = state
                    .with_store(|store| {
                        ort_application::application_workspace::ensure_profile(store, profile)?;
                        store.save_setting(SETTING, revision, &json!(settings))
                    })
                    .map_err(|_| "REVISION_CONFLICT")?;
                value.revision = Some(saved.revision);
                value.settings = settings.clone();
                let _ = window.app_handle().emit("ort:ai-model-changed", ());
            }
            if !value.connected
                && !value.login_pending
                && value.error_code.is_none()
                && settings.connection_id.is_some()
            {
                value.error_code = Some("PLAN_AUTH_REQUIRED".into());
            }
            if refresh_usage && value.connected {
                session.refresh_quota(Instant::now() + Duration::from_secs(10), &|| {
                    manager.interrupted()
                })?;
            }
            value.quota.clone_from(&session.quota);
            Ok(())
        },
    );
    if let Err(code) = result {
        value = manager.status(profile, value.settings, value.revision, busy);
        value.error_code = Some(code.into());
    }
    let (settings, revision) = state
        .with_store(|store| {
            ort_application::application_workspace::ensure_profile(store, profile)?;
            load_settings(store)
        })
        .map_err(|_| "REVISION_CONFLICT")?;
    value.settings = settings;
    value.revision = revision;
    value.operation_active = window.state::<AiRequestGate>().is_busy();
    if !value.settings.enabled || value.settings.cleanup_required {
        value = manager.status(
            profile,
            value.settings,
            value.revision,
            value.operation_active,
        );
    }
    Ok(value)
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavePlanRequest {
    pub expected_revision: Option<i64>,
    pub enabled: bool,
    pub model: Option<String>,
    pub reasoning: ReasoningEffort,
    pub reserve_enabled: bool,
    pub reserve_percent: u8,
}
impl SavePlanRequest {
    // The overlay owns model/reasoning selection, while connection and reserve
    // controls remain in the main window.
    fn overlay_selection_only(&self, old: &PlanSettings) -> bool {
        old.enabled
            && self.enabled == old.enabled
            && self.reserve_enabled == old.reserve_enabled
            && self.reserve_percent == old.reserve_percent
    }
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadPlanRequest {
    #[serde(default)]
    pub refresh_usage: bool,
}

fn response<T: Serialize>(result: Result<T, &'static str>) -> CommandResponse<T> {
    match result {
        Ok(value) => CommandResponse::success(value),
        Err(code) => CommandResponse::failure(code, "errors.chatgptPlan", true),
    }
}
#[tauri::command]
pub async fn load_chatgpt_plan(
    window: WebviewWindow,
    request: LoadPlanRequest,
) -> CommandResponse<PlanStatus> {
    if !matches!(window.label(), "main" | "overlay") {
        return crate::window_not_authorized();
    }
    response(
        tauri::async_runtime::spawn_blocking(move || status(&window, request.refresh_usage))
            .await
            .unwrap_or(Err("PLAN_RUNTIME_UNAVAILABLE")),
    )
}
#[tauri::command]
pub async fn connect_chatgpt_plan(window: WebviewWindow) -> CommandResponse<PlanStatus> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    response(
        tauri::async_runtime::spawn_blocking(move || {
            let gate = window.state::<AiRequestGate>();
            let lease = gate.begin(Uuid::now_v7()).ok_or("AI_BUSY")?;
            let profile = window
                .state::<DesktopState>()
                .with_store(|s| Ok(s.manifest().profile_id))
                .map_err(|_| "STORAGE_UNAVAILABLE")?;
            if window
                .state::<DesktopState>()
                .with_store(|s| load_settings(s).map(|v| v.0.cleanup_required))
                .map_err(|_| "STORAGE_UNAVAILABLE")?
            {
                return Err("PLAN_CREDENTIAL_CLEANUP_REQUIRED");
            }
            with_session(&window, profile, |session| {
                if session.login_id.is_some() {
                    return Err("PLAN_LOGIN_PENDING");
                }
                let login = session.rpc(
                    "account/login/start",
                    json!({"type":"chatgpt"}),
                    Duration::from_secs(10),
                    &|| false,
                )?;
                let url = login
                    .get("authUrl")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|u| url::Url::parse(u).ok())
                    .filter(|u| {
                        u.scheme() == "https"
                            && u.host_str() == Some("auth.openai.com")
                            && u.username().is_empty()
                            && u.password().is_none()
                    })
                    .ok_or("PLAN_PROTOCOL_INVALID")?;
                session.login_id = Some(
                    login
                        .get("loginId")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("PLAN_PROTOCOL_INVALID")?
                        .to_owned(),
                );
                session.login_started = Some(Instant::now());
                session.login_error = None;
                session.connection_id = None;
                let opened = std::process::Command::new("/usr/bin/open")
                    .arg(url.as_str())
                    .status()
                    .is_ok_and(|s| s.success());
                if !opened {
                    session.cancel_login()?;
                    return Err("PLAN_LOGIN_BROWSER_FAILED");
                }
                Ok(())
            })?;
            drop(lease);
            status(&window, false)
        })
        .await
        .unwrap_or(Err("PLAN_RUNTIME_UNAVAILABLE")),
    )
}
#[tauri::command]
pub async fn cancel_chatgpt_login(window: WebviewWindow) -> CommandResponse<PlanStatus> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    response(
        tauri::async_runtime::spawn_blocking(move || {
            let profile = window
                .state::<DesktopState>()
                .with_store(|s| Ok(s.manifest().profile_id))
                .map_err(|_| "STORAGE_UNAVAILABLE")?;
            with_session(&window, profile, Session::cancel_login)?;
            status(&window, false)
        })
        .await
        .unwrap_or(Err("PLAN_RUNTIME_UNAVAILABLE")),
    )
}
#[tauri::command]
pub async fn save_chatgpt_plan(
    window: WebviewWindow,
    request: SavePlanRequest,
) -> CommandResponse<PlanStatus> {
    if !matches!(window.label(), "main" | "overlay") {
        return crate::window_not_authorized();
    }
    response(
        tauri::async_runtime::spawn_blocking(move || {
            let gate = window.state::<AiRequestGate>();
            let state = window.state::<DesktopState>();
            let (old, revision, profile) = state
                .with_store(|s| {
                    let (v, r) = load_settings(s)?;
                    Ok((v, r, s.manifest().profile_id))
                })
                .map_err(|_| "STORAGE_UNAVAILABLE")?;
            if revision != request.expected_revision {
                return Err("REVISION_CONFLICT");
            }
            if window.label() == "overlay" && !request.overlay_selection_only(&old) {
                return Err("WINDOW_NOT_AUTHORIZED");
            }
            let disabling = old.enabled && !request.enabled;
            let lease = gate.begin(Uuid::now_v7());
            if lease.is_none() && !disabling {
                return Err("AI_BUSY");
            }
            let next = PlanSettings {
                cleanup_required: old.cleanup_required || disabling,
                connection_id: if disabling { None } else { old.connection_id },
                enabled: request.enabled,
                model: request.model,
                reasoning: request.reasoning,
                reserve_enabled: request.reserve_enabled,
                reserve_percent: request.reserve_percent,
            };
            if !next.valid() {
                return Err("PLAN_SETTINGS_INVALID");
            }
            if next.enabled && (next.model != old.model || next.reasoning != old.reasoning) {
                window
                    .state::<PlanRuntime>()
                    .validate_selection(profile, &next)?;
            }
            state
                .with_store(|store| {
                    ort_application::application_workspace::ensure_profile(store, profile)?;
                    persist_selection(store, revision, &next)
                })
                .map_err(|_| "REVISION_CONFLICT")?;
            if disabling {
                clean_up_session(&window, profile)?;
            }
            drop(lease);
            let _ = window.app_handle().emit("ort:ai-model-changed", ());
            let (settings, revision) = state
                .with_store(|store| {
                    ort_application::application_workspace::ensure_profile(store, profile)?;
                    load_settings(store)
                })
                .map_err(|_| "STORAGE_UNAVAILABLE")?;
            Ok(window
                .state::<PlanRuntime>()
                .status(profile, settings, revision, false))
        })
        .await
        .unwrap_or(Err("PLAN_RUNTIME_UNAVAILABLE")),
    )
}
pub(crate) fn persist_selection(
    store: &EncryptedStore,
    revision: Option<i64>,
    next: &PlanSettings,
) -> Result<(), StorageError> {
    if !next.valid() {
        return Err(StorageError::InvalidData);
    }
    store.save_setting(SETTING, revision, &json!(next))?;
    Ok(())
}
#[tauri::command]
pub async fn disconnect_chatgpt_plan(window: WebviewWindow) -> CommandResponse<PlanStatus> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    if window
        .state::<DesktopState>()
        .with_store(|s| load_settings(s).map(|v| v.0.enabled))
        .unwrap_or(false)
    {
        window.state::<AiRequestGate>().cancel_overlay();
    }
    response(
        tauri::async_runtime::spawn_blocking(move || {
            disconnect_profile(&window)?;
            let state = window.state::<DesktopState>();
            state
                .with_store(|store| {
                    let (settings, revision) = load_settings(store)?;
                    Ok(window.state::<PlanRuntime>().status(
                        store.manifest().profile_id,
                        settings,
                        revision,
                        false,
                    ))
                })
                .map_err(|_| "STORAGE_UNAVAILABLE")
        })
        .await
        .unwrap_or(Err("PLAN_RUNTIME_UNAVAILABLE")),
    )
}
pub(crate) fn disconnect_profile(window: &WebviewWindow) -> Result<(), &'static str> {
    let state = window.state::<DesktopState>();
    let (mut settings, revision, profile) = state
        .with_store(|s| {
            let (v, r) = load_settings(s)?;
            Ok((v, r, s.manifest().profile_id))
        })
        .map_err(|_| "STORAGE_UNAVAILABLE")?;
    // Retire authorization before cleanup while preserving runtime permission.
    settings.connection_id = None;
    settings.cleanup_required = true;
    state
        .with_store(|s| {
            ort_application::application_workspace::ensure_profile(s, profile)?;
            s.save_setting(SETTING, revision, &json!(settings))
        })
        .map_err(|_| "REVISION_CONFLICT")?;
    clean_up_session(window, profile)
}

fn clean_up_session(window: &WebviewWindow, profile: Uuid) -> Result<(), &'static str> {
    let state = window.state::<DesktopState>();
    window.state::<AiRequestGate>().cancel_overlay();
    window.state::<PlanRuntime>().disconnect(profile);
    let _ = window.app_handle().emit("ort:ai-model-changed", ());
    let root = auth_root(window, profile)?;
    std::fs::remove_dir_all(root).map_err(|_| "PLAN_CREDENTIAL_CLEANUP_REQUIRED")?;
    state
        .with_store(|store| {
            ort_application::application_workspace::ensure_profile(store, profile)?;
            let (mut cleaned, rev) = load_settings(store)?;
            cleaned.cleanup_required = false;
            store.save_setting(SETTING, rev, &json!(cleaned))
        })
        .map_err(|_| "REVISION_CONFLICT")?;
    let _ = window.app_handle().emit("ort:ai-model-changed", ());
    Ok(())
}
pub(crate) fn with_operation_session<T>(
    window: &WebviewWindow,
    profile: Uuid,
    end: Instant,
    cancel: &dyn Fn() -> bool,
    f: impl FnOnce(&mut Session) -> Result<T, &'static str>,
) -> Result<T, &'static str> {
    with_session_until(window, profile, end, cancel, f)
}
pub(crate) fn authorization_ready(
    session: &mut Session,
    deadline: Duration,
    cancel: &dyn Fn() -> bool,
) -> Result<(), &'static str> {
    let account = session.rpc(
        "account/read",
        json!({"refreshToken":true}),
        deadline.min(Duration::from_secs(10)),
        cancel,
    )?;
    if account
        .pointer("/account/type")
        .and_then(serde_json::Value::as_str)
        == Some("chatgpt")
    {
        Ok(())
    } else {
        Err("PLAN_AUTH_REQUIRED")
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn stop_chatgpt_plan(window: WebviewWindow) -> CommandResponse<bool> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    let enabled = window
        .state::<DesktopState>()
        .with_store(|s| load_settings(s).map(|v| v.0.enabled))
        .unwrap_or(false);
    CommandResponse::success(enabled && window.state::<AiRequestGate>().cancel_overlay())
}

/// The renderer carries no external endpoint; this fixed documentation target
/// opens only after the user selects installation guidance on the main page.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn open_chatgpt_plan_runtime_guidance(window: WebviewWindow) -> CommandResponse<bool> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    let url = format!(
        "https://github.com/openai/codex/releases/tag/rust-v{}",
        codex_runtime::qualified_version()
    );
    response(
        std::process::Command::new("/usr/bin/open")
            .arg(url)
            .status()
            .map_err(|_| "PLAN_RUNTIME_GUIDANCE_FAILED")
            .and_then(|status| {
                if status.success() {
                    Ok(true)
                } else {
                    Err("PLAN_RUNTIME_GUIDANCE_FAILED")
                }
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlay_can_only_change_model_and_reasoning_for_enabled_codex() {
        let mut settings = PlanSettings {
            enabled: true,
            connection_id: Some(Uuid::now_v7()),
            model: Some("gpt-6.1-sol".into()),
            ..PlanSettings::default()
        };
        let mut request = SavePlanRequest {
            expected_revision: Some(1),
            enabled: true,
            model: Some("gpt-6-sol".into()),
            reasoning: ReasoningEffort::High,
            reserve_enabled: settings.reserve_enabled,
            reserve_percent: settings.reserve_percent,
        };
        assert!(request.overlay_selection_only(&settings));
        request.enabled = false;
        assert!(!request.overlay_selection_only(&settings));
        request.enabled = true;
        request.reserve_percent += 1;
        assert!(!request.overlay_selection_only(&settings));
        request.reserve_percent = settings.reserve_percent;
        request.reserve_enabled = false;
        assert!(!request.overlay_selection_only(&settings));
        request.reserve_enabled = settings.reserve_enabled;
        settings.enabled = false;
        request.enabled = false;
        assert!(!request.overlay_selection_only(&settings));
    }
    #[test]
    fn first_connection_is_disabled_with_twenty_percent_reserve() {
        let s = PlanSettings::default();
        assert!(!s.enabled);
        assert!(s.reserve_enabled);
        assert_eq!(s.reserve_percent, 20);
    }

    #[test]
    fn reconnect_gets_a_new_identity_and_preserves_enabled_preference() {
        let previous = PlanSettings {
            connection_id: Some(Uuid::now_v7()),
            enabled: true,
            model: Some("gpt-6-sol".into()),
            reasoning: ReasoningEffort::High,
            reserve_enabled: false,
            reserve_percent: 35,
            ..Default::default()
        };
        let identity = Uuid::now_v7();
        let next = settings_for_session(&previous, identity, &[]);
        assert_eq!(next.connection_id, Some(identity));
        assert_ne!(next.connection_id, previous.connection_id);
        assert!(next.enabled);
        assert_eq!(next.model, previous.model);
        assert_eq!(next.reasoning, previous.reasoning);
        assert_eq!(next.reserve_enabled, previous.reserve_enabled);
        assert_eq!(next.reserve_percent, previous.reserve_percent);
        assert!(next.valid());
    }

    #[test]
    fn a_first_session_uses_the_supported_default_only_when_no_model_is_saved() {
        let models = ort_ai::plan::models_from_catalog(&json!({"data":[{
            "model":"gpt-6.1-sol", "hidden":false,
            "supportedReasoningEfforts":[{"reasoningEffort":"medium"}]
        }]}));
        let next = settings_for_session(&PlanSettings::default(), Uuid::now_v7(), &models);
        assert_eq!(next.model.as_deref(), Some("gpt-6.1-sol"));
        assert_eq!(next.reasoning, ReasoningEffort::Medium);
        assert!(!next.enabled);
        assert!(
            settings_for_session(&PlanSettings::default(), Uuid::now_v7(), &[])
                .model
                .is_none()
        );
    }
}
