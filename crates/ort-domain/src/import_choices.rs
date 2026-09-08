//! Bounded review decisions only: no source text, owner, path or storage authority.
use crate::EntityId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_IMPORT_CHOICES_BYTES: usize = 512 * 1024;
pub const MAX_IMPORT_CHOICES: usize = 1_000;
pub const MAX_IMPORT_REVIEW_CHARACTERS: usize = 100_000;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportChoices {
    pub choices: Vec<ImportChoice>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImportChoice {
    Pending {},
    Reject {},
    Section {
        heading: String,
        target: ImportSectionTarget,
    },
    Contact {
        field: ImportContactField,
        value: String,
        mode: ImportContactMode,
    },
    Text {
        text: String,
        bullet: bool,
        target: ImportTextTarget,
    },
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImportSectionTarget {
    New {},
    Existing { id: EntityId },
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImportTextTarget {
    Proposed { index: usize },
    Existing { id: EntityId },
    New { heading: String },
}
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImportContactField {
    FullName,
    Email,
    Phone,
    Location,
}
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImportContactMode {
    FillEmpty,
    Replace,
    KeepExisting,
}
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid or oversized import review decisions")]
pub struct InvalidImportChoices;

impl ImportChoices {
    /// Decode before mutating native review state. This does not authenticate a
    /// session or prove the decisions match its retained source-block count.
    /// # Errors
    /// Rejects oversized bytes, unknown fields/variants and character limits.
    pub fn decode(bytes: &[u8]) -> Result<Self, InvalidImportChoices> {
        if bytes.len() > MAX_IMPORT_CHOICES_BYTES {
            return Err(InvalidImportChoices);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| InvalidImportChoices)?;
        value.validate()?;
        Ok(value)
    }
    /// # Errors
    /// Rejects item/character limits even for native-constructed values.
    pub fn validate(&self) -> Result<(), InvalidImportChoices> {
        if self.choices.len() > MAX_IMPORT_CHOICES {
            return Err(InvalidImportChoices);
        }
        let mut total = 0usize;
        for choice in &self.choices {
            let texts: [&str; 2] = match choice {
                ImportChoice::Pending {} | ImportChoice::Reject {} => ["", ""],
                ImportChoice::Section { heading, .. } => [heading, ""],
                ImportChoice::Contact { value, .. } => [value, ""],
                ImportChoice::Text { text, target, .. } => {
                    if let ImportTextTarget::Proposed { index } = target
                        && *index >= MAX_IMPORT_CHOICES
                    {
                        return Err(InvalidImportChoices);
                    }
                    [
                        text,
                        if let ImportTextTarget::New { heading } = target {
                            heading
                        } else {
                            ""
                        },
                    ]
                }
            };
            for text in texts {
                for character in text.chars() {
                    if character.is_control() && !matches!(character, '\n' | '\r' | '\t') {
                        return Err(InvalidImportChoices);
                    }
                    total += 1;
                    if total > MAX_IMPORT_REVIEW_CHARACTERS {
                        return Err(InvalidImportChoices);
                    }
                }
            }
        }
        Ok(())
    }
}
