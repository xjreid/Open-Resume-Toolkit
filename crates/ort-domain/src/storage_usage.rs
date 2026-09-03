use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ErrorEnvelope, validate_request_metadata};

pub const DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE: &str = "DELETE ALL LOCAL ORT DATA";

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageUsagePayload {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageUsageRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: StorageUsagePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageUsageResponse {
    pub database_schema: u16,
    pub drafts: u32,
    pub published_snapshots: u32,
    pub settings: u32,
    pub render_manifests: u32,
    pub diagnostic_events: u32,
    pub database_bytes: u64,
    pub wal_bytes: u64,
    pub shared_memory_bytes: u64,
    pub manifest_bytes: u64,
    pub recovery_metadata_bytes: u64,
    pub total_profile_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteAllLocalDataPayload {
    pub confirmation: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteAllLocalDataRequest {
    pub contract_version: u16,
    pub request_id: String,
    pub payload: DeleteAllLocalDataPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeleteAllLocalDataResponse {
    #[serde(rename_all = "camelCase")]
    Deleted { fresh_profile_ready: bool },
    #[serde(rename_all = "camelCase")]
    CleanupPending { restart_required: bool },
}

impl StorageUsageRequest {
    /// Validates the content-free storage inventory request.
    ///
    /// # Errors
    /// Returns a safe contract error for unsupported metadata.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)
    }
}

impl DeleteAllLocalDataRequest {
    /// Requires an exact destructive-action phrase before any storage or vault
    /// mutation. Paths, profile identities and category selectors are never
    /// accepted from the webview.
    ///
    /// # Errors
    /// Rejects unsupported request metadata or confirmation text.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)?;
        if self.payload.confirmation != DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE {
            return Err(ErrorEnvelope::new(
                "DELETE_ALL_CONFIRMATION_REQUIRED",
                "errors.deleteAllConfirmation",
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
    fn storage_usage_request_accepts_no_paths_or_categories() {
        let request = json!({
            "contractVersion": CONTRACT_VERSION,
            "requestId": "test-storage-usage-123",
            "payload": {}
        });
        assert!(
            serde_json::from_value::<StorageUsageRequest>(request.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for field in ["path", "profileId", "includeContent", "vacuum"] {
            let mut invalid = request.clone();
            invalid["payload"][field] = json!(true);
            assert!(serde_json::from_value::<StorageUsageRequest>(invalid).is_err());
        }
    }

    #[test]
    fn delete_all_requires_the_exact_phrase_and_accepts_no_target_controls() {
        let request = json!({
            "contractVersion": CONTRACT_VERSION,
            "requestId": "test-delete-all-local-data-123",
            "payload": {"confirmation": DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE}
        });
        assert!(
            serde_json::from_value::<DeleteAllLocalDataRequest>(request.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for field in [
            "path",
            "profileId",
            "categories",
            "deleteExternalBackups",
            "keepSafetyCopy",
        ] {
            let mut invalid = request.clone();
            invalid["payload"][field] = json!(true);
            assert!(serde_json::from_value::<DeleteAllLocalDataRequest>(invalid).is_err());
        }
        let mut wrong = request;
        wrong["payload"]["confirmation"] = json!("delete everything");
        assert!(
            serde_json::from_value::<DeleteAllLocalDataRequest>(wrong)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
