use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

mod backup;
mod lifecycle;
mod pdf;
mod resume;
mod resume_commands;
mod storage_usage;
mod text_export;

pub use backup::{
    BackupRecoveryStatusPayload, BackupRecoveryStatusRequest, BackupRecoveryStatusResponse,
    DELETE_SAFETY_CONFIRMATION_PHRASE, DeleteSafetyCopyRequest, DeleteSafetyCopyResponse,
    ExportBackupPayload, ExportBackupRequest, ExportBackupResponse, MAX_BACKUP_BYTES,
    MAX_BACKUP_PASSPHRASE_BYTES, RESTORE_CONFIRMATION_PHRASE, ROLLBACK_CONFIRMATION_PHRASE,
    RestoreBackupPayload, RestoreBackupRequest, RestoreBackupResponse, RollbackSafetyCopyRequest,
    RollbackSafetyCopyResponse, SafetyCopyActionPayload, ValidateBackupPayload,
    ValidateBackupRequest, ValidateBackupResponse,
};
pub use pdf::{
    MAX_PDF_BYTES, MAX_PDF_PAGES, MAX_PDF_RENDER_HISTORY, PDF_PREVIEW_TTL_SECONDS,
    PdfExportResponse, PdfPreviewResponse, PdfReleaseResponse, PdfRenderHistoryRequest,
    PdfRenderHistoryResponse, PdfRenderManifest, PdfRenderReceipt, PdfTicketPayload,
    PdfTicketRequest, RenderPdfRequest,
};

pub use text_export::{
    ExportDocxRequest, ExportDocxResponse, ExportSource, ExportTextPayload, ExportTextRequest,
    ExportTextResponse,
};

pub use lifecycle::{
    CloseDecision, CloseStatusRequest, CloseStatusResponse, ResolveClosePayload,
    ResolveCloseRequest,
};

pub use resume::{
    Bullet, ContactDetails, DocumentLimits, EntityId, Link, NamedField, ResumeDocument,
    ResumeEntry, ResumeSection, ValidationError,
};
pub use resume_commands::{
    EmptyPayload, LoadResumeRequest, PublishResumePayload, PublishResumeRequest,
    PublishResumeResponse, ResumeWorkspaceResponse, SaveResumePayload, SaveResumeRequest,
    VersionedResumeResponse,
};
pub use storage_usage::{StorageUsagePayload, StorageUsageRequest, StorageUsageResponse};

pub const CONTRACT_VERSION: u16 = 2;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HealthRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: HealthPayload,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HealthPayload {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub app_version: String,
    pub profile: RuntimeProfile,
    pub storage_status: StorageStatus,
    pub contract_version: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProfile {
    Development,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageStatus {
    Ready,
    DevelopmentGated,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEnvelope {
    pub code: String,
    pub message_key: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    pub details: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CommandResponse<T: Serialize> {
    Success { ok: bool, value: T },
    Failure { ok: bool, error: ErrorEnvelope },
}

impl<T: Serialize> CommandResponse<T> {
    #[must_use]
    pub const fn success(value: T) -> Self {
        Self::Success { ok: true, value }
    }

    #[must_use]
    pub fn failure(code: &str, message_key: &str, retryable: bool) -> Self {
        Self::Failure {
            ok: false,
            error: ErrorEnvelope::new(code, message_key, retryable),
        }
    }
}

impl ErrorEnvelope {
    #[must_use]
    pub fn new(code: &str, message_key: &str, retryable: bool) -> Self {
        Self {
            code: code.to_owned(),
            message_key: message_key.to_owned(),
            retryable,
            operation_id: None,
            details: Map::new(),
        }
    }
}

/// Validates metadata shared by every desktop command.
///
/// # Errors
/// Returns a bounded, non-sensitive contract error.
pub fn validate_request_metadata(
    contract_version: u16,
    request_id: &str,
) -> Result<(), ErrorEnvelope> {
    if contract_version != CONTRACT_VERSION {
        return Err(ErrorEnvelope::new(
            "UNSUPPORTED_CONTRACT_VERSION",
            "errors.unsupportedContractVersion",
            false,
        ));
    }

    if !(8..=64).contains(&request_id.len())
        || !request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(ErrorEnvelope::new(
            "INVALID_REQUEST_ID",
            "errors.invalidRequestId",
            false,
        ));
    }

    Ok(())
}

/// Validates the bounded fields shared by every health request.
///
/// # Errors
///
/// Returns a safe error envelope when the contract version is unsupported or
/// the request identifier is outside the accepted character and length bounds.
pub fn validate_health_request(request: &HealthRequest) -> Result<(), ErrorEnvelope> {
    validate_request_metadata(request.contract_version, &request.request_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_contract_version_drift() {
        let request = HealthRequest {
            contract_version: CONTRACT_VERSION + 1,
            request_id: "018f8b1b-50ad-7b4a-8f7d-38fd63e44086".to_owned(),
            payload: HealthPayload {},
        };

        let error = validate_health_request(&request).expect_err("version must be rejected");
        assert_eq!(error.code, "UNSUPPORTED_CONTRACT_VERSION");
    }

    #[test]
    fn accepts_a_bounded_request_identifier() {
        let request = HealthRequest {
            contract_version: CONTRACT_VERSION,
            request_id: "018f8b1b-50ad-7b4a-8f7d-38fd63e44086".to_owned(),
            payload: HealthPayload {},
        };

        assert!(validate_health_request(&request).is_ok());
    }
}
