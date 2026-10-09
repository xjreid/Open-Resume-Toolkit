//! Offline installation checks, independent of permission to run Codex.
use crate::codex_runtime;
use ort_domain::CommandResponse;
use serde::Serialize;
use tauri::WebviewWindow;

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeReadiness {
    pub ready: bool,
    pub error_code: Option<String>,
}
impl RuntimeReadiness {
    pub(crate) fn unavailable(code: &str) -> Self {
        Self {
            ready: false,
            error_code: Some(code.into()),
        }
    }
}

#[tauri::command]
pub async fn check_codex_runtime(window: WebviewWindow) -> CommandResponse<RuntimeReadiness> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    // Discovery verifies protected paths, permissions and pinned executable
    // bytes. It never executes Codex, reads credentials or creates a session.
    // Hashing runs off the UI thread; actual startup verifies again.
    match tauri::async_runtime::spawn_blocking(codex_runtime::discover).await {
        Ok(Ok(_)) => CommandResponse::success(RuntimeReadiness {
            ready: true,
            error_code: None,
        }),
        Ok(Err(code)) => CommandResponse::success(RuntimeReadiness::unavailable(code)),
        Err(_) => {
            CommandResponse::failure("PLAN_RUNTIME_CHECK_FAILED", "errors.codexInstall", true)
        }
    }
}
