use ort_domain::{
    CONTRACT_VERSION, CommandResponse, HealthRequest, HealthResponse, HealthStatus, RuntimeProfile,
    StorageStatus, validate_health_request,
};
use tauri::WebviewWindow;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri command arguments are deserialized/injected by value.
fn health(window: WebviewWindow, request: HealthRequest) -> CommandResponse<HealthResponse> {
    if !matches!(window.label(), "main" | "overlay") {
        return CommandResponse::failure(
            "WINDOW_NOT_AUTHORIZED",
            "errors.windowNotAuthorized",
            false,
        );
    }

    if let Err(error) = validate_health_request(&request) {
        return CommandResponse::Failure { ok: false, error };
    }

    CommandResponse::success(HealthResponse {
        status: HealthStatus::Ok,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        profile: RuntimeProfile::Development,
        storage_status: StorageStatus::DevelopmentGated,
        contract_version: CONTRACT_VERSION,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the M0 desktop shell.
///
/// # Panics
///
/// Panics when Tauri cannot initialize the application runtime. At this stage
/// there is no user data to recover and continuing with a partial runtime would
/// be unsafe.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![health])
        .run(tauri::generate_context!())
        .expect("failed to run Open Resume Toolkit development shell");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_is_bound_to_the_development_profile() {
        let response = HealthResponse {
            status: HealthStatus::Ok,
            app_version: "0.0.0-dev".to_owned(),
            profile: RuntimeProfile::Development,
            storage_status: StorageStatus::DevelopmentGated,
            contract_version: CONTRACT_VERSION,
        };

        assert_eq!(response.profile, RuntimeProfile::Development);
        assert_eq!(response.storage_status, StorageStatus::DevelopmentGated);
        assert_eq!(response.contract_version, 2);
    }
}
