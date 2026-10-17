//! Main-window PDF import. Native file selection freezes the provider; Codex
//! uses bounded page rendering. Only validated proposals reach the review editor.
use super::begin::JobLease;
use crate::{
    DesktopState,
    ai_request::{
        AiRequestGate, MaterialFailure,
        material_provider::{MaterialProvider, PassContext},
    },
    text_export::ExportState,
    window_not_authorized,
};
use ort_ai::{OperationType, Provider, builtin_catalog};
use ort_domain::{
    BeginImportRequest, CommandResponse, ErrorEnvelope, ImportReviewSnapshot,
    VersionedResumeResponse,
};
use serde::Serialize;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiImportStatus {
    ready: bool,
    busy: bool,
    provider: Option<String>,
    model: Option<String>,
    reasoning: Option<ort_ai::plan::ReasoningEffort>,
    issue: Option<String>,
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri injects an owned window.
pub(crate) fn ai_import_status(window: WebviewWindow) -> CommandResponse<AiImportStatus> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let result = window.state::<DesktopState>().with_store(|store| {
        let connection = crate::ai_keys::request_connection(store, None)?;
        let uses_plan = connection.mode == "chatgpt_plan";
        let provider = match connection.provider.as_deref() {
            Some("openai") => Some(Provider::OpenAi),
            Some("anthropic") => Some(Provider::Anthropic),
            Some("gemini") => Some(Provider::Gemini),
            _ => None,
        };
        let issue = if uses_plan {
            let settings = crate::chatgpt_plan::load_settings(store)?.0;
            window
                .state::<crate::chatgpt_plan::PlanRuntime>()
                .validate_image_selection(store.vault_identity().2, &settings)
                .err()
        } else if connection.mode != "direct_api" {
            Some("AI_DISABLED")
        } else if !provider
            .zip(connection.model.as_deref())
            .is_some_and(|(provider, model)| {
                builtin_catalog(&jiff::Timestamp::now().to_string(), None).is_ok_and(|catalog| {
                    catalog
                        .resolve_model(provider, model, OperationType::ImportMapping)
                        .is_ok()
                })
            })
        {
            Some("AI_MODEL_UNAVAILABLE")
        } else {
            None
        };
        Ok(AiImportStatus {
            ready: issue.is_none(),
            busy: window.state::<AiRequestGate>().is_busy(),
            provider: if uses_plan {
                Some("codex".into())
            } else {
                connection.provider
            },
            model: connection.model,
            reasoning: if uses_plan {
                Some(crate::chatgpt_plan::load_settings(store)?.0.reasoning)
            } else {
                None
            },
            issue: issue.map(str::to_owned),
        })
    });
    match result {
        Ok(value) => CommandResponse::success(value),
        Err(error) => crate::storage_failure(&error),
    }
}

fn failure<T: Serialize>(failure: MaterialFailure) -> CommandResponse<T> {
    CommandResponse::Failure {
        ok: false,
        error: ErrorEnvelope {
            code: failure.code.into(),
            message_key: "errors.aiImportFailed".into(),
            retryable: false,
            operation_id: None,
            details: failure.details,
        },
    }
}

#[tauri::command]
pub(crate) async fn begin_ai_document_import(
    window: WebviewWindow,
    request: BeginImportRequest,
) -> CommandResponse<Option<ImportReviewSnapshot>> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    match begin(&window, request).await {
        Ok(snapshot) => CommandResponse::success(snapshot),
        Err(error) => failure(error),
    }
}

fn base(
    state: &DesktopState,
    expected: Option<i64>,
) -> Result<VersionedResumeResponse, MaterialFailure> {
    state
        .with_store(|store| match (store.load_draft()?, expected) {
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
        })
        .map_err(|error| match error {
            ort_storage::StorageError::RevisionConflict => "REVISION_CONFLICT".into(),
            _ => "STORAGE_UNAVAILABLE".into(),
        })
}

pub(super) async fn cancelled(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[allow(clippy::too_many_lines)]
async fn begin(
    window: &WebviewWindow,
    request: BeginImportRequest,
) -> Result<Option<ImportReviewSnapshot>, MaterialFailure> {
    let app = window.app_handle();
    let state = app.state::<DesktopState>();
    let _operation = app
        .state::<ExportState>()
        .begin()
        .ok_or("LOCAL_DATA_OPERATION_BUSY")?;
    let job = JobLease::begin(&state.reviews).ok_or("LOCAL_DATA_OPERATION_BUSY")?;
    let gate = app.state::<AiRequestGate>();
    let operation_id = Uuid::now_v7();
    let lease = gate.begin(operation_id).ok_or("AI_BUSY")?;
    let (provider, profile) = MaterialProvider::load(&state, OperationType::ImportMapping)?;
    if let MaterialProvider::Codex(settings) = &provider {
        app.state::<crate::chatgpt_plan::PlanRuntime>()
            .validate_image_selection(profile, settings)?;
    }
    let original = base(&state, request.payload.expected_revision)?;
    if state
        .reviews
        .sessions
        .lock()
        .map_err(|_| "IMPORT_REVIEW_UNAVAILABLE")?
        .is_active(Instant::now())
    {
        return Err("LOCAL_DATA_OPERATION_BUSY".into());
    }
    let picker_window = window.clone();
    let cancel = Arc::clone(&job.cancel);
    let pdf = tauri::async_runtime::spawn_blocking(move || pick_pdf(&picker_window, &cancel))
        .await
        .map_err(|_| "IMPORT_FAILED")??;
    let Some(pdf) = pdf else {
        return Ok(None);
    };
    if job.cancel.load(Ordering::Acquire) {
        return Err("AI_CANCELLED".into());
    }
    let uses_images = provider.source() == "chatgpt_plan";
    let prepared = super::ai_pdf::render(window, &pdf, &job.cancel, uses_images).await?;
    let signal = lease.cancellation_signal();
    let input = ort_ai::resume_import::input_with_pdf_hyperlinks(prepared.links());
    let pages = uses_images.then_some(prepared);
    let request = provider.execute_pdf(
        PassContext {
            window,
            state: &state,
            profile,
            operation_id,
            operation: OperationType::ImportMapping,
            system: ort_ai::resume_import::SYSTEM,
            input,
            call: 1,
            maximum: 1,
            previous: None,
            signal: Arc::clone(&signal),
        },
        &pdf,
        pages,
    );
    tokio::pin!(request);
    let completed = tokio::select! {
        result = &mut request => result,
        () = cancelled(&job.cancel) => { signal.cancel(); request.await },
    }?;
    if job.cancel.load(Ordering::Acquire) || signal.is_cancelled() {
        return Err("AI_CANCELLED".into());
    }
    let document = ort_ai::resume_import::decode(&completed.text).map_err(|issue| {
        let code = "AI_OUTPUT_INVALID";
        let details = ort_domain::AiFailureDetails {
            code: code.into(),
            validation_issues: vec![issue],
            ..Default::default()
        };
        let _ = state
            .with_store(|store| store.record_ai_failure_details(completed.attempt_id, &details));
        completed.failure(
            code,
            &details,
            operation_id,
            OperationType::ImportMapping,
            1,
            1,
        )
    })?;
    state
        .with_store(|store| {
            ort_application::application_workspace::ensure_profile(store, profile)?;
            let current = store.load_draft()?;
            if current.as_ref().map_or(0, |draft| draft.revision) != original.revision {
                return Err(ort_storage::StorageError::RevisionConflict);
            }
            let mut sessions = state
                .reviews
                .sessions
                .lock()
                .map_err(|_| ort_storage::StorageError::Unavailable)?;
            if job.cancel.load(Ordering::Acquire) {
                return Err(ort_storage::StorageError::Unavailable);
            }
            let token = sessions
                .begin_document(state.reviews.owner, original, document, Instant::now())
                .map_err(|_| ort_storage::StorageError::Unavailable)?;
            let review = sessions
                .read(state.reviews.owner, token, Instant::now())
                .map_err(|_| ort_storage::StorageError::Unavailable)?;
            Ok(Some(review.snapshot(token.identifier())))
        })
        .map_err(|error| match error {
            ort_storage::StorageError::RevisionConflict => "REVISION_CONFLICT".into(),
            _ => "IMPORT_REVIEW_UNAVAILABLE".into(),
        })
}

fn pick_pdf(
    window: &WebviewWindow,
    cancel: &AtomicBool,
) -> Result<Option<Vec<u8>>, MaterialFailure> {
    let selected = window
        .dialog()
        .file()
        .set_parent(window)
        .set_title("Choose a PDF resume to send to AI")
        .add_filter("PDF resume", &["pdf"])
        .blocking_pick_file();
    if cancel.load(Ordering::Acquire) {
        return Err("AI_CANCELLED".into());
    }
    let Some(selected) = selected else {
        return Ok(None);
    };
    let FilePath::Path(path) = selected else {
        return Err("IMPORT_INVALID_SOURCE".into());
    };
    let source = ort_platform::read_native_document(&path).map_err(|_| "IMPORT_INVALID_SOURCE")?;
    if source.format != ort_platform::NativeDocumentFormat::Pdf {
        return Err("AI_PDF_REQUIRED".into());
    }
    if source.bytes.len() > ort_ai::resume_import::MAX_PDF_BYTES {
        return Err("AI_PDF_TOO_LARGE".into());
    }
    ort_documents::import_source::inspect_source(
        &source.bytes,
        ort_documents::import::InputFormat::Pdf,
    )
    .map_err(|_| "IMPORT_INVALID_SOURCE")?;
    Ok(Some(source.bytes))
}
