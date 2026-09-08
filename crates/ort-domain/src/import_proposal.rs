use crate::EntityId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportReviewSnapshot {
    pub id: String,
    pub base_revision: i64,
    pub mapping_version: u16,
    pub blocks: Vec<ImportReviewBlock>,
    pub sections: Vec<ImportReviewSection>,
    pub contacts: ImportReviewContacts,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportReviewBlock {
    pub source: String,
    pub page: u16,
    pub explanation: String,
    pub suggested_target: ImportReviewTarget,
    pub suggested_value: String,
    pub proposed_section: Option<usize>,
}
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImportReviewTarget {
    Section,
    Text,
    Bullet,
    FullName,
    Email,
    Phone,
    Location,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportReviewSection {
    pub id: EntityId,
    pub heading: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportReviewContacts {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub location: String,
}
