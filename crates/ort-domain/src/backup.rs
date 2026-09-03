use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ErrorEnvelope, validate_request_metadata};

/// Maximum portable backup bytes accepted or published by native file boundaries.
///
/// The encrypted payload is capped at 64 MiB. The remaining allowance covers
/// the format's bounded clear header and authentication tag without coupling
/// the filesystem adapter to private container-layout constants.
pub const MAX_BACKUP_BYTES: usize = 64 * 1_024 * 1_024 + 128 + 16;
pub const MAX_BACKUP_PASSPHRASE_BYTES: usize = 1_024;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportBackupPayload {
    pub passphrase: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportBackupRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: ExportBackupPayload,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidateBackupPayload {
    pub passphrase: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidateBackupRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: ValidateBackupPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportBackupResponse {
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Exported {
        byte_count: usize,
        format_major: u16,
        format_minor: u16,
        cleanup_pending: bool,
        durability_unconfirmed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValidateBackupResponse {
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Validated {
        byte_count: usize,
        format_major: u16,
        format_minor: u16,
        app_version: String,
        database_schema: u16,
        document_schema: u16,
        created_at: String,
        master_drafts: u16,
        published_resumes: u16,
        settings: u16,
        render_manifests: u16,
    },
}

impl ExportBackupRequest {
    /// Validates bounded metadata and passphrase size before any KDF or dialog.
    /// The passphrase itself is never returned in a response or error detail.
    ///
    /// # Errors
    /// Rejects unsupported metadata and empty or oversized passphrases.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        validate_passphrase(&self.payload.passphrase)
    }
}

impl ValidateBackupRequest {
    /// Validates bounded metadata and passphrase size before opening a dialog.
    /// The passphrase itself is never returned in a response or error detail.
    ///
    /// # Errors
    /// Rejects unsupported metadata and empty or oversized passphrases.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        validate_passphrase(&self.payload.passphrase)
    }
}

fn validate_passphrase(passphrase: &str) -> Result<(), ErrorEnvelope> {
    if passphrase.is_empty() || passphrase.len() > MAX_BACKUP_PASSPHRASE_BYTES {
        return Err(ErrorEnvelope::new(
            "INVALID_BACKUP_PASSPHRASE",
            "errors.invalidBackupPassphrase",
            false,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CONTRACT_VERSION;
    use serde_json::json;

    #[test]
    fn backup_boundary_accepts_only_a_bounded_passphrase() {
        let request = json!({
            "contractVersion": CONTRACT_VERSION,
            "requestId": "test-backup-export-123",
            "payload": {"passphrase": "synthetic portable backup phrase"}
        });
        assert!(
            serde_json::from_value::<ExportBackupRequest>(request.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for field in ["path", "overwrite", "profile", "databaseKey"] {
            let mut invalid = request.clone();
            invalid["payload"][field] = json!("untrusted");
            assert!(serde_json::from_value::<ExportBackupRequest>(invalid).is_err());
        }
        for passphrase in [String::new(), "x".repeat(MAX_BACKUP_PASSPHRASE_BYTES + 1)] {
            let mut invalid = request.clone();
            invalid["payload"]["passphrase"] = json!(passphrase);
            assert!(
                serde_json::from_value::<ExportBackupRequest>(invalid)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
    }

    #[test]
    fn backup_validation_boundary_never_accepts_a_path_or_restore_flag() {
        let request = json!({
            "contractVersion": CONTRACT_VERSION,
            "requestId": "test-backup-validation-123",
            "payload": {"passphrase": "synthetic portable backup phrase"}
        });
        assert!(
            serde_json::from_value::<ValidateBackupRequest>(request.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for field in ["path", "replace", "profileId", "destination"] {
            let mut invalid = request.clone();
            invalid["payload"][field] = json!("untrusted");
            assert!(serde_json::from_value::<ValidateBackupRequest>(invalid).is_err());
        }
    }
}
