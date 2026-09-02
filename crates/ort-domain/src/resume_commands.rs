use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ErrorEnvelope, ResumeDocument, validate_request_metadata};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadResumeRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: EmptyPayload,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyPayload {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveResumeRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: SaveResumePayload,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveResumePayload {
    pub expected_revision: Option<i64>,
    pub document: ResumeDocument,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishResumeRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: PublishResumePayload,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishResumePayload {
    pub expected_draft_revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionedResumeResponse {
    pub revision: i64,
    pub document: ResumeDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeWorkspaceResponse {
    pub draft: Option<VersionedResumeResponse>,
    pub latest_published: Option<VersionedResumeResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishResumeResponse {
    pub draft_revision: i64,
    pub published: VersionedResumeResponse,
}

impl LoadResumeRequest {
    /// Validates the shared bounded command metadata.
    ///
    /// # Errors
    /// Returns a safe contract error for unsupported versions or malformed IDs.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)
    }
}

impl SaveResumeRequest {
    /// Validates command metadata and optimistic revision bounds.
    ///
    /// # Errors
    /// Returns a safe error before any storage operation is attempted.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        if self
            .payload
            .expected_revision
            .is_some_and(|revision| revision < 1)
        {
            return Err(invalid_revision());
        }
        Ok(())
    }
}

impl PublishResumeRequest {
    /// Validates command metadata and the draft revision to publish.
    ///
    /// # Errors
    /// Returns a safe error before any storage operation is attempted.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        if self.payload.expected_draft_revision < 1 {
            return Err(invalid_revision());
        }
        Ok(())
    }
}

fn invalid_revision() -> ErrorEnvelope {
    ErrorEnvelope::new("INVALID_REVISION", "errors.invalidRevision", false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CONTRACT_VERSION;

    #[test]
    fn save_rejects_negative_revision_before_storage() {
        let request = SaveResumeRequest {
            contract_version: CONTRACT_VERSION,
            request_id: "018f8b1b-50ad-7b4a-8f7d-38fd63e44086".to_owned(),
            payload: SaveResumePayload {
                expected_revision: Some(-1),
                document: ResumeDocument::empty("Resume"),
            },
        };

        assert_eq!(
            request.validate().expect_err("invalid").code,
            "INVALID_REVISION"
        );
    }
}
