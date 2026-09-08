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

// Legacy requests and plain receipts retain their original wire shape. Styled
// DOCX receipts add the exact bundled template identity; format v1 still denotes
// the constrained OPC format, with no implicit change to the old plain output.
pub type ExportDocxRequest = crate::StyledExportRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportDocxResponse {
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Exported {
        source: ExportSource,
        revision: i64,
        byte_count: usize,
        format_version: u16,
        cleanup_pending: bool,
        durability_unconfirmed: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        template_id: Option<String>,
    },
}

impl ExportDocxResponse {
    #[must_use]
    pub fn from_export(value: &ExportTextResponse, style: crate::DocumentStyle) -> Self {
        match *value {
            ExportTextResponse::Cancelled => Self::Cancelled,
            ExportTextResponse::Exported {
                source,
                revision,
                byte_count,
                format_version,
                cleanup_pending,
                durability_unconfirmed,
            } => Self::Exported {
                source,
                revision,
                byte_count,
                format_version,
                cleanup_pending,
                durability_unconfirmed,
                template_id: (style != crate::DocumentStyle::Plain)
                    .then(|| style.docx_template_id().to_owned()),
            },
        }
    }
}

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
    fn docx_plain_receipts_keep_the_legacy_shape_and_styles_name_the_template() {
        let legacy = ExportTextResponse::Exported {
            source: ExportSource::SavedDraft,
            revision: 2,
            byte_count: 123,
            format_version: 1,
            cleanup_pending: true,
            durability_unconfirmed: false,
        };
        assert_eq!(
            serde_json::to_value(ExportDocxResponse::from_export(
                &legacy,
                crate::DocumentStyle::Plain
            ))
            .unwrap(),
            serde_json::to_value(&legacy).unwrap()
        );
        for style in [
            crate::DocumentStyle::Technical,
            crate::DocumentStyle::Professional,
            crate::DocumentStyle::Modern,
        ] {
            let mut expected = serde_json::to_value(&legacy).unwrap();
            expected["templateId"] = json!(style.docx_template_id());
            assert_eq!(
                serde_json::to_value(ExportDocxResponse::from_export(&legacy, style)).unwrap(),
                expected
            );
            assert_eq!(
                serde_json::to_value(ExportDocxResponse::from_export(
                    &ExportTextResponse::Cancelled,
                    style
                ))
                .unwrap(),
                json!({"status": "cancelled"})
            );
        }
    }

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
