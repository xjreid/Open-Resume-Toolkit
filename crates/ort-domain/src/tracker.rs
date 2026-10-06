//! Shared tracker snapshot schema and validation for desktop commands and backups.
use crate::{ContactDetails, DocumentLimits, DocumentStyle, ResumeDocument};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovedAnswer {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
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

/// Editable metadata, deliberately separate from immutable retained documents.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrackerMetadata {
    pub company: String,
    pub title: String,
    pub location: String,
    pub date_applied: String,
    pub status: String,
    pub custom_status: String,
    pub source_url: String,
}

impl TrackerMetadata {
    /// # Errors
    /// Rejects oversized fields, invalid dates, or inconsistent custom statuses.
    pub fn validate(&self) -> Result<(), TrackerValidationError> {
        validate_metadata(
            &self.company,
            &self.title,
            &self.location,
            &self.date_applied,
            &self.status,
            &self.custom_status,
            &self.source_url,
        )
    }

    /// Applies only editable fields; retained documents never cross this boundary.
    pub fn apply_to(self, entry: &mut TrackerEntry) {
        entry.company = self.company;
        entry.title = self.title;
        entry.location = self.location;
        entry.date_applied = self.date_applied;
        entry.status = self.status;
        entry.custom_status = self.custom_status;
        entry.source_url = self.source_url;
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid tracker snapshot")]
pub struct TrackerValidationError;

/// Validates retained materials and editable tracker metadata.
///
/// # Errors
/// Rejects invalid dates, oversized fields, and malformed retained materials.
pub fn validate_tracker_entry(entry: &TrackerEntry) -> Result<(), TrackerValidationError> {
    validate_metadata(
        &entry.company,
        &entry.title,
        &entry.location,
        &entry.date_applied,
        &entry.status,
        &entry.custom_status,
        &entry.source_url,
    )?;
    if entry
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

fn validate_metadata(
    company: &str,
    title: &str,
    location: &str,
    date_applied: &str,
    status: &str,
    custom_status: &str,
    source_url: &str,
) -> Result<(), TrackerValidationError> {
    if company.chars().count() > 200
        || title.chars().count() > 200
        || location.chars().count() > 200
        || date_applied.len() > 10
        || (!date_applied.is_empty()
            && jiff::civil::Date::strptime("%Y-%m-%d", date_applied).is_err())
        || status.is_empty()
        || status.len() > 40
        || !status
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        || custom_status.chars().count() > 80
        || (status != "other" && !custom_status.is_empty())
        || source_url.len() > 4096
    {
        return Err(TrackerValidationError);
    }
    Ok(())
}
