//! The overlay's temporary, encrypted application workspace. Every command is
//! scoped to the overlay; only user edits may change validated generated text.
#![allow(clippy::needless_pass_by_value, clippy::manual_let_else)]
use crate::application_exports::ApplicationExportState;
use ort_ai::materials::{self, MAX_JOB_CHARS, MAX_QUESTION_CHARS};
use ort_ai::{OperationType, Preset, Provider};
use ort_application::application_workspace::{
    ANSWER_SYSTEM, COVER_SYSTEM, REFINE_ANSWER_SYSTEM, REFINE_SYSTEM, TAILOR_SYSTEM,
    ensure_profile, load, load_stage_one, merge_refinement_alerts, resume_system, save,
    save_reviewed, save_stage_one, validate_stage_one, validate_workspace,
};
use ort_application::material_document::preflight_pdf;
use ort_domain::MaterialKind;
use ort_domain::{CommandResponse, DocumentStyle, ResumeDocument};
use ort_storage::StorageError;
use serde::Serialize;
use serde_json::json;
use tauri::{Manager, WebviewWindow};

use crate::{DesktopState, ai_request, storage_failure, window_not_authorized};

const WORKSPACE_KEY: &str = ort_domain::APPLICATION_WORKSPACE_KEY;
const STAGE_ONE_KEY: &str = ort_domain::APPLICATION_STAGE_ONE_KEY;
const PENDING_CAPTURE_KEY: &str = ort_domain::APPLICATION_CAPTURE_KEY;
const MAX_CAPTURE_TEXT_BYTES: usize = ort_domain::MAX_CAPTURE_TEXT_BYTES;
const SCHEMA_VERSION: u16 = ort_domain::APPLICATION_SCHEMA_VERSION;
use ort_application::application_workspace::retain_current_answer;
#[cfg(test)]
use ort_application::material_document::document_for;

use ort_domain::{ApplicationWorkspace, SavedStageOneDraft, SavedWorkspace, StageOneDraft};

#[derive(Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedPendingCapture {
    pub revision: i64,
    pub capture: ort_ipc::CaptureEnvelope,
}

fn load_pending_capture(
    store: &ort_storage::EncryptedStore,
) -> Result<Option<SavedPendingCapture>, StorageError> {
    let Some(saved) = store.load_setting(PENDING_CAPTURE_KEY)? else {
        return Ok(None);
    };
    if saved.value.is_null() {
        return Ok(None);
    }
    let capture: ort_ipc::CaptureEnvelope =
        serde_json::from_value(saved.value).map_err(|_| StorageError::InvalidData)?;
    if capture.protocol_version != ort_ipc::PROTOCOL_VERSION
        || capture.kind != "capture.selection"
        || capture.payload.text.trim().is_empty()
        || capture.payload.text.len() > MAX_CAPTURE_TEXT_BYTES
        || capture.payload.url.is_empty()
        || capture.payload.title.len() > 2_000
        || !matches!(capture.payload.browser.as_str(), "chrome" | "edge")
        || !crate::tracker::valid_url(&capture.payload.url)
        || !matches!(capture.payload.target.as_str(), "job" | "question")
    {
        return Err(StorageError::InvalidData);
    }
    Ok(Some(SavedPendingCapture {
        revision: saved.revision,
        capture,
    }))
}

/// Desktop destination for an already authenticated native-host frame. This
/// function is intentionally not a Tauri command; renderer input cannot claim
/// that it came from the browser bridge. The opt-in dev transport calls it only
/// after authenticating the frame.
#[allow(dead_code)]
pub(crate) fn accept_authenticated_capture(
    store: &ort_storage::EncryptedStore,
    frame: &[u8],
    now_ms: i64,
) -> Result<uuid::Uuid, StorageError> {
    let capture =
        ort_ipc::validate_capture(frame, now_ms).map_err(|_| StorageError::InvalidData)?;
    let request_id = capture.request_id;
    if !crate::tracker::valid_url(&capture.payload.url) {
        return Err(StorageError::InvalidData);
    }
    if let Some(existing) = load_pending_capture(store)? {
        return if existing.capture.request_id == request_id {
            Ok(request_id)
        } else {
            Err(StorageError::RevisionConflict)
        };
    }
    let expected = store
        .load_setting(PENDING_CAPTURE_KEY)?
        .map(|saved| saved.revision);
    let value = serde_json::to_value(capture).map_err(|_| StorageError::InvalidData)?;
    store.save_setting(PENDING_CAPTURE_KEY, expected, &value)?;
    Ok(request_id)
}

#[tauri::command]
pub fn load_application_capture(
    window: WebviewWindow,
) -> CommandResponse<Option<SavedPendingCapture>> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    match window
        .state::<DesktopState>()
        .with_store(load_pending_capture)
    {
        Ok(value) => CommandResponse::success(value),
        Err(problem) => storage_failure(&problem),
    }
}

/// The overlay authorizes a bounded capture generation; Chrome receives it by
/// authenticated heartbeat. A late result after cancellation cannot reach intake.
#[tauri::command]
pub fn request_application_capture(
    window: WebviewWindow,
    target: Option<String>,
) -> CommandResponse<ort_ipc::capture_session::CaptureStatus> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let target = target.as_deref().unwrap_or("job");
    let pending = window
        .state::<DesktopState>()
        .with_store(load_pending_capture);
    match pending {
        Ok(Some(_)) => return error("CAPTURE_PENDING"),
        Err(problem) => return storage_failure(&problem),
        Ok(None) => {}
    }
    match window
        .state::<crate::browser_bridge::BrowserBridgeState>()
        .start(target)
    {
        Ok(status) => CommandResponse::success(status),
        Err(code) => error(code),
    }
}

#[tauri::command]
pub fn resolve_application_capture(
    window: WebviewWindow,
    request_id: uuid::Uuid,
    accept: bool,
    expected_revision: Option<i64>,
    reviewed_text: Option<String>,
    reviewed_url: Option<String>,
) -> CommandResponse<bool> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let result = window.state::<DesktopState>().with_store(|store| {
        resolve_pending_capture(
            store,
            request_id,
            accept,
            expected_revision,
            reviewed_text.as_deref(),
            reviewed_url.as_deref(),
        )
    });
    match result {
        Ok(()) => CommandResponse::success(true),
        Err(problem) => storage_failure(&problem),
    }
}

/// Apply the authenticated job payload and return the saved draft while holding
/// the store lock. The pending capture is consumed atomically with the draft save.
#[tauri::command]
pub fn apply_application_job_capture(
    window: WebviewWindow,
    request_id: uuid::Uuid,
    expected_revision: Option<i64>,
) -> CommandResponse<SavedStageOneDraft> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    match window
        .state::<DesktopState>()
        .with_store(|store| apply_pending_job_capture(store, request_id, expected_revision))
    {
        Ok(saved) => CommandResponse::success(saved),
        Err(problem) => storage_failure(&problem),
    }
}

fn apply_pending_job_capture(
    store: &ort_storage::EncryptedStore,
    request_id: uuid::Uuid,
    expected_revision: Option<i64>,
) -> Result<SavedStageOneDraft, StorageError> {
    let pending = load_pending_capture(store)?.ok_or(StorageError::NotFound)?;
    if pending.capture.payload.target != "job" {
        return Err(StorageError::InvalidData);
    }
    resolve_pending_capture(
        store,
        request_id,
        true,
        expected_revision,
        Some(&pending.capture.payload.text),
        Some(&pending.capture.payload.url),
    )?;
    load_stage_one(store)?.ok_or(StorageError::NotFound)
}

fn resolve_pending_capture(
    store: &ort_storage::EncryptedStore,
    request_id: uuid::Uuid,
    accept: bool,
    expected_revision: Option<i64>,
    reviewed_text: Option<&str>,
    reviewed_url: Option<&str>,
) -> Result<(), StorageError> {
    let pending = load_pending_capture(store)?.ok_or(StorageError::NotFound)?;
    if pending.capture.request_id != request_id {
        return Err(StorageError::RevisionConflict);
    }
    if !accept {
        return store.resolve_application_capture(pending.revision, None);
    }
    let reviewed_text = reviewed_text.ok_or(StorageError::InvalidData)?;
    let reviewed_url = reviewed_url.ok_or(StorageError::InvalidData)?;
    if reviewed_text.trim().is_empty()
        || reviewed_text.len() > MAX_CAPTURE_TEXT_BYTES
        || !crate::tracker::valid_url(reviewed_url)
    {
        return Err(StorageError::InvalidData);
    }
    match pending.capture.payload.target.as_str() {
        "job" => {
            if load(store)?.is_some() {
                return Err(StorageError::RevisionConflict);
            }
            let current = load_stage_one(store)?;
            if current.as_ref().map(|saved| saved.revision) != expected_revision {
                return Err(StorageError::RevisionConflict);
            }
            let draft = StageOneDraft {
                job_description: reviewed_text.to_owned(),
                job_url: reviewed_url.to_owned(),
                style: current
                    .as_ref()
                    .map_or(DocumentStyle::Technical, |saved| saved.draft.style),
            };
            validate_stage_one(&draft)?;
            let value = serde_json::to_value(draft).map_err(|_| StorageError::InvalidData)?;
            store.resolve_application_capture(
                pending.revision,
                Some((STAGE_ONE_KEY, expected_revision, &value)),
            )
        }
        "question" => {
            let mut current = load(store)?.ok_or(StorageError::NotFound)?;
            if Some(current.revision) != expected_revision {
                return Err(StorageError::RevisionConflict);
            }
            retain_current_answer(&mut current.workspace)?;
            reviewed_text.clone_into(&mut current.workspace.question);
            validate_workspace(&current.workspace)?;
            let value =
                serde_json::to_value(current.workspace).map_err(|_| StorageError::InvalidData)?;
            store.resolve_application_capture(
                pending.revision,
                Some((WORKSPACE_KEY, expected_revision, &value)),
            )
        }
        _ => Err(StorageError::InvalidData),
    }
}

#[tauri::command]
pub fn load_application_stage_one(
    window: WebviewWindow,
) -> CommandResponse<Option<SavedStageOneDraft>> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    match window.state::<DesktopState>().with_store(load_stage_one) {
        Ok(value) => CommandResponse::success(value),
        Err(problem) => storage_failure(&problem),
    }
}

#[tauri::command]
pub fn save_application_stage_one(
    window: WebviewWindow,
    expected_profile_id: uuid::Uuid,
    expected_revision: Option<i64>,
    draft: StageOneDraft,
) -> CommandResponse<SavedStageOneDraft> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    match window.state::<DesktopState>().with_store(|store| {
        ensure_profile(store, expected_profile_id)?;
        save_stage_one(store, expected_revision, &draft)
    }) {
        Ok(value) => CommandResponse::success(value),
        Err(problem) => storage_failure(&problem),
    }
}

#[derive(Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent readiness flags are serialized for the desktop UI"
)]
pub struct ApplicationContext {
    pub profile_id: uuid::Uuid,
    pub published_revision: Option<i64>,
    pub ai_label: String,
    pub ai_ready: bool,
    pub ai_busy: bool,
    pub selected_key_ready: bool,
    pub selected_key_id: Option<uuid::Uuid>,
    pub preset: Option<String>,
    pub preset_label: String,
    pub preset_options: Vec<ApplicationPresetOption>,
    pub browser_connected: bool,
}

#[derive(Clone, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationPresetOption {
    pub preset: String,
    pub label: String,
    pub model: Option<String>,
    pub available: bool,
}

fn provider_from_name(value: &str) -> Option<Provider> {
    match value {
        "openai" => Some(Provider::OpenAi),
        "anthropic" => Some(Provider::Anthropic),
        "gemini" => Some(Provider::Gemini),
        _ => None,
    }
}

fn preset_from_name(value: &str) -> Option<Preset> {
    match value {
        "economy" => Some(Preset::Economy),
        "balanced" => Some(Preset::Balanced),
        "quality" => Some(Preset::Quality),
        _ => None,
    }
}

fn preset_display(value: &str) -> &'static str {
    match value {
        "economy" => "Economy",
        "balanced" => "Balanced",
        "quality" => "Quality",
        _ => "Preset",
    }
}

#[tauri::command]
pub fn application_context(window: WebviewWindow) -> CommandResponse<ApplicationContext> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let browser_connected = window
        .state::<crate::browser_bridge::BrowserBridgeState>()
        .connected();
    match window.state::<DesktopState>().with_store(|store| {
        let published_revision = store.load_latest_published()?.map(|item| item.revision);
        let connection = crate::ai_keys::request_connection(store, None)?;
        let ai_ready = connection.mode == "direct_api";
        let provider_name = connection
            .provider
            .clone()
            .unwrap_or_else(|| "Direct AI".into());
        let provider = provider_from_name(&provider_name);
        let catalog = ort_ai::builtin_catalog(&jiff::Timestamp::now().to_string(), None).ok();
        let preset_options = ["economy", "balanced", "quality"]
            .into_iter()
            .map(|preset_name| {
                let model = provider
                    .zip(catalog.as_ref())
                    .and_then(|(provider, catalog)| {
                        catalog
                            .resolve(
                                provider,
                                preset_from_name(preset_name)?,
                                OperationType::TailorResume,
                            )
                            .ok()
                            .map(|entry| entry.model.clone())
                    });
                ApplicationPresetOption {
                    preset: preset_name.into(),
                    label: format!(
                        "{}: {}",
                        preset_display(preset_name),
                        model.as_deref().unwrap_or("model unavailable")
                    ),
                    available: model.is_some(),
                    model,
                }
            })
            .collect::<Vec<_>>();
        let selected_model = connection.preset.as_deref().and_then(|preset| {
            preset_options
                .iter()
                .find(|option| option.preset == preset)
                .and_then(|option| option.model.clone())
        });
        let preset_label = connection.preset.as_deref().map_or_else(
            || "AI not configured".into(),
            |preset| {
                format!(
                    "{}: {}",
                    preset_display(preset),
                    selected_model.as_deref().unwrap_or("model unavailable")
                )
            },
        );
        let ai_label = if ai_ready {
            format!(
                "{provider_name} · {}",
                selected_model.unwrap_or_else(|| "model unavailable".into())
            )
        } else {
            "AI not configured".into()
        };
        Ok(ApplicationContext {
            profile_id: store.manifest().profile_id,
            published_revision,
            ai_label,
            ai_ready,
            ai_busy: window.state::<ai_request::AiRequestGate>().is_busy(),
            selected_key_ready: ai_ready,
            selected_key_id: connection.credential_id,
            preset: connection.preset,
            preset_label,
            preset_options,
            browser_connected,
        })
    }) {
        Ok(value) => CommandResponse::success(value),
        Err(problem) => storage_failure(&problem),
    }
}

#[tauri::command]
pub fn cancel_application_generation(window: WebviewWindow) -> CommandResponse<bool> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    CommandResponse::success(window.state::<ai_request::AiRequestGate>().cancel_overlay())
}

pub(crate) fn error<T: Serialize>(code: &'static str) -> CommandResponse<T> {
    CommandResponse::failure(
        code,
        "errors.applicationMaterial",
        code == "AI_BUSY" || code == "AI_PROVIDER_UNAVAILABLE",
    )
}

#[tauri::command]
pub fn load_application_workspace(
    window: WebviewWindow,
) -> CommandResponse<Option<SavedWorkspace>> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    match window.state::<DesktopState>().with_store(load) {
        Ok(saved) => CommandResponse::success(saved),
        Err(problem) => storage_failure(&problem),
    }
}

#[tauri::command]
pub async fn start_application(
    window: WebviewWindow,
    job_description: String,
    job_url: String,
    style: DocumentStyle,
) -> CommandResponse<SavedWorkspace> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    if job_description.trim().is_empty()
        || job_description.chars().count() > MAX_JOB_CHARS
        || !crate::tracker::valid_url(&job_url)
    {
        return error("JOB_INVALID");
    }
    let state = window.state::<DesktopState>();
    let prepared = state.with_store(|store| {
        if load(store)?.is_some() {
            return Err(StorageError::RevisionConflict);
        }
        store.load_latest_published()?.ok_or(StorageError::NotFound)
    });
    let source = match prepared {
        Ok(source) => source,
        Err(StorageError::NotFound) => return error("PUBLISHED_RESUME_REQUIRED"),
        Err(problem) => return storage_failure(&problem),
    };
    let input = json!({"schemaVersion":5,"publishedRevision":source.revision,"publishedResume":materials::resume_context(&source.document),"reviewedJobDescription":job_description,"style":style});
    let system = resume_system(TAILOR_SYSTEM);
    let result = match ai_request::execute_material(
        &window,
        OperationType::TailorResume,
        &system,
        input,
        |raw| {
            materials::validate_template_tailoring(
                &source.document,
                &job_description,
                raw,
                source.revision,
            )
            .map_err(|_| ())
        },
    )
    .await
    {
        Ok(result) => result,
        Err(code) => return error(code),
    };
    let workspace = ApplicationWorkspace {
        schema_version: SCHEMA_VERSION,
        published_revision: source.revision,
        job_description,
        job_url,
        role_info: result.role_info.unwrap_or_default(),
        resume: result.resume,
        change_points: result.change_points,
        alerts: result.alerts,
        alerts_truncated: result.alerts_truncated,
        dismissed_alert_ids: Vec::new(),
        ignore_all_alerts: false,
        cover_letter: None,
        question: String::new(),
        answer: String::new(),
        approved_answers: Vec::new(),
        style,
    };
    if let Err(code) = preflight_pdf(&workspace, MaterialKind::Resume) {
        return error(code);
    }
    match state.with_store(|store| {
        if load(store)?.is_some() {
            return Err(StorageError::RevisionConflict);
        }
        save(store, None, &workspace)
    }) {
        Ok(saved) => CommandResponse::success(saved),
        Err(problem) => storage_failure(&problem),
    }
}

#[tauri::command]
pub async fn regenerate_application_resume(
    window: WebviewWindow,
    expected_revision: i64,
    correction_instruction: String,
) -> CommandResponse<SavedWorkspace> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    if correction_instruction.trim().is_empty() || correction_instruction.len() > 2_000 {
        return error("INSTRUCTION_REQUIRED");
    }
    let state = window.state::<DesktopState>();
    let prepared = state.with_store(|store| {
        let saved = load(store)?.ok_or(StorageError::NotFound)?;
        if saved.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        let source = store
            .load_published_revision(saved.workspace.published_revision)?
            .ok_or(StorageError::NotFound)?;
        Ok((saved.workspace, source))
    });
    let (mut workspace, source) = match prepared {
        Ok(value) => value,
        Err(problem) => return storage_failure(&problem),
    };
    let input = json!({"schemaVersion":5,"publishedRevision":source.revision,"publishedResume":materials::resume_context(&source.document),
        "currentReviewedResume":materials::resume_context(&workspace.resume),"reviewedJobDescription":workspace.job_description,"reviewedRoleInfo":workspace.role_info,"style":workspace.style,"correctionInstruction":correction_instruction});
    let system = resume_system(REFINE_SYSTEM);
    let result = match ai_request::execute_material(
        &window,
        OperationType::RefineResume,
        &system,
        input,
        |raw| {
            materials::validate_refinement(
                &source.document,
                &workspace.resume,
                &workspace.job_description,
                raw,
                source.revision,
            )
            .map_err(|_| ())
        },
    )
    .await
    {
        Ok(result) => result,
        Err(code) => return error(code),
    };
    workspace.resume = result.resume;
    if let Some(role_info) = result.role_info {
        workspace.role_info = role_info;
    }
    workspace.change_points = result.change_points;
    merge_refinement_alerts(&mut workspace, result.alerts, result.alerts_truncated);
    if let Err(code) = preflight_pdf(&workspace, MaterialKind::Resume) {
        return error(code);
    }
    match state.with_store(|store| save(store, Some(expected_revision), &workspace)) {
        Ok(saved) => CommandResponse::success(saved),
        Err(problem) => storage_failure(&problem),
    }
}

#[tauri::command]
pub async fn generate_application_cover_letter(
    window: WebviewWindow,
    expected_revision: i64,
    instruction: String,
) -> CommandResponse<SavedWorkspace> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    if instruction.len() > 2_000 {
        return error("INSTRUCTION_INVALID");
    }
    let state = window.state::<DesktopState>();
    let prepared = state.with_store(|store| {
        let saved = load(store)?.ok_or(StorageError::NotFound)?;
        if saved.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        let source = store
            .load_published_revision(saved.workspace.published_revision)?
            .ok_or(StorageError::NotFound)?;
        Ok((saved.workspace, source))
    });
    let (mut workspace, source) = match prepared {
        Ok(value) => value,
        Err(problem) => return storage_failure(&problem),
    };
    let input = json!({"schemaVersion":2,"publishedResume":source.document,"reviewedJobDescription":workspace.job_description,"reviewedRoleInfo":workspace.role_info,"instruction":instruction});
    workspace.cover_letter = match ai_request::execute_material(
        &window,
        OperationType::CoverLetter,
        COVER_SYSTEM,
        input,
        |raw| {
            materials::validate_cover_letter(raw, &source.document, &workspace.job_description)
                .map_err(|_| ())
        },
    )
    .await
    {
        Ok(text) => Some(text),
        Err(code) => return error(code),
    };
    if let Err(code) = preflight_pdf(&workspace, MaterialKind::CoverLetter) {
        return error(code);
    }
    match state.with_store(|store| save(store, Some(expected_revision), &workspace)) {
        Ok(saved) => CommandResponse::success(saved),
        Err(problem) => storage_failure(&problem),
    }
}

#[tauri::command]
pub async fn generate_application_answer(
    window: WebviewWindow,
    expected_revision: i64,
    question: String,
) -> CommandResponse<SavedWorkspace> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    if question.trim().is_empty() || question.chars().count() > MAX_QUESTION_CHARS {
        return error("QUESTION_INVALID");
    }
    if materials::question_requires_personal_answer(&question) {
        return error("PERSONAL_ANSWER_REQUIRED");
    }
    let state = window.state::<DesktopState>();
    let prepared = state.with_store(|store| {
        let saved = load(store)?.ok_or(StorageError::NotFound)?;
        if saved.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        let source = store
            .load_published_revision(saved.workspace.published_revision)?
            .ok_or(StorageError::NotFound)?;
        Ok((saved.workspace, source))
    });
    let (mut workspace, source) = match prepared {
        Ok(value) => value,
        Err(problem) => return storage_failure(&problem),
    };
    if !workspace.answer.is_empty() {
        return error("ANSWER_ALREADY_EXISTS");
    }
    if workspace.approved_answers.len() >= 30 {
        return error("ANSWER_LIMIT_REACHED");
    }
    let input = json!({"schemaVersion":2,"publishedResume":source.document,"reviewedJobDescription":workspace.job_description,"reviewedRoleInfo":workspace.role_info,"reviewedQuestion":question});
    workspace.answer = match ai_request::execute_material(
        &window,
        OperationType::AnswerQuestion,
        ANSWER_SYSTEM,
        input,
        |raw| materials::validate_answer(raw, &source.document, &question, None).map_err(|_| ()),
    )
    .await
    {
        Ok(text) => text,
        Err(code) => return error(code),
    };
    workspace.question = question;
    match state.with_store(|store| save(store, Some(expected_revision), &workspace)) {
        Ok(saved) => CommandResponse::success(saved),
        Err(problem) => storage_failure(&problem),
    }
}

#[tauri::command]
pub async fn refine_application_answer(
    window: WebviewWindow,
    expected_revision: i64,
    instruction: String,
) -> CommandResponse<SavedWorkspace> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    if instruction.trim().is_empty() || instruction.chars().count() > 2_000 {
        return error("INSTRUCTION_INVALID");
    }
    let state = window.state::<DesktopState>();
    let prepared = state.with_store(|store| {
        let saved = load(store)?.ok_or(StorageError::NotFound)?;
        if saved.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        let source = store
            .load_published_revision(saved.workspace.published_revision)?
            .ok_or(StorageError::NotFound)?;
        Ok((saved.workspace, source))
    });
    let (mut workspace, source) = match prepared {
        Ok(value) => value,
        Err(problem) => return storage_failure(&problem),
    };
    if workspace.question.trim().is_empty() || workspace.answer.trim().is_empty() {
        return error("QUESTION_INVALID");
    }
    if materials::question_requires_personal_answer(&workspace.question) {
        return error("PERSONAL_ANSWER_REQUIRED");
    }
    let input = json!({"schemaVersion":2,"publishedResume":source.document,"reviewedJobDescription":workspace.job_description,"reviewedRoleInfo":workspace.role_info,"reviewedQuestion":workspace.question,"previousAnswer":workspace.answer,"correctionInstruction":instruction});
    workspace.answer = match ai_request::execute_material(
        &window,
        OperationType::AnswerQuestion,
        REFINE_ANSWER_SYSTEM,
        input,
        |raw| {
            materials::validate_answer(raw, &source.document, &workspace.question, None)
                .map_err(|_| ())
        },
    )
    .await
    {
        Ok(text) => text,
        Err(code) => return error(code),
    };
    match state.with_store(|store| save(store, Some(expected_revision), &workspace)) {
        Ok(saved) => CommandResponse::success(saved),
        Err(problem) => storage_failure(&problem),
    }
}

#[tauri::command]
pub fn save_application_workspace(
    window: WebviewWindow,
    expected_profile_id: uuid::Uuid,
    expected_revision: i64,
    workspace: ApplicationWorkspace,
) -> CommandResponse<SavedWorkspace> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let state = window.state::<DesktopState>();
    let prior = match state.with_store(|store| {
        ensure_profile(store, expected_profile_id)?;
        load(store)?.ok_or(StorageError::NotFound)
    }) {
        Ok(prior) => prior,
        Err(problem) => return storage_failure(&problem),
    };
    if prior.revision != expected_revision {
        return storage_failure(&StorageError::RevisionConflict);
    }
    let resume_pdf_unchanged = workspace.style == prior.workspace.style
        && printable_resume_unchanged(&prior.workspace.resume, &workspace.resume);
    if !resume_pdf_unchanged && let Err(code) = preflight_pdf(&workspace, MaterialKind::Resume) {
        return error(code);
    }
    if (workspace.cover_letter != prior.workspace.cover_letter
        || workspace.resume.contact != prior.workspace.resume.contact
        || workspace.style != prior.workspace.style)
        && workspace.cover_letter.is_some()
        && let Err(code) = preflight_pdf(&workspace, MaterialKind::CoverLetter)
    {
        return error(code);
    }
    match state.with_store(|store| {
        ensure_profile(store, expected_profile_id)?;
        save_reviewed(store, expected_revision, &workspace)
    }) {
        Ok(saved) => {
            if resume_pdf_unchanged {
                window.state::<ApplicationExportState>().promote_unchanged(
                    expected_profile_id,
                    prior.revision,
                    saved.revision,
                    MaterialKind::Resume,
                );
            }
            CommandResponse::success(saved)
        }
        Err(problem) => storage_failure(&problem),
    }
}

// Empty sections are useful while editing but contribute nothing to either
// exported format. Adding one should not force a PDF rerender or invalidate an
// already prepared download of the same printable content.
fn printable_resume_unchanged(before: &ResumeDocument, after: &ResumeDocument) -> bool {
    let mut before = before.clone();
    let mut after = after.clone();
    before
        .sections
        .retain(|section| !section.entries.is_empty());
    after.sections.retain(|section| !section.entries.is_empty());
    before == after
}

#[tauri::command]
pub fn finish_application(
    window: WebviewWindow,
    expected_revision: i64,
    selection: Option<crate::tracker::FinishSelection>,
) -> CommandResponse<bool> {
    crate::tracker::finish_with_selection(&window, expected_revision, selection)
}

#[cfg(test)]
#[path = "application_materials_tests.rs"]
mod tests;
