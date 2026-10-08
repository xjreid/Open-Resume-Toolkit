//! Tracker commands and validated retained application snapshots.
#![allow(clippy::needless_pass_by_value)] // Tauri command parameters are owned by the IPC adapter.

use base64::{Engine, engine::general_purpose::STANDARD};
use ort_domain::{CommandResponse, ResumeDocument};
use ort_storage::{
    StorageError,
    tracker::{TrackerRecord, TrackerSummary},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Manager, WebviewWindow};
use uuid::Uuid;

use crate::{DesktopState, application_exports, storage_failure, window_not_authorized};

pub use ort_domain::TrackerEntry;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinishSelection {
    pub entry: TrackerEntry,
}

pub use ort_domain::valid_application_url as valid_url;

fn validate(entry: &TrackerEntry) -> Result<(), StorageError> {
    ort_domain::validate_tracker_entry(entry).map_err(|_| StorageError::InvalidData)
}

fn same_retained_content(before: &TrackerEntry, after: &TrackerEntry) -> bool {
    let old = serde_json::to_value((
        &before.resume,
        &before.cover_letter,
        &before.cover_contact,
        &before.answers,
        before.style,
    ));
    let new = serde_json::to_value((
        &after.resume,
        &after.cover_letter,
        &after.cover_contact,
        &after.answers,
        after.style,
    ));
    matches!((old, new), (Ok(old), Ok(new)) if old == new)
}

#[cfg(test)]
mod content_tests {
    use super::*;
    use ort_domain::{ApprovedAnswer, DocumentStyle, RoleInfo};

    fn entry() -> TrackerEntry {
        TrackerEntry {
            company: "Company".into(),
            title: String::new(),
            location: String::new(),
            date_applied: String::new(),
            status: "applied".into(),
            custom_status: String::new(),
            source_url: String::new(),
            resume: Some(ResumeDocument::empty("Final resume")),
            cover_letter: Some("Final letter".into()),
            cover_contact: None,
            answers: vec![ApprovedAnswer {
                question: "Why?".into(),
                answer: "Because.".into(),
            }],
            style: DocumentStyle::Technical,
        }
    }

    #[test]
    fn metadata_can_change_without_changing_retained_content() {
        let original = entry();
        let mut edited = original.clone();
        edited.company = "New name".into();
        assert!(same_retained_content(&original, &edited));
        edited.resume = Some(ResumeDocument::empty("Earlier draft"));
        assert!(!same_retained_content(&original, &edited));
        edited.resume = original.resume.clone();
        edited.cover_letter = Some("Changed letter".into());
        assert!(!same_retained_content(&original, &edited));
        assert!(has_retained_content(&original));
    }

    #[test]
    fn tracker_accepts_source_text_without_a_url_scheme() {
        let mut record = entry();
        record.source_url = "linkedin.com/jobs/123".into();
        assert!(validate(&record).is_ok());
        record.source_url = "Job board reference 123".into();
        assert!(validate(&record).is_ok());
    }

    #[test]
    fn finish_retains_all_current_materials() {
        let mut workspace = ort_domain::ApplicationWorkspace {
            schema_version: 1,
            tracker_metadata: None,
            published_revision: 1,
            job_description: "Job".into(),
            job_url: String::new(),
            role_info: RoleInfo::default(),
            resume: ResumeDocument::empty("Final corrected resume"),
            change_points: Vec::new(),
            change_summary: Vec::new(),
            alerts: Vec::new(),
            alerts_truncated: false,
            dismissed_alert_ids: Vec::new(),
            ignore_all_alerts: false,
            cover_letter: Some("Final letter".into()),
            question: String::new(),
            answer: String::new(),
            approved_answers: Vec::new(),
            style: DocumentStyle::Technical,
        };
        let selection = FinishSelection { entry: entry() };
        let retained = selected_snapshot(selection, &workspace).unwrap();
        assert_eq!(retained.resume.unwrap().title, "Final corrected resume");
        assert_eq!(retained.cover_letter.as_deref(), Some("Final letter"));
        assert!(retained.answers.is_empty());
        workspace.question = "Why this role?".into();
        workspace.answer = "Final reviewed answer".into();
        workspace.approved_answers.push(ApprovedAnswer {
            question: "Earlier question?".into(),
            answer: "Earlier final answer".into(),
        });
        let retained = selected_snapshot(FinishSelection { entry: entry() }, &workspace).unwrap();
        assert_eq!(retained.resume.unwrap().title, "Final corrected resume");
        assert_eq!(retained.cover_letter.as_deref(), Some("Final letter"));
        assert_eq!(retained.answers.len(), 2);
        assert_eq!(retained.answers[0].answer, "Earlier final answer");
        assert_eq!(retained.answers[1].question, "Why this role?");
        assert_eq!(retained.answers[1].answer, "Final reviewed answer");
        assert_eq!(workspace.answer, "Final reviewed answer");
    }
}

fn has_retained_content(entry: &TrackerEntry) -> bool {
    entry.resume.is_some()
        || entry.cover_letter.is_some()
        || entry.cover_contact.is_some()
        || !entry.answers.is_empty()
}

fn decode(record: TrackerRecord) -> Result<SavedTrackerEntry, StorageError> {
    let entry: TrackerEntry =
        serde_json::from_value(record.value.clone()).map_err(|_| StorageError::InvalidData)?;
    validate(&entry)?;
    Ok(SavedTrackerEntry {
        id: record.id,
        revision: record.revision,
        value: entry,
    })
}

fn tracker_failure<T: Serialize>(error: &StorageError) -> CommandResponse<T> {
    match error {
        StorageError::InvalidData => {
            CommandResponse::failure("TRACKER_INVALID", "errors.trackerInvalid", false)
        }
        StorageError::NotFound => {
            CommandResponse::failure("TRACKER_NOT_FOUND", "errors.trackerNotFound", false)
        }
        _ => storage_failure(error),
    }
}

#[derive(Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedTrackerEntry {
    pub id: String,
    pub revision: i64,
    pub value: TrackerEntry,
}

fn summary(record: TrackerRecord) -> Result<TrackerSummary, StorageError> {
    let saved = decode(record)?;
    let entry = saved.value;
    Ok(TrackerSummary {
        id: saved.id,
        revision: saved.revision,
        value: ort_domain::TrackerMetadata {
            company: entry.company,
            title: entry.title,
            location: entry.location,
            date_applied: entry.date_applied,
            status: entry.status,
            custom_status: entry.custom_status,
            source_url: entry.source_url,
        },
        has_resume: entry.resume.is_some(),
        has_cover_letter: entry.cover_letter.is_some(),
        answer_count: u32::try_from(entry.answers.len()).map_err(|_| StorageError::InvalidData)?,
    })
}

#[tauri::command]
pub fn list_tracker_entries(
    window: WebviewWindow,
    offset: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
    status: Option<String>,
) -> CommandResponse<Vec<TrackerSummary>> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match window.state::<DesktopState>().with_store(|store| {
        store.tracker_summaries(
            offset.unwrap_or(0),
            limit.unwrap_or(100),
            search.as_deref().unwrap_or(""),
            status.as_deref().unwrap_or(""),
        )
    }) {
        Ok(entries) => CommandResponse::success(entries),
        Err(error) => tracker_failure(&error),
    }
}

#[tauri::command]
pub fn get_tracker_entry(window: WebviewWindow, id: String) -> CommandResponse<SavedTrackerEntry> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match window
        .state::<DesktopState>()
        .with_store(|store| decode(store.tracker_get(&id)?.ok_or(StorageError::NotFound)?))
    {
        Ok(record) => CommandResponse::success(record),
        Err(error) => tracker_failure(&error),
    }
}

#[tauri::command]
pub fn save_tracker_metadata(
    window: WebviewWindow,
    id: String,
    expected_revision: i64,
    metadata: ort_domain::TrackerMetadata,
) -> CommandResponse<TrackerSummary> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match window.state::<DesktopState>().with_store(|store| {
        let mut saved = decode(store.tracker_get(&id)?.ok_or(StorageError::NotFound)?)?;
        if saved.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        metadata.apply_to(&mut saved.value);
        validate(&saved.value)?;
        let value = serde_json::to_value(saved.value).map_err(|_| StorageError::InvalidData)?;
        summary(store.tracker_save(&id, Some(expected_revision), &value)?)
    }) {
        Ok(record) => CommandResponse::success(record),
        Err(error) => tracker_failure(&error),
    }
}

#[tauri::command]
pub fn save_tracker_entry(
    window: WebviewWindow,
    id: Option<String>,
    expected_revision: Option<i64>,
    entry: TrackerEntry,
) -> CommandResponse<TrackerSummary> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = validate(&entry) {
        return tracker_failure(&error);
    }
    let id = id.unwrap_or_else(|| Uuid::now_v7().to_string());
    let Ok(value) = serde_json::to_value(&entry) else {
        return tracker_failure(&StorageError::InvalidData);
    };
    match window.state::<DesktopState>().with_store(|store| {
        if let Some(revision) = expected_revision {
            let current = store.tracker_get(&id)?.ok_or(StorageError::NotFound)?;
            if current.revision != revision {
                return Err(StorageError::RevisionConflict);
            }
            let existing: TrackerEntry =
                serde_json::from_value(current.value).map_err(|_| StorageError::InvalidData)?;
            if !same_retained_content(&existing, &entry) {
                return Err(StorageError::InvalidData);
            }
        } else if has_retained_content(&entry) {
            return Err(StorageError::InvalidData);
        }
        summary(store.tracker_save(&id, expected_revision, &value)?)
    }) {
        Ok(record) => CommandResponse::success(record),
        Err(error) => tracker_failure(&error),
    }
}

#[tauri::command]
pub fn delete_tracker_entry(
    window: WebviewWindow,
    id: String,
    expected_revision: i64,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match window
        .state::<DesktopState>()
        .with_store(|store| store.tracker_delete(&id, expected_revision))
    {
        Ok(()) => CommandResponse::success(true),
        Err(error) => tracker_failure(&error),
    }
}

#[tauri::command]
pub fn open_tracker_link(window: WebviewWindow, target: String) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Ok(url) = url::Url::parse(&target) else {
        return CommandResponse::failure("LINK_INVALID", "errors.linkInvalid", false);
    };
    if target.len() > 16_384
        || !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return CommandResponse::failure("LINK_INVALID", "errors.linkInvalid", false);
    }
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("/usr/bin/open")
        .arg(&target)
        .spawn();
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(&target)
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let opened = std::process::Command::new("xdg-open").arg(&target).spawn();
    match opened {
        Ok(_) => CommandResponse::success(true),
        Err(_) => CommandResponse::failure("LINK_OPEN_FAILED", "errors.linkOpenFailed", true),
    }
}

#[tauri::command]
pub fn preview_tracker_pdf(
    window: WebviewWindow,
    id: String,
    expected_revision: i64,
    kind: ort_domain::MaterialKind,
) -> CommandResponse<application_exports::MaterialPdf> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let prepared = window.state::<DesktopState>().with_store(|store| {
        let record = store.tracker_get(&id)?.ok_or(StorageError::NotFound)?;
        if record.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        let entry: TrackerEntry =
            serde_json::from_value(record.value).map_err(|_| StorageError::InvalidData)?;
        validate(&entry)?;
        if matches!(kind, ort_domain::MaterialKind::Resume) && entry.resume.is_none() {
            return Err(StorageError::NotFound);
        }
        let resume = entry.resume.unwrap_or_else(|| {
            let mut document = ResumeDocument::empty("Cover letter");
            if let Some(contact) = entry.cover_contact {
                document.contact = contact;
            }
            document
        });
        let document = ort_application::material_document::document_for(
            &resume,
            entry.cover_letter.as_deref(),
            kind,
        )?;
        Ok((document, entry.style))
    });
    let (document, style) = match prepared {
        Ok(value) => value,
        Err(error) => return tracker_failure(&error),
    };
    match ort_render::render_pdf_with_style(&document, style) {
        Ok(pdf) => CommandResponse::success(application_exports::MaterialPdf {
            base64: STANDARD.encode(&pdf.bytes),
            filename: match kind {
                ort_domain::MaterialKind::Resume => "tailored-resume.pdf",
                ort_domain::MaterialKind::CoverLetter => "cover-letter.pdf",
            }
            .into(),
        }),
        Err(_) => CommandResponse::failure("PDF_UNAVAILABLE", "errors.pdfUnavailable", true),
    }
}

pub fn finish_with_selection(
    window: &WebviewWindow,
    expected_revision: i64,
    selection: Option<FinishSelection>,
) -> CommandResponse<bool> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let result = window.state::<DesktopState>().with_store(|store| {
        let current =
            ort_application::application_workspace::load(store)?.ok_or(StorageError::NotFound)?;
        if current.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        let selected = selection
            .map(|selection| {
                let entry = selected_snapshot(selection, &current.workspace)?;
                let value: Value =
                    serde_json::to_value(entry).map_err(|_| StorageError::InvalidData)?;
                Ok((Uuid::now_v7().to_string(), value))
            })
            .transpose()?;
        store.tracker_finish_workspace(
            expected_revision,
            selected.as_ref().map(|(id, value)| (id.as_str(), value)),
        )
    });
    match result {
        Ok(()) => {
            window.state::<application_exports::DragFiles>().clear();
            window
                .state::<application_exports::ApplicationExportState>()
                .clear();
            CommandResponse::success(true)
        }
        Err(error) => tracker_failure(&error),
    }
}

fn selected_snapshot(
    mut selection: FinishSelection,
    workspace: &ort_domain::ApplicationWorkspace,
) -> Result<TrackerEntry, StorageError> {
    validate(&selection.entry)?;
    selection.entry.resume = Some(workspace.resume.clone());
    selection
        .entry
        .cover_letter
        .clone_from(&workspace.cover_letter);
    selection.entry.cover_contact = selection
        .entry
        .cover_letter
        .as_ref()
        .map(|_| workspace.resume.contact.clone());
    let mut final_workspace = workspace.clone();
    ort_application::application_workspace::retain_current_answer(&mut final_workspace)?;
    selection.entry.answers = final_workspace.approved_answers;
    selection.entry.style = workspace.style;
    if selection.entry.source_url.is_empty() {
        selection.entry.source_url.clone_from(&workspace.job_url);
    }
    validate(&selection.entry)?;
    Ok(selection.entry)
}
