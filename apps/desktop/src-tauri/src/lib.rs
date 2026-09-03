use ort_domain::{
    CONTRACT_VERSION, CloseDecision, CloseStatusRequest, CloseStatusResponse, CommandResponse,
    HealthRequest, HealthResponse, HealthStatus, LoadResumeRequest, PublishResumeRequest,
    PublishResumeResponse, ResolveCloseRequest, ResumeWorkspaceResponse, RuntimeProfile,
    SaveResumeRequest, StorageStatus, StorageUsageRequest, StorageUsageResponse,
    VersionedResumeResponse, validate_health_request,
};
use ort_storage::{EncryptedStore, StorageError, VersionedResume};
use ort_vault::OsDatabaseKeyVault;
use tauri::{
    AppHandle, Emitter, EventTarget, Manager, RunEvent, State, WebviewWindow, WindowEvent,
};

mod backup_export;
mod close_guard;
mod menu;
mod pdf_preview;
mod text_export;
use close_guard::CloseGuard;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn close_status(
    window: WebviewWindow,
    state: State<'_, CloseGuard>,
    request: CloseStatusRequest,
) -> CommandResponse<CloseStatusResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    match state.status(window.label()) {
        Ok(value) => CommandResponse::success(value),
        Err(code) => CommandResponse::failure(code, "errors.closeUnavailable", true),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn resolve_close(
    window: WebviewWindow,
    state: State<'_, CloseGuard>,
    exports: State<'_, text_export::ExportState>,
    request: ResolveCloseRequest,
) -> CommandResponse<CloseStatusResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    if request.payload.decision == CloseDecision::Quit && exports.is_active() {
        return CommandResponse::failure("EXPORT_BUSY", "errors.closeUnavailable", true);
    }
    if let Err(code) = state.resolve(
        window.label(),
        &request.payload.attempt,
        request.payload.decision,
    ) {
        return CommandResponse::failure(code, "errors.closeUnavailable", true);
    }
    if request.payload.decision == CloseDecision::Quit {
        window.app_handle().exit(0);
    }
    CommandResponse::success(CloseStatusResponse {
        pending_attempt: None,
    })
}

fn request_native_close(app: &AppHandle) {
    let guard = app.state::<CloseGuard>();
    if guard.request().is_err() {
        return;
    } // Poisoned state never authorizes exit.
    if let Some(main) = app.get_webview_window("main") {
        // Quit from the overlay/Dock must surface the editor's confirmation.
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
        // The event is only a wakeup. The renderer fetches native state through
        // close_status; it never trusts an event's payload as exit authority.
        let _ = app.emit_to(
            EventTarget::webview_window("main"),
            "ort:close-requested",
            (),
        );
    }
}

enum DesktopStorage {
    Ready(EncryptedStore),
    Unavailable,
}

struct DesktopState {
    storage: DesktopStorage,
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn health(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: HealthRequest,
) -> CommandResponse<HealthResponse> {
    if !matches!(window.label(), "main" | "overlay") {
        return window_not_authorized();
    }
    if let Err(error) = validate_health_request(&request) {
        return CommandResponse::Failure { ok: false, error };
    }

    let storage_status = match state.storage {
        DesktopStorage::Ready(_) => StorageStatus::Ready,
        DesktopStorage::Unavailable => StorageStatus::Unavailable,
    };
    CommandResponse::success(HealthResponse {
        status: HealthStatus::Ok,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        profile: RuntimeProfile::Development,
        storage_status,
        contract_version: CONTRACT_VERSION,
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn load_resume(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: LoadResumeRequest,
) -> CommandResponse<ResumeWorkspaceResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let DesktopStorage::Ready(store) = &state.storage else {
        return storage_unavailable();
    };

    let draft = match store.load_draft() {
        Ok(value) => value.map(versioned_response),
        Err(error) => return storage_failure(&error),
    };
    let latest_published = match store.load_latest_published() {
        Ok(value) => value.map(versioned_response),
        Err(error) => return storage_failure(&error),
    };

    CommandResponse::success(ResumeWorkspaceResponse {
        draft,
        latest_published,
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn load_storage_usage(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: StorageUsageRequest,
) -> CommandResponse<StorageUsageResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let DesktopStorage::Ready(store) = &state.storage else {
        return storage_unavailable();
    };
    match store.storage_usage() {
        Ok(usage) => CommandResponse::success(StorageUsageResponse {
            database_schema: usage.database_schema,
            drafts: usage.drafts,
            published_snapshots: usage.published_snapshots,
            settings: usage.settings,
            render_manifests: usage.render_manifests,
            diagnostic_events: usage.diagnostic_events,
            database_bytes: usage.database_bytes,
            wal_bytes: usage.wal_bytes,
            shared_memory_bytes: usage.shared_memory_bytes,
            manifest_bytes: usage.manifest_bytes,
            recovery_metadata_bytes: usage.recovery_metadata_bytes,
            total_profile_bytes: usage.total_profile_bytes,
        }),
        Err(error) => storage_failure(&error),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn save_resume(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: SaveResumeRequest,
) -> CommandResponse<VersionedResumeResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let DesktopStorage::Ready(store) = &state.storage else {
        return storage_unavailable();
    };

    let saved = match request.payload.expected_revision {
        Some(revision) => store.save_draft(revision, &request.payload.document),
        None => store.create_draft(&request.payload.document),
    };
    match saved {
        Ok(value) => CommandResponse::success(versioned_response(value)),
        Err(error) => storage_failure(&error),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn publish_resume(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    request: PublishResumeRequest,
) -> CommandResponse<PublishResumeResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let DesktopStorage::Ready(store) = &state.storage else {
        return storage_unavailable();
    };

    let draft_revision = request.payload.expected_draft_revision;
    match store.publish_draft(draft_revision) {
        Ok(published) => CommandResponse::success(PublishResumeResponse {
            draft_revision,
            published: versioned_response(published),
        }),
        Err(error) => storage_failure(&error),
    }
}

fn versioned_response(value: VersionedResume) -> VersionedResumeResponse {
    VersionedResumeResponse {
        revision: value.revision,
        document: value.document,
    }
}

fn window_not_authorized<T: serde::Serialize>() -> CommandResponse<T> {
    CommandResponse::failure("WINDOW_NOT_AUTHORIZED", "errors.windowNotAuthorized", false)
}

fn storage_unavailable<T: serde::Serialize>() -> CommandResponse<T> {
    CommandResponse::failure("STORAGE_UNAVAILABLE", "errors.storageUnavailable", true)
}

fn storage_failure<T: serde::Serialize>(error: &StorageError) -> CommandResponse<T> {
    match error {
        StorageError::RevisionConflict => {
            CommandResponse::failure("REVISION_CONFLICT", "errors.revisionConflict", true)
        }
        StorageError::InvalidData => {
            CommandResponse::failure("INVALID_RESUME", "errors.invalidResume", false)
        }
        StorageError::NotFound => {
            CommandResponse::failure("DRAFT_NOT_FOUND", "errors.draftNotFound", false)
        }
        _ => storage_unavailable(),
    }
}

fn initialize_storage(app: &tauri::App) -> DesktopStorage {
    let Ok(app_data) = app.path().app_data_dir() else {
        return DesktopStorage::Unavailable;
    };
    let profile_root = app_data.join("profiles").join("default");
    let vault = OsDatabaseKeyVault::new();
    match EncryptedStore::open_or_activate_pending_restore(&profile_root, "dev", &vault) {
        Ok((store, _activated_restore)) => DesktopStorage::Ready(store),
        Err(_) => DesktopStorage::Unavailable,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the isolated development desktop application.
///
/// # Panics
/// Panics when Tauri cannot initialize the application runtime. Storage
/// initialization itself fails closed and leaves the UI available for recovery.
pub fn run() {
    tauri::Builder::default()
        .manage(CloseGuard::default())
        .manage(text_export::ExportState::default())
        .manage(pdf_preview::PdfState::default())
        .plugin(tauri_plugin_dialog::init())
        .menu(menu::editor_menu)
        .on_menu_event(|app, event| {
            if event.id().as_ref() == menu::QUIT_ID {
                request_native_close(app);
            }
        })
        .setup(|app| {
            app.manage(DesktopState {
                storage: initialize_storage(app),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health,
            load_resume,
            load_storage_usage,
            save_resume,
            publish_resume,
            backup_export::export_portable_backup,
            backup_export::validate_portable_backup,
            backup_export::restore_portable_backup,
            backup_export::load_backup_recovery_status,
            backup_export::rollback_safety_copy,
            backup_export::delete_safety_copy,
            text_export::export_resume_text,
            text_export::export_resume_docx,
            pdf_preview::render_resume_pdf,
            pdf_preview::load_pdf_render_history,
            pdf_preview::export_resume_pdf,
            pdf_preview::release_resume_pdf,
            close_status,
            resolve_close
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Open Resume Toolkit development shell")
        .run(|app, event| match event {
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } if label == "main" => {
                api.prevent_close();
                request_native_close(app);
            }
            RunEvent::ExitRequested { api, .. } if !app.state::<CloseGuard>().approved() => {
                api.prevent_exit();
                request_native_close(app);
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_contract_can_report_ready_encrypted_storage() {
        let response = HealthResponse {
            status: HealthStatus::Ok,
            app_version: "0.0.0-dev".to_owned(),
            profile: RuntimeProfile::Development,
            storage_status: StorageStatus::Ready,
            contract_version: CONTRACT_VERSION,
        };

        assert_eq!(response.storage_status, StorageStatus::Ready);
        assert_eq!(response.contract_version, 2);
    }

    #[test]
    fn storage_errors_map_to_stable_non_sensitive_codes() {
        let response: CommandResponse<VersionedResumeResponse> =
            storage_failure(&StorageError::RevisionConflict);
        let CommandResponse::Failure { error, .. } = response else {
            panic!("expected failure");
        };
        assert_eq!(error.code, "REVISION_CONFLICT");
        assert!(error.retryable);
        assert!(error.details.is_empty());
    }
}
