//! One bounded, memory-only preview. The webview cannot submit PDFs or paths.
use super::{
    DesktopState, DesktopStorage, storage_failure, storage_unavailable,
    text_export::{ExportState, exact_revision},
    window_not_authorized,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ort_domain::{
    CommandResponse, ExportSource, PDF_PREVIEW_TTL_SECONDS, PdfExportResponse, PdfPreviewResponse,
    PdfReleaseResponse, PdfTicketRequest, RenderPdfRequest,
};
use ort_platform::{ExportDestination, ExportFileType, ExportWriteError};
use ort_render::{PdfArtifact, PdfRenderError};
use ort_storage::{StorageError, VersionedResume};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

struct Preview {
    id: String,
    source: ExportSource,
    revision: i64,
    created: Instant,
    generated_at_unix_ms: u64,
    artifact: PdfArtifact,
}

#[derive(Default)]
pub(crate) struct PdfState(Mutex<Option<Arc<Preview>>>);

impl PdfState {
    fn get(&self, id: &str, now: Instant) -> Option<Arc<Preview>> {
        let mut slot = self.0.lock().ok()?;
        if slot.as_ref().is_some_and(|p| {
            now.saturating_duration_since(p.created) >= Duration::from_secs(PDF_PREVIEW_TTL_SECONDS)
        }) {
            *slot = None;
        }
        slot.as_ref().filter(|p| p.id == id).cloned()
    }

    fn release(&self, id: &str) -> bool {
        let Ok(mut slot) = self.0.lock() else {
            return false;
        };
        if slot.as_ref().is_some_and(|p| p.id == id) {
            *slot = None;
            true
        } else {
            false
        }
    }
}

fn load_exact(
    state: &DesktopState,
    source: ExportSource,
    revision: i64,
) -> Result<VersionedResume, StorageError> {
    let DesktopStorage::Ready(store) = &state.storage else {
        return Err(StorageError::NotFound);
    };
    let value = match source {
        ExportSource::SavedDraft => store.load_draft(),
        ExportSource::PublishedSnapshot => store.load_latest_published(),
    }?;
    exact_revision(value, revision)
}

#[tauri::command]
pub(crate) async fn render_resume_pdf(
    window: WebviewWindow,
    request: RenderPdfRequest,
) -> CommandResponse<PdfPreviewResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("EXPORT_BUSY");
    };
    let state = app.state::<DesktopState>();
    if !matches!(state.storage, DesktopStorage::Ready(_)) {
        return storage_unavailable();
    }
    let source = request.payload.source;
    let saved = match load_exact(&state, source, request.payload.expected_revision) {
        Ok(saved) => saved,
        Err(error) => return storage_failure(&error),
    };
    // Drop the previous native bytes before compiling another bounded preview.
    {
        let cache = app.state::<PdfState>();
        let Ok(mut slot) = cache.0.lock() else {
            return failure("PDF_UNAVAILABLE");
        };
        *slot = None;
    }
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let artifact = match ort_render::render_pdf(&saved.document) {
            Ok(value) => value,
            Err(error) => {
                return failure(match error {
                    PdfRenderError::UnsupportedGlyph => "PDF_UNSUPPORTED_GLYPH",
                    PdfRenderError::LayoutLimit => "PDF_LAYOUT_LIMIT",
                    PdfRenderError::OutputTooLarge => "PDF_BYTE_LIMIT",
                    PdfRenderError::InvalidContent => "PDF_INVALID_CONTENT",
                    PdfRenderError::Unavailable => "PDF_UNAVAILABLE",
                });
            }
        };
        let Some(generated_at_unix_ms) = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|v| u64::try_from(v.as_millis()).ok())
            .filter(|value| (1..=9_007_199_254_740_991).contains(value))
        else {
            return failure("PDF_UNAVAILABLE");
        };
        let preview = Arc::new(Preview {
            id: uuid::Uuid::now_v7().to_string(),
            source,
            revision: saved.revision,
            created: Instant::now(),
            generated_at_unix_ms,
            artifact,
        });
        let response = PdfPreviewResponse {
            render_id: preview.id.clone(),
            source,
            revision: saved.revision,
            generated_at_unix_ms: preview.generated_at_unix_ms,
            receipt: preview.artifact.receipt.clone(),
            pdf_base64: STANDARD.encode(&preview.artifact.bytes),
        };
        let cache = app.state::<PdfState>();
        let Ok(mut slot) = cache.0.lock() else {
            return failure("PDF_UNAVAILABLE");
        };
        *slot = Some(preview);
        CommandResponse::success(response)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("PDF_UNAVAILABLE"),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri command arguments are owned.
pub(crate) fn release_resume_pdf(
    window: WebviewWindow,
    request: PdfTicketRequest,
) -> CommandResponse<PdfReleaseResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    CommandResponse::success(PdfReleaseResponse {
        released: window
            .state::<PdfState>()
            .release(&request.payload.render_id),
    })
}

#[tauri::command]
pub(crate) async fn export_resume_pdf(
    window: WebviewWindow,
    request: PdfTicketRequest,
) -> CommandResponse<PdfExportResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("EXPORT_BUSY");
    };
    let Some(preview) = app
        .state::<PdfState>()
        .get(&request.payload.render_id, Instant::now())
    else {
        return failure("PDF_PREVIEW_EXPIRED");
    };
    if let Err(error) = load_exact(
        &app.state::<DesktopState>(),
        preview.source,
        preview.revision,
    ) {
        return storage_failure(&error);
    }
    // The exact preview is captured before the dialog. Later saves cannot alter it.
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let selection = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Export unencrypted PDF — choose a new filename")
            .set_file_name("resume.pdf")
            .add_filter("PDF document", &["pdf"])
            .blocking_save_file();
        let Some(selection) = selection else {
            return CommandResponse::success(PdfExportResponse::Cancelled);
        };
        let FilePath::Path(path) = selection else {
            return failure("EXPORT_INVALID_DESTINATION");
        };
        match ExportDestination::for_native_dialog(&path, ExportFileType::Pdf)
            .and_then(|target| target.write(&preview.artifact.bytes))
        {
            Ok(receipt) => CommandResponse::success(PdfExportResponse::Exported {
                render_id: preview.id.clone(),
                pdf_sha256: preview.artifact.receipt.pdf_sha256.clone(),
                byte_count: preview.artifact.bytes.len(),
                cleanup_pending: receipt.cleanup_pending,
                durability_unconfirmed: receipt.durability_unconfirmed,
            }),
            Err(error) => failure(match error {
                ExportWriteError::AlreadyExists => "EXPORT_ALREADY_EXISTS",
                ExportWriteError::InvalidDestination => "EXPORT_INVALID_DESTINATION",
                ExportWriteError::InvalidContent => "EXPORT_INVALID_CONTENT",
                ExportWriteError::Unavailable => "EXPORT_UNAVAILABLE",
            }),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("EXPORT_OUTCOME_UNKNOWN"),
    }
}

fn failure<T: serde::Serialize>(code: &str) -> CommandResponse<T> {
    CommandResponse::failure(code, "errors.pdf", false)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn preview() -> Arc<Preview> {
        let mut document = ort_domain::ResumeDocument::empty("Private title");
        document.contact.full_name = "Synthetic".into();
        Arc::new(Preview {
            id: uuid::Uuid::now_v7().to_string(),
            source: ExportSource::SavedDraft,
            revision: 1,
            created: Instant::now(),
            generated_at_unix_ms: 1,
            artifact: ort_render::render_pdf(&document).unwrap(),
        })
    }
    #[test]
    fn ticket_expiry_release_and_replacement_are_identity_bound() {
        let first = preview();
        let state = PdfState(Mutex::new(Some(first.clone())));
        assert!(Arc::ptr_eq(
            &state.get(&first.id, first.created).unwrap(),
            &first
        ));
        assert!(state.get("unknown", first.created).is_none());
        assert!(!state.release("unknown"));
        let second = preview();
        *state.0.lock().unwrap() = Some(second.clone());
        assert!(!state.release(&first.id));
        assert!(state.get(&first.id, first.created).is_none());
        assert!(
            state
                .get(
                    &second.id,
                    second.created + Duration::from_secs(PDF_PREVIEW_TTL_SECONDS)
                )
                .is_none()
        );
        assert!(state.0.lock().unwrap().is_none());
        *state.0.lock().unwrap() = Some(first.clone());
        assert!(state.release(&first.id));
        assert!(!state.release(&first.id));
    }
}
