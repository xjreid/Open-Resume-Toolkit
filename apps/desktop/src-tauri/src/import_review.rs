pub(crate) mod begin;
use super::{DesktopState, text_export::ExportState, window_not_authorized};
use ort_application::import_session::{CommitError, ReviewOwner, ReviewSessions, ReviewToken};
use ort_domain::{
    ApplyImportReviewRequest, CommandResponse, ImportChoices, ImportReviewRequest,
    VersionedResumeResponse,
};
use ort_storage::{EncryptedStore, StorageError};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};

#[derive(Default)]
pub(crate) struct ReviewState {
    sessions: Mutex<ReviewSessions>,
    owner: ReviewOwner,
    job: Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,
}
impl ReviewState {
    pub(crate) fn clear(&self) -> Result<(), StorageError> {
        if let Some(cancel) = self
            .job
            .lock()
            .map_err(|_| StorageError::Unavailable)?
            .as_ref()
        {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        }
        self.sessions
            .lock()
            .map_err(|_| StorageError::Unavailable)?
            .close_owner(self.owner);
        Ok(())
    }
    pub(crate) fn start_expiry(state: &Arc<Self>) -> std::io::Result<()> {
        let weak = Arc::downgrade(state);
        std::thread::Builder::new()
            .name("ort-review-expiry".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(5));
                    let Some(state) = weak.upgrade() else {
                        break;
                    };
                    if let Ok(mut sessions) = state.sessions.lock() {
                        sessions.expire(Instant::now());
                    }
                }
            })?;
        Ok(())
    }
    fn apply(
        &self,
        store: &EncryptedStore,
        identifier: &str,
        bytes: &[u8],
    ) -> CommandResponse<VersionedResumeResponse> {
        let Some(token) = ReviewToken::parse(identifier) else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        let Ok(choices) = ImportChoices::decode(bytes) else {
            return failure("IMPORT_REVIEW_INVALID");
        };
        let rejected = choices
            .choices
            .iter()
            .filter(|choice| matches!(choice, ort_domain::ImportChoice::Reject {}))
            .count();
        let accepted = choices.choices.len() - rejected;
        let (Ok(accepted), Ok(rejected)) = (u16::try_from(accepted), u16::try_from(rejected))
        else {
            return failure("IMPORT_REVIEW_INVALID");
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        let now = Instant::now();
        if sessions.read(self.owner, token, now).is_err() {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        }
        let current = match store.load_draft() {
            Ok(Some(value)) => VersionedResumeResponse {
                revision: value.revision,
                document: value.document,
            },
            Ok(None) => match sessions
                .read(self.owner, token, Instant::now())
                .ok()
                .and_then(ort_application::import_review::ImportReview::empty_base)
            {
                Some(base) => base,
                None => return failure("IMPORT_REVIEW_UNAVAILABLE"),
            },
            _ => return failure("IMPORT_REVIEW_UNAVAILABLE"),
        };
        if sessions
            .replace_choices(self.owner, token, Instant::now(), choices)
            .is_err()
        {
            return failure("IMPORT_REVIEW_INVALID");
        }
        match sessions.commit(self.owner, token, Instant::now(), &current, |payload| {
            store
                .save_imported_draft(
                    payload.expected_revision.unwrap_or(0),
                    &payload.document,
                    accepted,
                    rejected,
                )
                .map(|saved| VersionedResumeResponse {
                    revision: saved.revision,
                    document: saved.document,
                })
        }) {
            Ok(saved) => CommandResponse::success(saved),
            Err(CommitError::Storage(error)) => super::storage_failure(&error),
            Err(CommitError::Session(_)) => failure("IMPORT_REVIEW_INVALID"),
            Err(CommitError::UnexpectedReceipt) => failure("IMPORT_REVIEW_OUTCOME_UNKNOWN"),
        }
    }
}
fn failure<T: serde::Serialize>(code: &str) -> CommandResponse<T> {
    CommandResponse::failure(code, "errors.importReviewUnavailable", false)
}
fn import_available(label: &str) -> bool {
    // The legacy native-parser gate remains off. Only a package with both
    // pinned Wasm-helper identities can expose the new supervised import path.
    label == "main"
        && cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && option_env!("ORT_PARSER_HELPER_SHA256").is_some_and(|value| {
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        && option_env!("ORT_PARSER_HELPER_CDHASH").is_some_and(|value| {
            value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

#[tauri::command]
pub(crate) async fn read_import_review(
    window: WebviewWindow,
    request: ImportReviewRequest,
) -> CommandResponse<ort_domain::ImportReviewSnapshot> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if !import_available(window.label()) {
        return failure("IMPORT_DISABLED");
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    match tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<DesktopState>();
        let Some(token) = ReviewToken::parse(&request.payload.review_id) else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        let Ok(mut sessions) = state.reviews.sessions.lock() else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        match sessions.read(state.reviews.owner, token, Instant::now()) {
            Ok(review) => CommandResponse::success(review.snapshot(token.identifier())),
            Err(_) => failure("IMPORT_REVIEW_UNAVAILABLE"),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("IMPORT_REVIEW_UNAVAILABLE"),
    }
}

#[tauri::command]
pub(crate) async fn apply_import_review(
    window: WebviewWindow,
    request: ApplyImportReviewRequest,
) -> CommandResponse<VersionedResumeResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if !import_available(window.label()) {
        return failure("IMPORT_DISABLED");
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("LOCAL_DATA_OPERATION_BUSY");
    };
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let state = app.state::<DesktopState>();
        state
            .with_store(|store| {
                Ok(state.reviews.apply(
                    store,
                    &request.payload.review_id,
                    request.payload.decisions_json.as_bytes(),
                ))
            })
            .unwrap_or_else(|_| failure("IMPORT_REVIEW_UNAVAILABLE"))
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("IMPORT_REVIEW_OUTCOME_UNKNOWN"),
    }
}

#[tauri::command]
pub(crate) async fn cancel_import_review(
    window: WebviewWindow,
    request: ImportReviewRequest,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if !import_available(window.label()) {
        return failure("IMPORT_DISABLED");
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("LOCAL_DATA_OPERATION_BUSY");
    };
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let state = app.state::<DesktopState>();
        let Some(token) = ReviewToken::parse(&request.payload.review_id) else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        let Ok(mut sessions) = state.reviews.sessions.lock() else {
            return failure("IMPORT_REVIEW_UNAVAILABLE");
        };
        match sessions.cancel(state.reviews.owner, token, Instant::now()) {
            Ok(()) => CommandResponse::success(true),
            Err(_) => failure("IMPORT_REVIEW_UNAVAILABLE"),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("IMPORT_REVIEW_UNAVAILABLE"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_documents::import::{ImportProposal, InputFormat, ValidatedExtraction};
    use ort_domain::ResumeDocument;
    use ort_vault::testing::MemoryDatabaseKeyVault;
    use tempfile::TempDir;

    fn seed(state: &ReviewState, store: &EncryptedStore) -> ReviewToken {
        let draft = store.load_draft().unwrap().unwrap();
        let source = ValidatedExtraction::decode(br#"{"version":1,"format":"docx","pageCount":1,"blocks":[{"page":1,"kind":"paragraph","text":"Synthetic contribution"}]}"#, InputFormat::Docx).unwrap();
        state
            .sessions
            .lock()
            .unwrap()
            .begin(
                state.owner,
                VersionedResumeResponse {
                    revision: draft.revision,
                    document: draft.document,
                },
                ImportProposal::map(source),
                Instant::now(),
            )
            .unwrap()
    }
    const CHOICES: &[u8] = br#"{"choices":[{"kind":"text","text":"Synthetic contribution","bullet":true,"target":{"kind":"new","heading":"Projects"}}]}"#;
    #[test]
    fn native_apply_saves_once_and_invalid_input_preserves_review_and_storage() {
        let directory = TempDir::new().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(directory.path(), "test", &vault).unwrap();
        let original = store
            .create_draft(&ResumeDocument::empty("Synthetic native import"))
            .unwrap();
        let state = ReviewState::default();
        let token = seed(&state, &store);
        assert!(matches!(
            state.apply(&store, &token.identifier(), br#"{"choices":[]}"#),
            CommandResponse::Failure { .. }
        ));
        assert_eq!(
            store.load_draft().unwrap().unwrap().revision,
            original.revision
        );
        let result = state.apply(&store, &token.identifier(), CHOICES);
        assert!(matches!(result, CommandResponse::Success { .. }));
        assert_eq!(
            store.load_draft().unwrap().unwrap().revision,
            original.revision + 1
        );
        assert!(matches!(
            state.apply(&store, &token.identifier(), CHOICES),
            CommandResponse::Failure { .. }
        ));
    }
    #[test]
    fn clearing_native_review_drops_source_and_cannot_commit_after_storage_teardown() {
        let directory = TempDir::new().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(directory.path(), "test", &vault).unwrap();
        store
            .create_draft(&ResumeDocument::empty("Synthetic teardown"))
            .unwrap();
        let state = DesktopState {
            storage: Mutex::new(super::super::DesktopStorage::Ready(store)),
            reviews: std::sync::Arc::default(),
        };
        let token = state
            .with_store(|store| Ok(seed(&state.reviews, store)))
            .unwrap();
        let store = state.take_store().unwrap();
        assert!(matches!(
            state.reviews.apply(&store, &token.identifier(), CHOICES),
            CommandResponse::Failure { .. }
        ));
        assert_eq!(store.load_draft().unwrap().unwrap().revision, 1);
    }
    #[test]
    fn stale_or_expired_native_review_cannot_change_the_saved_draft() {
        let directory = TempDir::new().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(directory.path(), "test", &vault).unwrap();
        let original = store
            .create_draft(&ResumeDocument::empty("Synthetic stale review"))
            .unwrap();
        let state = ReviewState::default();
        let token = seed(&state, &store);
        let mut edited = original.document;
        edited.contact.full_name = "Later synthetic edit".into();
        store.save_draft(original.revision, &edited).unwrap();
        assert!(matches!(
            state.apply(&store, &token.identifier(), CHOICES),
            CommandResponse::Failure { .. }
        ));
        assert_eq!(store.load_draft().unwrap().unwrap().document, edited);
        state.clear().unwrap();
        let next = seed(&state, &store);
        state
            .sessions
            .lock()
            .unwrap()
            .expire(Instant::now() + ort_application::import_session::REVIEW_LIFETIME);
        assert!(matches!(
            state.apply(&store, &next.identifier(), CHOICES),
            CommandResponse::Failure { .. }
        ));
        assert_eq!(store.load_draft().unwrap().unwrap().document, edited);
    }

    #[test]
    fn parser_gate_refuses_non_main_windows_and_unpinned_builds() {
        for label in ["overlay", "unknown", ""] {
            assert!(!import_available(label));
        }
        if option_env!("ORT_PARSER_HELPER_SHA256").is_none()
            || option_env!("ORT_PARSER_HELPER_CDHASH").is_none()
        {
            assert!(!import_available("main"));
        }
    }
}
