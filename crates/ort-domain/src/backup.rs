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
pub const RESTORE_CONFIRMATION_PHRASE: &str = "REPLACE SAVED PROFILE";
pub const ROLLBACK_CONFIRMATION_PHRASE: &str = "ROLL BACK SAVED PROFILE";
pub const DELETE_SAFETY_CONFIRMATION_PHRASE: &str = "DELETE SAFETY COPY";

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

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreBackupPayload {
    pub passphrase: String,
    pub confirmation: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreBackupRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: RestoreBackupPayload,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupRecoveryStatusPayload {}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupRecoveryStatusRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: BackupRecoveryStatusPayload,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SafetyCopyActionPayload {
    pub confirmation: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RollbackSafetyCopyRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: SafetyCopyActionPayload,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSafetyCopyRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: SafetyCopyActionPayload,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum RestoreBackupResponse {
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Staged {
        restart_required: bool,
        safety_copy_retained: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupRecoveryStatusResponse {
    pub safety_copy_available: bool,
    pub restart_operation_pending: bool,
    pub safety_cleanup_pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RollbackSafetyCopyResponse {
    pub restart_required: bool,
    pub current_profile_retained: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSafetyCopyResponse {
    pub deleted: bool,
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

impl RestoreBackupRequest {
    /// Validates the passphrase and exact destructive-action phrase before a
    /// native picker or expensive KDF operation is allowed.
    ///
    /// # Errors
    /// Rejects unsupported metadata, passphrases, or confirmation text.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        validate_passphrase(&self.payload.passphrase)?;
        if self.payload.confirmation != RESTORE_CONFIRMATION_PHRASE {
            return Err(ErrorEnvelope::new(
                "RESTORE_CONFIRMATION_REQUIRED",
                "errors.restoreConfirmation",
                false,
            ));
        }
        Ok(())
    }
}

impl BackupRecoveryStatusRequest {
    /// Validates the empty, content-free recovery-status request.
    ///
    /// # Errors
    /// Rejects unsupported request metadata.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)
    }
}

impl RollbackSafetyCopyRequest {
    /// Requires the exact rollback phrase before staging a restart operation.
    ///
    /// # Errors
    /// Rejects unsupported metadata or confirmation text.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_safety_action(
            self.contract_version,
            &self.request_id,
            &self.payload.confirmation,
            ROLLBACK_CONFIRMATION_PHRASE,
        )
    }
}

impl DeleteSafetyCopyRequest {
    /// Requires the exact permanent-deletion phrase before vault/filesystem work.
    ///
    /// # Errors
    /// Rejects unsupported metadata or confirmation text.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_safety_action(
            self.contract_version,
            &self.request_id,
            &self.payload.confirmation,
            DELETE_SAFETY_CONFIRMATION_PHRASE,
        )
    }
}

fn validate_safety_action(
    contract_version: u16,
    request_id: &str,
    confirmation: &str,
    expected: &str,
) -> Result<(), ErrorEnvelope> {
    validate_request_metadata(contract_version, request_id)?;
    if confirmation != expected {
        return Err(ErrorEnvelope::new(
            "SAFETY_COPY_CONFIRMATION_REQUIRED",
            "errors.safetyCopyConfirmation",
            false,
        ));
    }
    Ok(())
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

    #[test]
    fn restore_requires_the_exact_phrase_and_accepts_no_path_or_merge_controls() {
        let request = json!({
            "contractVersion": CONTRACT_VERSION,
            "requestId": "test-backup-restore-123",
            "payload": {
                "passphrase": "synthetic portable backup phrase",
                "confirmation": RESTORE_CONFIRMATION_PHRASE
            }
        });
        assert!(
            serde_json::from_value::<RestoreBackupRequest>(request.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for field in ["path", "merge", "destination", "profileId"] {
            let mut invalid = request.clone();
            invalid["payload"][field] = json!(true);
            assert!(serde_json::from_value::<RestoreBackupRequest>(invalid).is_err());
        }
        let mut wrong = request;
        wrong["payload"]["confirmation"] = json!("replace");
        assert!(
            serde_json::from_value::<RestoreBackupRequest>(wrong)
                .unwrap()
                .validate()
                .is_err()
        );
    }

    #[test]
    fn safety_copy_actions_are_empty_or_exactly_confirmed() {
        let status = json!({
            "contractVersion": CONTRACT_VERSION,
            "requestId": "test-recovery-status-123",
            "payload": {}
        });
        assert!(
            serde_json::from_value::<BackupRecoveryStatusRequest>(status)
                .unwrap()
                .validate()
                .is_ok()
        );
        for (phrase, rollback) in [
            (ROLLBACK_CONFIRMATION_PHRASE, true),
            (DELETE_SAFETY_CONFIRMATION_PHRASE, false),
        ] {
            let request = json!({
                "contractVersion": CONTRACT_VERSION,
                "requestId": "test-safety-action-123",
                "payload": {"confirmation": phrase}
            });
            if rollback {
                assert!(
                    serde_json::from_value::<RollbackSafetyCopyRequest>(request.clone())
                        .unwrap()
                        .validate()
                        .is_ok()
                );
            } else {
                assert!(
                    serde_json::from_value::<DeleteSafetyCopyRequest>(request.clone())
                        .unwrap()
                        .validate()
                        .is_ok()
                );
            }
            for field in ["path", "profileId", "deleteExternalBackups"] {
                let mut invalid = request.clone();
                invalid["payload"][field] = json!(true);
                if rollback {
                    assert!(serde_json::from_value::<RollbackSafetyCopyRequest>(invalid).is_err());
                } else {
                    assert!(serde_json::from_value::<DeleteSafetyCopyRequest>(invalid).is_err());
                }
            }
        }
    }
}
