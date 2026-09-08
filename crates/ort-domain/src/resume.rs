use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

// New documents stay v1 until the structured editor is enabled. Readers support
// both versions; upgrades are explicit and never rewrite immutable sources.
pub const RESUME_SCHEMA_VERSION: u16 = 1;
pub const MAX_RESUME_DATES: usize = 200;

#[derive(Debug, Clone, Copy)]
pub struct DocumentLimits {
    pub total_characters: usize,
    pub sections: usize,
    pub entries: usize,
    pub bullets: usize,
    pub links: usize,
    pub skills: usize,
    pub field_characters: usize,
    pub bullet_characters: usize,
    pub serialized_bytes: usize,
}

impl Default for DocumentLimits {
    fn default() -> Self {
        Self {
            total_characters: 30_000,
            sections: 20,
            entries: 100,
            bullets: 500,
            links: 25,
            skills: 100,
            field_characters: 2_000,
            bullet_characters: 500,
            serialized_bytes: 512 * 1_024,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("the document schema version is unsupported")]
    UnsupportedSchema,
    #[error("an entity identifier is not UUIDv7")]
    InvalidEntityId,
    #[error("an entity identifier is duplicated")]
    DuplicateEntityId,
    #[error("an ordered collection is not in canonical order")]
    InvalidOrder,
    #[error("a document limit was exceeded")]
    LimitExceeded,
    #[error("a required field is empty")]
    EmptyRequiredField,
    #[error("a link uses an unsupported or invalid scheme")]
    InvalidLink,
    #[error("a structured date is invalid")]
    InvalidDate,
    #[error("fields do not match the document schema version")]
    SchemaMismatch,
    #[error("the serialized document is too large")]
    SerializedDocumentTooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct EntityId(Uuid);

impl EntityId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Parses a `UUIDv7` entity identifier at an import boundary.
    ///
    /// # Errors
    /// Returns `InvalidEntityId` for malformed or non-v7 UUIDs.
    pub fn parse(value: &str) -> Result<Self, ValidationError> {
        let uuid = Uuid::parse_str(value).map_err(|_| ValidationError::InvalidEntityId)?;
        if uuid.get_version_num() != 7 {
            return Err(ValidationError::InvalidEntityId);
        }
        Ok(Self(uuid))
    }

    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContactDetails {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeDocument {
    pub schema_version: u16,
    pub document_id: EntityId,
    pub title: String,
    pub contact: ContactDetails,
    pub sections: Vec<ResumeSection>,
}

impl ResumeDocument {
    #[must_use]
    pub fn empty(title: impl Into<String>) -> Self {
        Self {
            schema_version: RESUME_SCHEMA_VERSION,
            document_id: EntityId::new(),
            title: title.into(),
            contact: ContactDetails::default(),
            sections: Vec::new(),
        }
    }

    /// Creates a v2 copy without interpreting legacy date text or changing
    /// existing identities. Callers persist it only as an ordinary new revision.
    ///
    /// # Errors
    /// Rejects invalid input and a result that exceeds the serialized bound.
    pub fn upgraded_v2(&self) -> Result<Self, ValidationError> {
        self.validate(DocumentLimits::default())?;
        if self.schema_version == 2 {
            return Ok(self.clone());
        }
        let mut upgraded = self.clone();
        upgraded.schema_version = 2;
        for (index, link) in upgraded.contact.links.iter_mut().enumerate() {
            link.id = Some(EntityId::new());
            link.order = Some(u16::try_from(index).map_err(|_| ValidationError::LimitExceeded)?);
        }
        for section in &mut upgraded.sections {
            for entry in &mut section.entries {
                entry.dates = Some(Vec::new());
                for (index, link) in entry.links.iter_mut().enumerate() {
                    link.id = Some(EntityId::new());
                    link.order =
                        Some(u16::try_from(index).map_err(|_| ValidationError::LimitExceeded)?);
                }
            }
        }
        upgraded.validate(DocumentLimits::default())?;
        Ok(upgraded)
    }

    /// Validates identifiers, ordering, links, and all persistence bounds.
    ///
    /// # Errors
    /// Returns a stable, non-sensitive error category for invalid content.
    pub fn validate(&self, limits: DocumentLimits) -> Result<(), ValidationError> {
        if !matches!(self.schema_version, 1 | 2) {
            return Err(ValidationError::UnsupportedSchema);
        }
        if self.title.trim().is_empty() {
            return Err(ValidationError::EmptyRequiredField);
        }

        let mut identifiers = HashSet::new();
        check_identifier(self.document_id, &mut identifiers)?;
        let mut total_characters = bounded_characters(&self.title, limits.field_characters)?;
        total_characters += bounded_characters(&self.contact.full_name, limits.field_characters)?;
        total_characters += bounded_characters(&self.contact.email, limits.field_characters)?;
        total_characters += bounded_characters(&self.contact.phone, limits.field_characters)?;
        total_characters += bounded_characters(&self.contact.location, limits.field_characters)?;

        if self.sections.len() > limits.sections || self.contact.links.len() > limits.links {
            return Err(ValidationError::LimitExceeded);
        }

        let mut link_count = self.contact.links.len();
        for (index, link) in self.contact.links.iter().enumerate() {
            validate_link_identity(link, index, self.schema_version, &mut identifiers)?;
            total_characters += validate_link(link, limits.field_characters)?;
        }

        let mut entry_count = 0_usize;
        let mut bullet_count = 0_usize;
        let mut skill_count = 0_usize;
        let mut date_count = 0_usize;

        for (section_index, section) in self.sections.iter().enumerate() {
            if usize::from(section.order) != section_index {
                return Err(ValidationError::InvalidOrder);
            }
            check_identifier(section.id, &mut identifiers)?;
            if section.heading.trim().is_empty() {
                return Err(ValidationError::EmptyRequiredField);
            }
            total_characters += bounded_characters(&section.heading, limits.field_characters)?;
            entry_count = entry_count
                .checked_add(section.entries.len())
                .ok_or(ValidationError::LimitExceeded)?;

            for (entry_index, entry) in section.entries.iter().enumerate() {
                if usize::from(entry.order) != entry_index {
                    return Err(ValidationError::InvalidOrder);
                }
                check_identifier(entry.id, &mut identifiers)?;
                total_characters += bounded_characters(&entry.heading, limits.field_characters)?;
                total_characters += bounded_characters(&entry.subheading, limits.field_characters)?;
                total_characters += bounded_characters(&entry.date_range, limits.field_characters)?;
                total_characters += bounded_characters(&entry.location, limits.field_characters)?;

                bullet_count = bullet_count
                    .checked_add(entry.bullets.len())
                    .ok_or(ValidationError::LimitExceeded)?;
                link_count = link_count
                    .checked_add(entry.links.len())
                    .ok_or(ValidationError::LimitExceeded)?;

                for (field_index, field) in entry.fields.iter().enumerate() {
                    if usize::from(field.order) != field_index {
                        return Err(ValidationError::InvalidOrder);
                    }
                    check_identifier(field.id, &mut identifiers)?;
                    total_characters += bounded_characters(&field.label, limits.field_characters)?;
                    total_characters += bounded_characters(&field.value, limits.field_characters)?;
                    if field.is_skill {
                        skill_count += 1;
                    }
                }

                for (bullet_index, bullet) in entry.bullets.iter().enumerate() {
                    if usize::from(bullet.order) != bullet_index {
                        return Err(ValidationError::InvalidOrder);
                    }
                    check_identifier(bullet.id, &mut identifiers)?;
                    total_characters += bounded_characters(&bullet.text, limits.bullet_characters)?;
                }
                let (dates, characters) =
                    validate_dates(entry, self.schema_version, limits, &mut identifiers)?;
                date_count += dates;
                total_characters += characters;
                for (index, link) in entry.links.iter().enumerate() {
                    validate_link_identity(link, index, self.schema_version, &mut identifiers)?;
                    total_characters += validate_link(link, limits.field_characters)?;
                }
            }
        }

        if date_count > MAX_RESUME_DATES
            || entry_count > limits.entries
            || bullet_count > limits.bullets
            || link_count > limits.links
            || skill_count > limits.skills
            || total_characters > limits.total_characters
        {
            return Err(ValidationError::LimitExceeded);
        }

        let serialized =
            serde_json::to_vec(self).map_err(|_| ValidationError::SerializedDocumentTooLarge)?;
        if serialized.len() > limits.serialized_bytes {
            return Err(ValidationError::SerializedDocumentTooLarge);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeSection {
    pub id: EntityId,
    pub order: u16,
    pub heading: String,
    pub entries: Vec<ResumeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeEntry {
    pub id: EntityId,
    pub order: u16,
    pub heading: String,
    pub subheading: String,
    pub date_range: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_option"
    )]
    #[schemars(with = "Vec<crate::ResumeDate>")]
    pub dates: Option<Vec<crate::ResumeDate>>,
    pub location: String,
    pub fields: Vec<NamedField>,
    pub bullets: Vec<Bullet>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamedField {
    pub id: EntityId,
    pub order: u16,
    pub label: String,
    pub value: String,
    pub is_skill: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bullet {
    pub id: EntityId,
    pub order: u16,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Link {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_option"
    )]
    #[schemars(with = "EntityId")]
    pub id: Option<EntityId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_option"
    )]
    #[schemars(with = "u16")]
    pub order: Option<u16>,
    pub label: String,
    pub url: String,
}

fn validate_dates(
    entry: &ResumeEntry,
    version: u16,
    limits: DocumentLimits,
    identifiers: &mut HashSet<Uuid>,
) -> Result<(usize, usize), ValidationError> {
    match (version, &entry.dates) {
        (1, None) => Ok((0, 0)),
        (2, Some(dates)) => {
            if !dates.is_empty() && !entry.date_range.trim().is_empty() {
                return Err(ValidationError::SchemaMismatch);
            }
            if dates.len() > MAX_RESUME_DATES {
                return Err(ValidationError::LimitExceeded);
            }
            let mut characters = 0;
            for (index, date) in dates.iter().enumerate() {
                check_identifier(date.id, identifiers)?;
                if usize::from(date.order) != index {
                    return Err(ValidationError::InvalidOrder);
                }
                date.validate()?;
                characters += bounded_characters(&date.label, limits.field_characters)?;
            }
            Ok((dates.len(), characters))
        }
        _ => Err(ValidationError::SchemaMismatch),
    }
}

// Absence denotes a v1 field. Explicit null must not be discarded and silently
// change authenticated document hashes during deserialization.
fn present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn validate_link_identity(
    link: &Link,
    index: usize,
    version: u16,
    seen: &mut HashSet<Uuid>,
) -> Result<(), ValidationError> {
    match (version, link.id, link.order) {
        (1, None, None) => Ok(()),
        (2, Some(id), Some(order)) => {
            check_identifier(id, seen)?;
            if usize::from(order) != index {
                return Err(ValidationError::InvalidOrder);
            }
            Ok(())
        }
        _ => Err(ValidationError::SchemaMismatch),
    }
}

fn check_identifier(identifier: EntityId, seen: &mut HashSet<Uuid>) -> Result<(), ValidationError> {
    if identifier.as_uuid().get_version_num() != 7 {
        return Err(ValidationError::InvalidEntityId);
    }
    if !seen.insert(identifier.as_uuid()) {
        return Err(ValidationError::DuplicateEntityId);
    }
    Ok(())
}

fn bounded_characters(value: &str, maximum: usize) -> Result<usize, ValidationError> {
    let count = value.chars().count();
    if count > maximum {
        Err(ValidationError::LimitExceeded)
    } else {
        Ok(count)
    }
}

fn validate_link(link: &Link, maximum: usize) -> Result<usize, ValidationError> {
    let label_characters = bounded_characters(&link.label, maximum)?;
    let url_characters = bounded_characters(&link.url, maximum)?;
    let parsed = Url::parse(&link.url).map_err(|_| ValidationError::InvalidLink)?;
    if !matches!(parsed.scheme(), "http" | "https" | "mailto") {
        return Err(ValidationError::InvalidLink);
    }
    Ok(label_characters + url_characters)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{DocumentLimits, EntityId, ResumeDocument, ValidationError};

    #[test]
    fn empty_document_is_valid() {
        let document = ResumeDocument::empty("Master Resume");
        assert_eq!(document.validate(DocumentLimits::default()), Ok(()));
    }

    #[test]
    fn non_v7_document_identifier_is_rejected() {
        let mut document = ResumeDocument::empty("Master Resume");
        document.document_id = EntityId(Uuid::nil());
        assert_eq!(
            document.validate(DocumentLimits::default()),
            Err(ValidationError::InvalidEntityId)
        );
    }

    #[test]
    fn oversized_field_is_rejected() {
        let mut document = ResumeDocument::empty("Master Resume");
        document.title = "x".repeat(DocumentLimits::default().field_characters + 1);
        assert_eq!(
            document.validate(DocumentLimits::default()),
            Err(ValidationError::LimitExceeded)
        );
    }
}
