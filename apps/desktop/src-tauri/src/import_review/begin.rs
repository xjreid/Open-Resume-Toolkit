use super::{ReviewState, failure, import_available};
use crate::{DesktopState, text_export::ExportState, window_not_authorized};
use ort_domain::{
    BeginImportRequest, CommandResponse, ImportReviewSnapshot, VersionedResumeResponse,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

struct JobLease {
    state: Arc<ReviewState>,
    cancel: Arc<AtomicBool>,
}
impl JobLease {
    fn begin(state: &Arc<ReviewState>) -> Option<Self> {
        let mut current = state.job.lock().ok()?;
        if current.is_some() {
            return None;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *current = Some(Arc::clone(&cancel));
        Some(Self {
            state: Arc::clone(state),
            cancel,
        })
    }
}
impl Drop for JobLease {
    fn drop(&mut self) {
        if let Ok(mut current) = self.state.job.lock()
            && current
                .as_ref()
                .is_some_and(|value| Arc::ptr_eq(value, &self.cancel))
        {
            *current = None;
        }
    }
}

// Tauri injects an owned window handle into synchronous commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub(crate) fn document_import_available(window: WebviewWindow) -> bool {
    import_available(window.label())
}
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub(crate) fn cancel_document_import(window: WebviewWindow) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let state = window.state::<DesktopState>();
    let Ok(job) = state.reviews.job.lock() else {
        return failure("IMPORT_REVIEW_UNAVAILABLE");
    };
    if let Some(cancel) = job.as_ref() {
        cancel.store(true, Ordering::Release);
    }
    CommandResponse::success(true)
}
#[tauri::command]
pub(crate) async fn begin_document_import(
    window: WebviewWindow,
    request: BeginImportRequest,
) -> CommandResponse<Option<ImportReviewSnapshot>> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if !document_import_available(window.clone()) {
        return failure("IMPORT_DISABLED");
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(operation) = app.state::<ExportState>().begin() else {
        return failure("LOCAL_DATA_OPERATION_BUSY");
    };
    let state = app.state::<DesktopState>();
    let Some(job) = JobLease::begin(&state.reviews) else {
        return failure("LOCAL_DATA_OPERATION_BUSY");
    };
    let base = match state.with_store(|store| {
        match (store.load_draft()?, request.payload.expected_revision) {
            (Some(saved), Some(expected)) if saved.revision == expected => {
                Ok(VersionedResumeResponse {
                    revision: saved.revision,
                    document: saved.document,
                })
            }
            (None, None) => Ok(VersionedResumeResponse {
                revision: 0,
                document: ort_domain::ResumeDocument::empty("My Resume"),
            }),
            _ => Err(ort_storage::StorageError::RevisionConflict),
        }
    }) {
        Ok(base) => base,
        Err(error) => return crate::storage_failure(&error),
    };
    match tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let Ok(mut sessions) = job.state.sessions.lock() else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        if sessions.is_active(Instant::now()) {
            return failure("LOCAL_DATA_OPERATION_BUSY");
        }
        drop(sessions);
        let selection = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose a PDF or DOCX resume to review")
            .add_filter("Resume documents", &["pdf", "docx"])
            .blocking_pick_file();
        if job.cancel.load(Ordering::Acquire) {
            return failure("IMPORT_CANCELLED");
        }
        let Some(selection) = selection else {
            return CommandResponse::success(None);
        };
        let FilePath::Path(path) = selection else {
            return failure("IMPORT_INVALID_SOURCE");
        };
        let Ok(source) = ort_platform::read_native_document(&path) else {
            return failure("IMPORT_INVALID_SOURCE");
        };
        let Ok(extraction) = extract(&source, &job.cancel) else {
            return failure("IMPORT_FAILED");
        };
        let Ok(mut sessions) = job.state.sessions.lock() else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        if job.cancel.load(Ordering::Acquire) {
            return failure("IMPORT_CANCELLED");
        }
        let proposal = ort_documents::import::ImportProposal::map(extraction);
        let Ok(token) = sessions.begin(job.state.owner, base, proposal, Instant::now()) else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        match sessions.read(job.state.owner, token, Instant::now()) {
            Ok(review) => CommandResponse::success(Some(review.snapshot(token.identifier()))),
            Err(_) => failure("IMPORT_REVIEW_UNAVAILABLE"),
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => failure("IMPORT_REVIEW_UNAVAILABLE"),
    }
}
#[cfg(target_os = "macos")]
fn extract(
    source: &ort_platform::NativeDocumentSource,
    cancel: &AtomicBool,
) -> Result<ort_documents::import::ValidatedExtraction, ()> {
    let hash = option_env!("ORT_PARSER_HELPER_SHA256").ok_or(())?;
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(());
    }
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hash[index * 2..index * 2 + 2], 16).map_err(|_| ())?;
    }
    let executable = std::env::current_exe().map_err(|_| ())?;
    let contents = executable
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or(())?;
    let helper = contents.join("Helpers/ORT Parser Helper.app/Contents/MacOS/ort-parser-helper");
    let format = match source.format {
        ort_platform::NativeDocumentFormat::Pdf => ort_documents::import::InputFormat::Pdf,
        ort_platform::NativeDocumentFormat::Docx => ort_documents::import::InputFormat::Docx,
    };
    ort_platform::ParserHelper::new(
        helper,
        digest,
        option_env!("ORT_PARSER_HELPER_CDHASH")
            .ok_or(())?
            .to_owned(),
    )
    .map_err(|_| ())?
    .extract(format, &source.bytes, cancel)
    .map_err(|_| ())
}
#[cfg(not(target_os = "macos"))]
fn extract(
    _: &ort_platform::NativeDocumentSource,
    _: &AtomicBool,
) -> Result<ort_documents::import::ValidatedExtraction, ()> {
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn job_slot_is_exclusive_and_teardown_cancels_before_reuse() {
        let state = Arc::new(ReviewState::default());
        let first = JobLease::begin(&state).unwrap();
        assert!(JobLease::begin(&state).is_none());
        state.clear().unwrap();
        assert!(first.cancel.load(Ordering::Acquire));
        assert!(JobLease::begin(&state).is_none());
        drop(first);
        let next = JobLease::begin(&state).unwrap();
        assert!(!next.cancel.load(Ordering::Acquire));
    }
}
