use crate::{EntityId, ErrorEnvelope, MAX_IMPORT_CHOICES_BYTES, validate_request_metadata};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BeginImportRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: BeginImportPayload,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BeginImportPayload {
    pub expected_revision: Option<i64>,
}
impl BeginImportRequest {
    /// # Errors
    /// Refuses invalid metadata or a non-positive saved-draft revision.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        if self
            .payload
            .expected_revision
            .is_some_and(|revision| revision < 1)
        {
            return Err(ErrorEnvelope::new(
                "IMPORT_REVIEW_INVALID",
                "errors.importReviewInvalid",
                false,
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportReviewRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: ImportReviewPayload,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportReviewPayload {
    pub review_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyImportReviewRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: ApplyImportReviewPayload,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyImportReviewPayload {
    pub review_id: String,
    /// Encoded decision-only payload, bounded before its native decode.
    pub decisions_json: String,
}
fn identifier(value: &str) -> Result<(), ErrorEnvelope> {
    if value.len() != 36 || EntityId::parse(value).is_err() {
        return Err(ErrorEnvelope::new(
            "IMPORT_REVIEW_UNAVAILABLE",
            "errors.importReviewUnavailable",
            false,
        ));
    }
    Ok(())
}
impl ImportReviewRequest {
    /// # Errors
    /// Refuses invalid metadata or review identifiers.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        identifier(&self.payload.review_id)
    }
}
impl ApplyImportReviewRequest {
    /// # Errors
    /// Refuses invalid metadata, identifiers or oversized decision bytes.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        identifier(&self.payload.review_id)?;
        if self.payload.decisions_json.len() > MAX_IMPORT_CHOICES_BYTES {
            return Err(ErrorEnvelope::new(
                "IMPORT_REVIEW_INVALID",
                "errors.importReviewInvalid",
                false,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_command_metadata_ids_and_bytes_are_bounded() {
        let mut request = ApplyImportReviewRequest {
            contract_version: crate::CONTRACT_VERSION,
            request_id: "synthetic-review-request".into(),
            payload: ApplyImportReviewPayload {
                review_id: EntityId::new().as_uuid().to_string(),
                decisions_json: "{\"choices\":[]}".into(),
            },
        };
        assert!(request.validate().is_ok());
        request.contract_version += 1;
        assert!(request.validate().is_err());
        request.contract_version = crate::CONTRACT_VERSION;
        request.payload.decisions_json = " ".repeat(MAX_IMPORT_CHOICES_BYTES + 1);
        assert!(request.validate().is_err());
        request.payload.decisions_json.clear();
        request.payload.review_id = "../other-review".into();
        assert!(request.validate().is_err());
        assert!(serde_json::from_str::<ImportReviewRequest>(r#"{"contractVersion":2,"requestId":"synthetic","payload":{"reviewId":"x","owner":"main"}}"#).is_err());
    }
}
