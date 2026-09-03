use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ErrorEnvelope, validate_request_metadata};

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

impl StorageUsageRequest {
    /// Validates the content-free storage inventory request.
    ///
    /// # Errors
    /// Returns a safe contract error for unsupported metadata.
    pub fn validate(&self) -> Result<(), ErrorEnvelope> {
        validate_request_metadata(self.contract_version, &self.request_id)
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
}
