use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ErrorEnvelope, validate_request_metadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportSource {
    SavedDraft,
    PublishedSnapshot,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportTextPayload {
    pub source: ExportSource,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportTextRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: ExportTextPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportTextResponse {
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Exported {
        source: ExportSource,
        revision: i64,
        byte_count: usize,
        format_version: u16,
        cleanup_pending: bool,
        durability_unconfirmed: bool,
    },
}

// DOCX uses the same path-free saved-revision selection and receipt structure.
// Its command and separate response validator fix format v1 = plain_docx_v1,
// with a 2 MiB bound (text remains format v1 with a 256 KiB bound).
pub type ExportDocxRequest = ExportTextRequest;
pub type ExportDocxResponse = ExportTextResponse;

impl ExportTextRequest {
    /// Validates metadata and a JavaScript-safe saved revision. No path or
    /// renderer-provided document is part of this command contract.
    ///
    /// # Errors
    /// Rejects unsupported metadata or out-of-range revisions before any dialog.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        if !(1..=9_007_199_254_740_991).contains(&self.payload.expected_revision) {
            return Err(ErrorEnvelope::new(
                "INVALID_REVISION",
                "errors.invalidRevision",
                false,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CONTRACT_VERSION;
    use serde_json::json;

    #[test]
    fn no_path_content_or_unknown_source_crosses_export_boundary() {
        let request = json!({"contractVersion": CONTRACT_VERSION, "requestId": "test-export-123",
            "payload": {"source": "saved_draft", "expectedRevision": 1}});
        assert!(
            serde_json::from_value::<ExportTextRequest>(request.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for field in ["path", "document", "overwrite"] {
            let mut invalid = request.clone();
            invalid["payload"][field] = json!("untrusted");
            assert!(serde_json::from_value::<ExportTextRequest>(invalid).is_err());
        }
        for revision in [0, -1, i64::MAX] {
            let mut invalid = request.clone();
            invalid["payload"]["expectedRevision"] = json!(revision);
            assert!(
                serde_json::from_value::<ExportTextRequest>(invalid)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        let mut invalid = request;
        invalid["payload"]["source"] = json!("unsaved_editor");
        assert!(serde_json::from_value::<ExportTextRequest>(invalid).is_err());
    }
}
