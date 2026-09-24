//! Tracker commands and validated retained application snapshots.

use base64::{Engine, engine::general_purpose::STANDARD};
use ort_domain::{CommandResponse, ContactDetails, DocumentLimits, DocumentStyle, ResumeDocument};
use ort_storage::{StorageError, tracker::TrackerRecord};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Manager, WebviewWindow};
use uuid::Uuid;

use crate::{
    DesktopState,
    application_materials::{self, ApprovedAnswer},
    storage_failure, window_not_authorized,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrackerEntry {
    pub company: String,
    pub title: String,
    pub location: String,
    pub date_applied: String,
    pub status: String,
    pub custom_status: String,
    pub source_url: String,
    pub resume: Option<ResumeDocument>,
    pub cover_letter: Option<String>,
    #[serde(default)]
    pub cover_contact: Option<ContactDetails>,
    pub answers: Vec<ApprovedAnswer>,
    pub style: DocumentStyle,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinishSelection {
    pub entry: TrackerEntry,
    pub retain_resume: bool,
    pub retain_cover_letter: bool,
    pub retain_answers: bool,
}

pub(crate) fn valid_url(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    if value.len() > 4_096 {
        return false;
    }
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && !url.query_pairs().any(|(key, _)| {
            let lower = key.to_ascii_lowercase();
            lower.starts_with("utm_")
                || matches!(
                    lower.as_str(),
                    "token"
                        | "access_token"
                        | "auth"
                        | "session"
                        | "code"
                        | "utm_source"
                        | "utm_medium"
                        | "utm_campaign"
                        | "utm_term"
                        | "utm_content"
                        | "gclid"
                        | "fbclid"
                        | "msclkid"
                        | "mc_cid"
                        | "mc_eid"
                )
        })
}

fn validate(entry: &TrackerEntry) -> Result<(), StorageError> {
    if entry.company.chars().count() > 200
        || entry.title.chars().count() > 200
        || entry.location.chars().count() > 200
        || entry.date_applied.len() > 10
        || (!entry.date_applied.is_empty()
            && jiff::civil::Date::strptime("%Y-%m-%d", &entry.date_applied).is_err())
        || entry.status.is_empty()
        || entry.status.len() > 40
        || !entry
            .status
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        || entry.custom_status.chars().count() > 80
        || (entry.status != "other" && !entry.custom_status.is_empty())
        || entry.source_url.len() > 4096
        || entry
            .cover_letter
            .as_ref()
            .is_some_and(|text| text.len() > 12_000)
        || (entry.cover_contact.is_some() && entry.cover_letter.is_none())
        || entry.cover_contact.as_ref().is_some_and(|contact| {
            let mut document = ResumeDocument::empty("Cover letter");
            document.contact = contact.clone();
            document.validate(DocumentLimits::default()).is_err()
        })
        || entry.answers.len() > 30
        || entry.answers.iter().any(|answer| {
            answer.question.trim().is_empty()
                || answer.question.chars().count() > 2_000
                || answer.answer.trim().is_empty()
                || answer.answer.chars().count() > 4_000
        })
        || entry
            .resume
            .as_ref()
            .is_some_and(|resume| resume.validate(DocumentLimits::default()).is_err())
    {
        return Err(StorageError::InvalidData);
    }
    Ok(())
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
    fn finish_retains_only_the_current_corrected_resume() {
        let workspace = application_materials::ApplicationWorkspace {
            schema_version: 1,
            published_revision: 1,
            job_description: "Job".into(),
            job_url: String::new(),
            role_info: Default::default(),
            resume: ResumeDocument::empty("Final corrected resume"),
            change_points: Vec::new(),
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
        let selection = FinishSelection {
            entry: entry(),
            retain_resume: true,
            retain_cover_letter: true,
            retain_answers: false,
        };
        let retained = selected_snapshot(selection, &workspace).unwrap();
        assert_eq!(retained.resume.unwrap().title, "Final corrected resume");
        assert_eq!(retained.cover_letter.as_deref(), Some("Final letter"));
        assert!(retained.answers.is_empty());
    }
}

fn has_retained_content(entry: &TrackerEntry) -> bool {
    entry.resume.is_some()
        || entry.cover_letter.is_some()
        || entry.cover_contact.is_some()
        || !entry.answers.is_empty()
}

fn decode(record: TrackerRecord) -> Result<TrackerRecord, StorageError> {
    let entry: TrackerEntry =
        serde_json::from_value(record.value.clone()).map_err(|_| StorageError::InvalidData)?;
    validate(&entry)?;
    Ok(record)
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

#[tauri::command]
pub fn list_tracker_entries(window: WebviewWindow) -> CommandResponse<Vec<TrackerRecord>> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match window
        .state::<DesktopState>()
        .with_store(|store| store.tracker_list()?.into_iter().map(decode).collect())
    {
        Ok(entries) => CommandResponse::success(entries),
        Err(error) => tracker_failure(&error),
    }
}

#[tauri::command]
pub fn save_tracker_entry(
    window: WebviewWindow,
    id: Option<String>,
    expected_revision: Option<i64>,
    entry: TrackerEntry,
) -> CommandResponse<TrackerRecord> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = validate(&entry) {
        return tracker_failure(&error);
    }
    let id = id.unwrap_or_else(|| Uuid::now_v7().to_string());
    let value = match serde_json::to_value(&entry) {
        Ok(value) => value,
        Err(_) => return tracker_failure(&StorageError::InvalidData),
    };
    match window.state::<DesktopState>().with_store(|store| {
        if let Some(revision) = expected_revision {
            let current = store
                .tracker_list()?
                .into_iter()
                .find(|record| record.id == id)
                .ok_or(StorageError::NotFound)?;
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
        store.tracker_save(&id, expected_revision, &value)
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
    kind: application_materials::MaterialKind,
) -> CommandResponse<application_materials::MaterialPdf> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let prepared = window.state::<DesktopState>().with_store(|store| {
        let record = store
            .tracker_list()?
            .into_iter()
            .find(|record| record.id == id)
            .ok_or(StorageError::NotFound)?;
        if record.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        let entry: TrackerEntry =
            serde_json::from_value(record.value).map_err(|_| StorageError::InvalidData)?;
        validate(&entry)?;
        if matches!(kind, application_materials::MaterialKind::Resume) && entry.resume.is_none() {
            return Err(StorageError::NotFound);
        }
        let resume = entry.resume.unwrap_or_else(|| {
            let mut document = ResumeDocument::empty("Cover letter");
            if let Some(contact) = entry.cover_contact {
                document.contact = contact;
            }
            document
        });
        let document = application_materials::document_for(
            &application_materials::ApplicationWorkspace {
                schema_version: 1,
                published_revision: 1,
                job_description: "Retained application".into(),
                job_url: String::new(),
                role_info: Default::default(),
                resume,
                change_points: Vec::new(),
                alerts: Vec::new(),
                alerts_truncated: false,
                dismissed_alert_ids: Vec::new(),
                ignore_all_alerts: false,
                cover_letter: entry.cover_letter,
                question: String::new(),
                answer: String::new(),
                approved_answers: Vec::new(),
                style: entry.style,
            },
            kind,
        )?;
        Ok((document, entry.style))
    });
    let (document, style) = match prepared {
        Ok(value) => value,
        Err(error) => return tracker_failure(&error),
    };
    match ort_render::render_pdf_with_style(&document, style) {
        Ok(pdf) => CommandResponse::success(application_materials::MaterialPdf {
            base64: STANDARD.encode(&pdf.bytes),
            filename: match kind {
                application_materials::MaterialKind::Resume => "tailored-resume.pdf",
                application_materials::MaterialKind::CoverLetter => "cover-letter.pdf",
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
            application_materials::load_for_tracker(store)?.ok_or(StorageError::NotFound)?;
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
            window.state::<application_materials::DragFiles>().clear();
            CommandResponse::success(true)
        }
        Err(error) => tracker_failure(&error),
    }
}

fn selected_snapshot(
    mut selection: FinishSelection,
    workspace: &application_materials::ApplicationWorkspace,
) -> Result<TrackerEntry, StorageError> {
    validate(&selection.entry)?;
    selection.entry.resume = selection.retain_resume.then(|| workspace.resume.clone());
    selection.entry.cover_letter = if selection.retain_cover_letter {
        workspace.cover_letter.clone()
    } else {
        None
    };
    selection.entry.cover_contact = selection
        .entry
        .cover_letter
        .as_ref()
        .map(|_| workspace.resume.contact.clone());
    selection.entry.answers = if selection.retain_answers {
        workspace.approved_answers.clone()
    } else {
        Vec::new()
    };
    selection.entry.style = workspace.style;
    if selection.entry.source_url.is_empty() {
        selection.entry.source_url = workspace.job_url.clone();
    }
    validate(&selection.entry)?;
    Ok(selection.entry)
}
