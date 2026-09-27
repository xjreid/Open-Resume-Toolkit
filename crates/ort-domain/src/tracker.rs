//! Shared tracker snapshot schema and validation for desktop commands and backups.
use crate::{ContactDetails, DocumentLimits, DocumentStyle, ResumeDocument};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovedAnswer {
    pub question: String,
    pub answer: String,
}

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

#[derive(Debug, thiserror::Error)]
#[error("invalid tracker snapshot")]
pub struct TrackerValidationError;

/// Validates retained materials and editable tracker metadata.
///
/// # Errors
/// Rejects invalid dates, oversized fields, and malformed retained materials.
pub fn validate_tracker_entry(entry: &TrackerEntry) -> Result<(), TrackerValidationError> {
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
        return Err(TrackerValidationError);
    }
    Ok(())
}
