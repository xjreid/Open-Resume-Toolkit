use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ErrorEnvelope, ExportSource, ExportTextPayload, ExportTextRequest};

/// Reviewed, bundled presentation choices. This is never a path or template source.
/// Plain remains the default for requests created before style selection existed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStyle {
    #[default]
    Plain,
    Technical,
    Professional,
    Modern,
}

impl DocumentStyle {
    #[must_use]
    pub const fn pdf_template_id(self) -> &'static str {
        match self {
            Self::Plain => "plain_pdf_v1",
            Self::Technical => "technical_pdf_v1",
            Self::Professional => "professional_pdf_v1",
            Self::Modern => "modern_pdf_v1",
        }
    }

    #[must_use]
    pub const fn docx_template_id(self) -> &'static str {
        match self {
            Self::Plain => "plain_docx_v1",
            Self::Technical => "technical_docx_v1",
            Self::Professional => "professional_docx_v1",
            Self::Modern => "modern_docx_v1",
        }
    }

    #[must_use]
    pub fn from_pdf_template_id(id: &str) -> Option<Self> {
        match id {
            "plain_pdf_v1" => Some(Self::Plain),
            "technical_pdf_v1" => Some(Self::Technical),
            "professional_pdf_v1" => Some(Self::Professional),
            "modern_pdf_v1" => Some(Self::Modern),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyledExportPayload {
    pub source: ExportSource,
    pub expected_revision: i64,
    #[serde(default)]
    pub style: DocumentStyle,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyledExportRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: StyledExportPayload,
}

impl StyledExportRequest {
    #[must_use]
    pub fn saved_selection(&self) -> ExportTextRequest {
        ExportTextRequest {
            contract_version: self.contract_version,
            request_id: self.request_id.clone(),
            payload: ExportTextPayload {
                source: self.payload.source,
                expected_revision: self.payload.expected_revision,
            },
        }
    }

    /// # Errors
    /// Rejects invalid metadata and saved revisions before rendering or dialogs.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        self.saved_selection().validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn styles_are_optional_but_only_bundled_choices_cross_the_boundary() {
        let request = json!({"contractVersion": crate::CONTRACT_VERSION,
            "requestId": "synthetic-style", "payload": {
                "source": "saved_draft", "expectedRevision": 1}});
        let legacy: StyledExportRequest = serde_json::from_value(request.clone()).unwrap();
        assert_eq!(legacy.payload.style, DocumentStyle::Plain);
        assert!(legacy.validate().is_ok());
        for style in ["plain", "technical", "professional", "modern"] {
            let mut styled = request.clone();
            styled["payload"]["style"] = json!(style);
            assert!(
                serde_json::from_value::<StyledExportRequest>(styled)
                    .unwrap()
                    .validate()
                    .is_ok()
            );
        }
        for style in [
            json!("../template.typ"),
            json!("#read(\"secret\")"),
            json!(null),
            json!({"name": "modern"}),
        ] {
            let mut invalid = request.clone();
            invalid["payload"]["style"] = style;
            assert!(serde_json::from_value::<StyledExportRequest>(invalid).is_err());
        }
        for field in ["path", "template", "document", "font"] {
            let mut invalid = request.clone();
            invalid["payload"][field] = json!("untrusted");
            assert!(serde_json::from_value::<StyledExportRequest>(invalid).is_err());
        }
        let mut invalid = request;
        invalid["payload"]["expectedRevision"] = json!(0);
        assert!(
            serde_json::from_value::<StyledExportRequest>(invalid)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
