use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ort_documents::{TEXT_FORMAT_VERSION, render_plain_text};
use ort_domain::{CommandResponse, ExportSource, ExportTextRequest, ExportTextResponse};
use ort_platform::{ExportDestination, ExportWriteError};
use ort_storage::{StorageError, VersionedResume};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use super::{
    DesktopState, DesktopStorage, storage_failure, storage_unavailable, window_not_authorized,
};

#[derive(Default)]
pub(crate) struct ExportState(Arc<AtomicBool>);

impl ExportState {
    pub(crate) fn is_active(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn begin(&self) -> Option<ExportLease> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| ExportLease(Arc::clone(&self.0)))
    }
}

struct ExportLease(Arc<AtomicBool>);
impl Drop for ExportLease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub(crate) async fn export_resume_text(
    window: WebviewWindow,
    request: ExportTextRequest,
) -> CommandResponse<ExportTextResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let state = app.state::<DesktopState>();
    let exports = app.state::<ExportState>();
    let Some(lease) = exports.begin() else {
        return export_failure("EXPORT_BUSY");
    };
    let DesktopStorage::Ready(store) = &state.storage else {
        return storage_unavailable();
    };
    let source = request.payload.source;
    let loaded = match source {
        ExportSource::SavedDraft => store.load_draft(),
        ExportSource::PublishedSnapshot => store.load_latest_published(),
    };
    let saved =
        match loaded.and_then(|value| exact_revision(value, request.payload.expected_revision)) {
            Ok(value) => value,
            Err(error) => return storage_failure(&error),
        };
    // The immutable saved content is captured before the dialog; editing while
    // it is open cannot substitute renderer text into this export.
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        export_with_dialog(&window, source, &saved)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => export_failure("EXPORT_OUTCOME_UNKNOWN"),
    }
}

fn exact_revision(
    value: Option<VersionedResume>,
    expected: i64,
) -> Result<VersionedResume, StorageError> {
    let value = value.ok_or(StorageError::NotFound)?;
    if value.revision != expected {
        return Err(StorageError::RevisionConflict);
    }
    Ok(value)
}

fn export_with_dialog(
    window: &WebviewWindow,
    source: ExportSource,
    saved: &VersionedResume,
) -> CommandResponse<ExportTextResponse> {
    let Ok(text) = render_plain_text(&saved.document) else {
        return export_failure("EXPORT_INVALID_CONTENT");
    };
    let selection = window
        .dialog()
        .file()
        .set_parent(window)
        .set_title("Export unencrypted text — choose a new filename")
        .set_file_name("resume.txt")
        .add_filter("Plain text", &["txt"])
        .blocking_save_file();
    let Some(selection) = selection else {
        return CommandResponse::success(ExportTextResponse::Cancelled);
    };
    let FilePath::Path(path) = selection else {
        return export_failure("EXPORT_INVALID_DESTINATION");
    };
    // No selected path, token, document text or OS error string is sent to JS.
    match ExportDestination::from_native_dialog(&path)
        .and_then(|destination| destination.write(text.as_bytes()))
    {
        Ok(receipt) => CommandResponse::success(ExportTextResponse::Exported {
            source,
            revision: saved.revision,
            byte_count: text.len(),
            format_version: TEXT_FORMAT_VERSION,
            cleanup_pending: receipt.cleanup_pending,
            durability_unconfirmed: receipt.durability_unconfirmed,
        }),
        Err(error) => export_failure(match error {
            ExportWriteError::AlreadyExists => "EXPORT_ALREADY_EXISTS",
            ExportWriteError::InvalidDestination => "EXPORT_INVALID_DESTINATION",
            ExportWriteError::InvalidContent => "EXPORT_INVALID_CONTENT",
            ExportWriteError::Unavailable => "EXPORT_UNAVAILABLE",
        }),
    }
}

fn export_failure(code: &str) -> CommandResponse<ExportTextResponse> {
    CommandResponse::failure(code, "errors.textExport", false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_domain::ResumeDocument;

    #[test]
    fn only_one_dialog_operation_runs_and_dropping_it_releases_the_gate() {
        let state = ExportState::default();
        let lease = state.begin().expect("first export");
        assert!(state.is_active());
        assert!(state.begin().is_none());
        drop(lease);
        assert!(!state.is_active());
        assert!(state.begin().is_some());
    }

    #[test]
    fn export_rejects_missing_and_stale_saved_revisions() {
        assert!(matches!(
            exact_revision(None, 1),
            Err(StorageError::NotFound)
        ));
        let saved = || VersionedResume {
            revision: 2,
            document: ResumeDocument::empty("Synthetic"),
        };
        assert!(matches!(
            exact_revision(Some(saved()), 1),
            Err(StorageError::RevisionConflict)
        ));
        assert_eq!(exact_revision(Some(saved()), 2).unwrap().revision, 2);
    }
}
