use ort_domain::{
    CONTRACT_VERSION, CloseDecision, CloseStatusRequest, CloseStatusResponse, CommandResponse,
    HealthRequest, HealthResponse, HealthStatus, LoadResumeRequest, PublishResumeRequest,
    PublishResumeResponse, ResolveCloseRequest, ResumeWorkspaceResponse, RuntimeProfile,
    SaveResumeRequest, StorageStatus, StorageUsageRequest, StorageUsageResponse,
    VersionedResumeResponse, validate_health_request,
};
use ort_storage::{EncryptedStore, StorageError, VersionedResume};
use ort_vault::OsDatabaseKeyVault;
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, PhysicalSize, RunEvent, State,
    WebviewWindow, WindowEvent,
};

mod ai_keys;
mod ai_request;
mod ai_settings;
mod application_materials;
mod backup_export;
mod close_guard;
mod data_deletion;
mod import_review;
mod menu;
mod pdf_preview;
mod text_export;
mod tracker;
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
async fn resolve_close(
    window: WebviewWindow,
    request: ResolveCloseRequest,
) -> CommandResponse<CloseStatusResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let main_app = app.clone();
    if app
        .run_on_main_thread(move || {
            let response = resolve_close_on_main(&main_app, &request);
            let _ = send.send(response);
        })
        .is_err()
    {
        return CommandResponse::failure("CLOSE_UNAVAILABLE", "errors.closeUnavailable", true);
    }
    match tauri::async_runtime::spawn_blocking(move || receive.recv()).await {
        Ok(Ok(response)) => response,
        _ => CommandResponse::failure("CLOSE_UNAVAILABLE", "errors.closeUnavailable", true),
    }
}

fn resolve_close_on_main(
    app: &AppHandle,
    request: &ResolveCloseRequest,
) -> CommandResponse<CloseStatusResponse> {
    let state = app.state::<CloseGuard>();
    let exports = app.state::<text_export::ExportState>();
    let quitting = request.payload.decision == CloseDecision::Quit;
    if let Err(code) = resolve_close_decision(
        &state,
        &exports,
        &request.payload.attempt,
        request.payload.decision,
    ) {
        return CommandResponse::failure(code, "errors.closeUnavailable", true);
    }
    #[cfg(target_os = "macos")]
    let native_reply = ort_macos_lifecycle::reply(quitting);
    #[cfg(not(target_os = "macos"))]
    let native_reply = false;
    if quitting && !native_reply {
        app.exit(0);
    }
    CommandResponse::success(CloseStatusResponse {
        pending_attempt: None,
    })
}

fn resolve_close_decision(
    guard: &CloseGuard,
    operations: &text_export::ExportState,
    attempt: &str,
    decision: CloseDecision,
) -> Result<(), &'static str> {
    let quitting = decision == CloseDecision::Quit;
    if quitting && !operations.seal_for_quit() {
        return Err("EXPORT_BUSY");
    }
    if let Err(code) = guard.resolve("main", attempt, decision) {
        if quitting {
            operations.undo_quit_seal();
        }
        return Err(code);
    }
    Ok(())
}

fn request_native_close(app: &AppHandle) -> bool {
    let guard = app.state::<CloseGuard>();
    if guard.request().is_err() {
        return false;
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
        return true;
    }
    false
}

enum DesktopStorage {
    Ready(EncryptedStore),
    Unavailable,
}

struct DesktopState {
    storage: Mutex<DesktopStorage>,
    reviews: std::sync::Arc<import_review::ReviewState>,
}

impl DesktopState {
    fn with_store<T>(
        &self,
        operation: impl FnOnce(&EncryptedStore) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let storage = self.storage.lock().map_err(|_| StorageError::Unavailable)?;
        let DesktopStorage::Ready(store) = &*storage else {
            return Err(StorageError::Unavailable);
        };
        operation(store)
    }

    fn storage_status(&self) -> StorageStatus {
        self.storage
            .lock()
            .ok()
            .map_or(StorageStatus::Unavailable, |storage| match &*storage {
                DesktopStorage::Ready(_) => StorageStatus::Ready,
                DesktopStorage::Unavailable => StorageStatus::Unavailable,
            })
    }

    fn take_store(&self) -> Result<EncryptedStore, StorageError> {
        self.reviews.clear()?;
        let mut storage = self.storage.lock().map_err(|_| StorageError::Unavailable)?;
        match std::mem::replace(&mut *storage, DesktopStorage::Unavailable) {
            DesktopStorage::Ready(store) => Ok(store),
            DesktopStorage::Unavailable => Err(StorageError::Unavailable),
        }
    }

    fn replace_storage(&self, replacement: DesktopStorage) -> Result<(), StorageError> {
        let mut storage = self.storage.lock().map_err(|_| StorageError::Unavailable)?;
        *storage = replacement;
        Ok(())
    }
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

    let storage_status = state.storage_status();
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
    let (draft, latest_published) = match state.with_store(|store| {
        Ok((
            store.load_draft()?.map(versioned_response),
            store.load_latest_published()?.map(versioned_response),
        ))
    }) {
        Ok(value) => value,
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
    match state.with_store(EncryptedStore::storage_usage) {
        Ok(usage) => CommandResponse::success(StorageUsageResponse {
            database_schema: usage.database_schema,
            drafts: usage.drafts,
            published_snapshots: usage.published_snapshots,
            settings: usage.settings,
            tracker_entries: usage.tracker_entries,
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
    let saved = state.with_store(|store| match request.payload.expected_revision {
        Some(revision) => store.save_draft(revision, &request.payload.document),
        None => store.create_draft(&request.payload.document),
    });
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
    let draft_revision = request.payload.expected_draft_revision;
    match state.with_store(|store| store.publish_draft(draft_revision)) {
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

fn initialize_storage(app: &AppHandle) -> DesktopStorage {
    // This composition root only implements the development channel. Reject a
    // packaging override before resolving paths or accessing any vault item.
    if !development_identity_allowed(&app.config().identifier) {
        return DesktopStorage::Unavailable;
    }
    let Ok(app_data) = app.path().app_data_dir() else {
        return DesktopStorage::Unavailable;
    };
    let profile_root = app_data.join("profiles").join("default");
    let vault = OsDatabaseKeyVault::new();
    match EncryptedStore::open_or_activate_pending_restore(&profile_root, "dev", &vault) {
        Ok((store, _activated_restore)) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|duration| i64::try_from(duration.as_millis()).ok());
            if now.is_some_and(|now| store.recover_ai_attempts(now).is_ok()) {
                DesktopStorage::Ready(store)
            } else {
                DesktopStorage::Unavailable
            }
        }
        Err(_) => DesktopStorage::Unavailable,
    }
}

fn development_identity_allowed(identifier: &str) -> bool {
    identifier == "com.openresumetoolkit.dev"
}

#[tauri::command]
fn application_overlay_visibility(window: WebviewWindow) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(overlay) = window.get_webview_window("overlay") else {
        return CommandResponse::failure("OVERLAY_UNAVAILABLE", "errors.overlayUnavailable", true);
    };
    match overlay.is_visible().and_then(|visible| {
        overlay
            .is_minimized()
            .map(|minimized| visible && !minimized)
    }) {
        Ok(visible) => CommandResponse::success(visible),
        Err(_) => {
            CommandResponse::failure("OVERLAY_UNAVAILABLE", "errors.overlayUnavailable", true)
        }
    }
}

#[tauri::command]
fn toggle_application_overlay(window: WebviewWindow) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(overlay) = window.get_webview_window("overlay") else {
        return CommandResponse::failure("OVERLAY_UNAVAILABLE", "errors.overlayUnavailable", true);
    };
    if overlay.is_visible().unwrap_or(false) && !overlay.is_minimized().unwrap_or(false) {
        hide_application_popup_for(window.app_handle());
        if overlay.hide().is_err() {
            return CommandResponse::failure(
                "OVERLAY_UNAVAILABLE",
                "errors.overlayUnavailable",
                true,
            );
        }
        let _ = window.emit("ort:overlay-visibility", false);
        return CommandResponse::success(false);
    }
    if position_overlay(&overlay).is_err() {
        return CommandResponse::failure("OVERLAY_UNAVAILABLE", "errors.overlayUnavailable", true);
    }
    if overlay.show().is_err() || overlay.unminimize().is_err() {
        return CommandResponse::failure("OVERLAY_UNAVAILABLE", "errors.overlayUnavailable", true);
    }
    if overlay.set_focus().is_err() {
        let _ = overlay.hide();
        return CommandResponse::failure("OVERLAY_UNAVAILABLE", "errors.overlayUnavailable", true);
    }
    let _ = window.emit("ort:overlay-visibility", true);
    CommandResponse::success(true)
}

#[tauri::command]
fn retry_storage(window: WebviewWindow, state: State<'_, DesktopState>) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Ok(mut storage) = state.storage.lock() else {
        return storage_unavailable();
    };
    if matches!(*storage, DesktopStorage::Ready(_)) {
        return CommandResponse::success(true);
    }
    *storage = initialize_storage(window.app_handle());
    let ready = matches!(*storage, DesktopStorage::Ready(_));
    CommandResponse::success(ready)
}

const OVERLAY_LOGICAL_WIDTH: f64 = 360.0;
const OVERLAY_LOGICAL_HEIGHT: f64 = 760.0;
const POPUP_GAP_PHYSICAL: i32 = 12;

fn position_overlay(overlay: &WebviewWindow) -> tauri::Result<()> {
    let scale = overlay.scale_factor()?;
    let Some(monitor) = overlay.current_monitor()?.or(overlay.primary_monitor()?) else {
        return Ok(());
    };
    let work = monitor.work_area();
    let width = (OVERLAY_LOGICAL_WIDTH * scale).round() as u32;
    let desired_height = (OVERLAY_LOGICAL_HEIGHT * scale).round() as u32;
    let height = desired_height.min(work.size.height);
    let x = work.position.x;
    let y = work.position.y + (i64::from(work.size.height) - i64::from(height)).max(0) as i32 / 2;
    overlay.set_size(PhysicalSize::new(width.min(work.size.width), height))?;
    overlay.set_position(PhysicalPosition::new(x, y))
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ApplicationPopupKind {
    Job,
    Url,
    ResumeView,
    ResumeEdit,
    Cover,
}

impl ApplicationPopupKind {
    fn logical_size(self) -> (f64, f64) {
        match self {
            Self::ResumeView | Self::ResumeEdit => (850.0, 760.0),
            Self::Job | Self::Url | Self::Cover => (520.0, 420.0),
        }
    }
}

fn hide_application_popup_for(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window("application-popup") {
        let _ = popup.hide();
    }
}

#[tauri::command]
fn show_application_popup(
    window: WebviewWindow,
    kind: ApplicationPopupKind,
) -> CommandResponse<bool> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let Some(popup) = window.get_webview_window("application-popup") else {
        return CommandResponse::failure("POPUP_UNAVAILABLE", "errors.overlayUnavailable", true);
    };
    let Ok(overlay_position) = window.outer_position() else {
        return CommandResponse::failure("POPUP_UNAVAILABLE", "errors.overlayUnavailable", true);
    };
    let Ok(overlay_size) = window.outer_size() else {
        return CommandResponse::failure("POPUP_UNAVAILABLE", "errors.overlayUnavailable", true);
    };
    let Ok(scale) = window.scale_factor() else {
        return CommandResponse::failure("POPUP_UNAVAILABLE", "errors.overlayUnavailable", true);
    };
    let Ok(Some(monitor)) = window
        .current_monitor()
        .or_else(|_| window.primary_monitor())
    else {
        return CommandResponse::failure("POPUP_UNAVAILABLE", "errors.overlayUnavailable", true);
    };
    let work = monitor.work_area();
    let (logical_width, logical_height) = kind.logical_size();
    let width = ((logical_width * scale).round() as u32).min(work.size.width);
    let height = ((logical_height * scale).round() as u32).min(work.size.height);
    let right = overlay_position
        .x
        .saturating_add(overlay_size.width as i32)
        .saturating_add(POPUP_GAP_PHYSICAL);
    let left = overlay_position
        .x
        .saturating_sub(width as i32)
        .saturating_sub(POPUP_GAP_PHYSICAL);
    let work_right = work.position.x.saturating_add(work.size.width as i32);
    let preferred_x = if right.saturating_add(width as i32) <= work_right {
        right
    } else {
        left
    };
    let x = preferred_x.clamp(work.position.x, work_right.saturating_sub(width as i32));
    let work_bottom = work.position.y.saturating_add(work.size.height as i32);
    let y = overlay_position
        .y
        .clamp(work.position.y, work_bottom.saturating_sub(height as i32));
    if popup.set_size(PhysicalSize::new(width, height)).is_err()
        || popup.set_position(PhysicalPosition::new(x, y)).is_err()
        || popup.show().is_err()
        || popup.unminimize().is_err()
        || popup.set_focus().is_err()
    {
        return CommandResponse::failure("POPUP_UNAVAILABLE", "errors.overlayUnavailable", true);
    }
    CommandResponse::success(true)
}

#[tauri::command]
fn hide_application_popup(window: WebviewWindow) -> CommandResponse<bool> {
    if !matches!(window.label(), "overlay" | "application-popup") {
        return window_not_authorized();
    }
    hide_application_popup_for(&window.app_handle());
    CommandResponse::success(true)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the isolated development desktop application.
///
/// # Panics
/// Panics when Tauri cannot initialize the application runtime. Storage
/// initialization itself fails closed and leaves the UI available for recovery.
#[allow(clippy::too_many_lines)]
pub fn run() {
    tauri::Builder::default()
        .manage(CloseGuard::default())
        .manage(text_export::ExportState::default())
        .manage(pdf_preview::PdfState::default())
        .manage(pdf_preview::PortablePdfState::default())
        .manage(ai_request::AiRequestGate::default())
        .manage(application_materials::DragFiles::default())
        .manage(application_materials::ApplicationExportState::default())
        .plugin(tauri_plugin_dialog::init())
        .menu(menu::editor_menu)
        .on_menu_event(|app, event| {
            if event.id().as_ref() == menu::QUIT_ID {
                request_native_close(app);
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                application_materials::DragFiles::sweep_stale();
                let handle = app.handle().clone();
                if !ort_macos_lifecycle::install(move || request_native_close(&handle)) {
                    return Err(
                        std::io::Error::other("Native termination guard is unavailable").into(),
                    );
                }
            }
            app.manage(DesktopState {
                storage: Mutex::new(initialize_storage(app.handle())),
                reviews: std::sync::Arc::default(),
            });
            import_review::ReviewState::start_expiry(&app.state::<DesktopState>().reviews)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ai_keys::load_ai_connection,
            ai_keys::add_ai_key,
            ai_keys::change_ai_key,
            ai_keys::clear_ai_primary,
            ai_keys::delete_removed_ai_key_data,
            ai_keys::rename_ai_key,
            ai_keys::set_ai_key_preset,
            ai_settings::load_ai_catalog,
            application_materials::load_application_workspace,
            application_materials::load_application_stage_one,
            application_materials::save_application_stage_one,
            application_materials::load_application_capture,
            application_materials::request_application_capture,
            application_materials::resolve_application_capture,
            application_materials::application_context,
            application_materials::cancel_application_generation,
            application_materials::start_application,
            application_materials::regenerate_application_resume,
            application_materials::generate_application_cover_letter,
            application_materials::generate_application_answer,
            application_materials::refine_application_answer,
            application_materials::save_application_workspace,
            application_materials::finish_application,
            application_materials::preview_application_pdf,
            application_materials::prepare_application_exports,
            application_materials::download_application_export,
            application_materials::drag_application_export,
            application_materials::download_application_pdf,
            application_materials::drag_application_pdf,
            tracker::list_tracker_entries,
            tracker::save_tracker_entry,
            tracker::delete_tracker_entry,
            tracker::open_tracker_link,
            tracker::preview_tracker_pdf,
            ai_request::test_ai_connection,
            ai_request::preview_ai_test,
            ai_request::cancel_ai_test,
            ai_settings::load_ai_monitoring,
            ai_settings::load_ai_retention,
            ai_settings::save_ai_retention,
            ai_settings::load_ai_caps,
            ai_settings::load_ai_key_settings,
            ai_settings::load_ai_general_settings,
            ai_settings::save_ai_cap,
            ai_settings::save_ai_general_cap,
            ai_settings::disable_ai_cap,
            ai_settings::disable_ai_general_cap,
            ai_settings::reset_ai_cap,
            ai_settings::reset_ai_general_cap,
            ai_settings::clear_ai_monitoring,
            ai_settings::export_ai_monitoring,
            import_review::begin::begin_document_import,
            import_review::begin::cancel_document_import,
            import_review::begin::document_import_available,
            import_review::read_import_review,
            import_review::apply_import_review,
            import_review::cancel_import_review,
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
            data_deletion::delete_all_local_data,
            text_export::export_resume_text,
            text_export::export_resume_docx,
            pdf_preview::render_resume_pdf,
            pdf_preview::replay_resume_pdf,
            pdf_preview::regenerate_resume_pdf,
            pdf_preview::open_portable_pdf_render_history,
            pdf_preview::replay_portable_resume_pdf,
            pdf_preview::regenerate_portable_resume_pdf,
            pdf_preview::release_portable_pdf_archive,
            pdf_preview::load_pdf_render_history,
            pdf_preview::export_resume_pdf,
            pdf_preview::release_resume_pdf,
            close_status,
            application_overlay_visibility,
            toggle_application_overlay,
            retry_storage,
            show_application_popup,
            hide_application_popup,
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
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } if label == "overlay" => {
                api.prevent_close();
                request_native_close(app);
            }
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::Focused(false),
                ..
            } if label == "overlay" => {
                if let Some(overlay) = app.get_webview_window("overlay") {
                    if overlay.is_minimized().unwrap_or(false) {
                        hide_application_popup_for(app);
                        let _ = app.emit_to(
                            EventTarget::webview_window("main"),
                            "ort:overlay-visibility",
                            false,
                        );
                    }
                }
            }
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::Focused(true),
                ..
            } if label == "main" => {
                if let Some(overlay) = app.get_webview_window("overlay") {
                    let visible = overlay.is_visible().unwrap_or(false)
                        && !overlay.is_minimized().unwrap_or(false);
                    let _ = app.emit_to(
                        EventTarget::webview_window("main"),
                        "ort:overlay-visibility",
                        visible,
                    );
                }
            }
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } if label == "application-popup" => {
                api.prevent_close();
                hide_application_popup_for(app);
            }
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::Focused(false),
                ..
            } if label == "application-popup" => {
                hide_application_popup_for(app);
            }
            RunEvent::ExitRequested { api, .. } if !app.state::<CloseGuard>().approved() => {
                api.prevent_exit();
                request_native_close(app);
            }
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::Destroyed,
                ..
            } if label == "main" => {
                let _ = app.state::<DesktopState>().reviews.clear();
            }
            RunEvent::Exit => {
                let _ = app.state::<DesktopState>().reviews.clear();
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_decision_waits_for_operations_and_stale_decisions_do_not_seal() {
        let guard = CloseGuard::default();
        let operations = text_export::ExportState::default();
        guard.request().expect("request");
        let attempt = guard
            .status("main")
            .expect("status")
            .pending_attempt
            .expect("attempt");
        let lease = operations.begin().expect("operation");
        assert_eq!(
            resolve_close_decision(&guard, &operations, &attempt, CloseDecision::Quit),
            Err("EXPORT_BUSY")
        );
        assert!(!guard.approved());
        assert_eq!(
            guard
                .status("main")
                .expect("status")
                .pending_attempt
                .as_deref(),
            Some(attempt.as_str())
        );
        drop(lease);
        assert_eq!(
            resolve_close_decision(&guard, &operations, "stale", CloseDecision::Quit),
            Err("STALE_CLOSE_ATTEMPT")
        );
        drop(
            operations
                .begin()
                .expect("stale decision released its seal"),
        );
        resolve_close_decision(&guard, &operations, &attempt, CloseDecision::Quit)
            .expect("approve");
        assert!(guard.approved());
        assert!(operations.begin().is_none());
        assert!(
            resolve_close_decision(&guard, &operations, &attempt, CloseDecision::Quit).is_err()
        );
        assert!(
            operations.begin().is_none(),
            "replayed approval must not unseal"
        );
    }

    #[test]
    fn cancel_can_resolve_while_an_operation_remains_active() {
        let guard = CloseGuard::default();
        let operations = text_export::ExportState::default();
        guard.request().expect("request");
        let attempt = guard
            .status("main")
            .expect("status")
            .pending_attempt
            .expect("attempt");
        let lease = operations.begin().expect("operation");
        resolve_close_decision(&guard, &operations, &attempt, CloseDecision::Cancel)
            .expect("cancel");
        assert!(!guard.approved());
        assert!(
            guard
                .status("main")
                .expect("status")
                .pending_attempt
                .is_none()
        );
        assert!(operations.begin().is_none());
        drop(lease);
        assert!(operations.begin().is_some());
    }

    #[test]
    fn development_storage_rejects_other_package_identities() {
        assert!(development_identity_allowed("com.openresumetoolkit.dev"));
        for identifier in [
            "com.openresumetoolkit",
            "com.openresumetoolkit.preview",
            "com.openresumetoolkit.test",
            "com.openresumetoolkit.dev.other",
            "",
        ] {
            assert!(!development_identity_allowed(identifier));
        }
    }

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
