use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{EntityId, ErrorEnvelope, validate_request_metadata};

pub type CloseStatusRequest = crate::LoadResumeRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloseStatusResponse {
    pub pending_attempt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveCloseRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: ResolveClosePayload,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveClosePayload {
    pub attempt: String,
    pub decision: CloseDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseDecision {
    Cancel,
    Quit,
}

impl ResolveCloseRequest {
    /// Validates metadata and the bounded native-generated attempt identifier.
    ///
    /// # Errors
    /// Returns a safe contract error for invalid metadata or identifiers.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        if self.payload.attempt.len() != 36 || EntityId::parse(&self.payload.attempt).is_err() {
            return Err(ErrorEnvelope::new(
                "INVALID_CLOSE_ATTEMPT",
                "errors.invalidCloseAttempt",
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
    fn malformed_close_attempt_is_rejected() {
        let mut request = ResolveCloseRequest {
            contract_version: crate::CONTRACT_VERSION,
            request_id: "synthetic-request".to_owned(),
            payload: ResolveClosePayload {
                attempt: "not-a-native-attempt".to_owned(),
                decision: CloseDecision::Quit,
            },
        };
        assert!(request.validate().is_err());
        request.payload.attempt = EntityId::new().as_uuid().to_string();
        assert!(request.validate().is_ok());
        request.contract_version += 1;
        assert!(request.validate().is_err());
    }
}
