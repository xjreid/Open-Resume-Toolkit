use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

pub const RESUME_SCHEMA_VERSION: u16 = 1;

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

    /// Validates identifiers, ordering, links, and all persistence bounds.
    ///
    /// # Errors
    /// Returns a stable, non-sensitive error category for invalid content.
    pub fn validate(&self, limits: DocumentLimits) -> Result<(), ValidationError> {
        if self.schema_version != RESUME_SCHEMA_VERSION {
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
        for link in &self.contact.links {
            total_characters += validate_link(link, limits.field_characters)?;
        }

        let mut entry_count = 0_usize;
        let mut bullet_count = 0_usize;
        let mut skill_count = 0_usize;

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
                for link in &entry.links {
                    total_characters += validate_link(link, limits.field_characters)?;
                }
            }
        }

        if entry_count > limits.entries
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
    pub label: String,
    pub url: String,
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
