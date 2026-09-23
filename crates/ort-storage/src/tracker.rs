//! Encrypted application tracker records. The finish transaction commits the
//! selected snapshot and clears the temporary workspace as one unit.

use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[cfg(test)]
use crate::APPLICATION_CAPTURE_PENDING_SETTING_KEY;
use crate::{
    APPLICATION_STAGE_ONE_SETTING_KEY, APPLICATION_WORKSPACE_SETTING_KEY, EncryptedStore,
    StorageError, now_string,
};

const MAX_ENTRY_BYTES: usize = 1_024 * 1_024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackerRecord {
    pub id: String,
    pub revision: i64,
    pub value: Value,
}

fn encode(value: &Value) -> Result<Vec<u8>, StorageError> {
    let bytes = serde_json::to_vec(value).map_err(|_| StorageError::InvalidData)?;
    if bytes.len() > MAX_ENTRY_BYTES {
        return Err(StorageError::InvalidData);
    }
    Ok(bytes)
}

fn valid_id(id: &str) -> Result<(), StorageError> {
    if Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id) {
        Ok(())
    } else {
        Err(StorageError::InvalidData)
    }
}

impl EncryptedStore {
    /// Lists encrypted tracker entries for the active profile.
    /// # Errors
    /// Returns an error if storage or a stored record is invalid.
    pub fn tracker_list(&self) -> Result<Vec<TrackerRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let mut statement = connection
            .prepare("SELECT entry_id, revision, entry_json FROM tracker_entries WHERE profile_id = ?1 ORDER BY updated_at DESC, entry_id DESC")
            .map_err(|_| StorageError::Unavailable)?;
        statement
            .query_map([self.manifest.profile_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(|_| StorageError::Unavailable)?
            .map(|row| {
                let (id, revision, bytes) = row.map_err(|_| StorageError::Unavailable)?;
                valid_id(&id)?;
                let value =
                    serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidData)?;
                Ok(TrackerRecord {
                    id,
                    revision,
                    value,
                })
            })
            .collect()
    }

    /// Creates or replaces one tracker entry with an optimistic revision check.
    /// # Errors
    /// Returns a conflict when the entry changed or already exists.
    pub fn tracker_save(
        &self,
        id: &str,
        expected: Option<i64>,
        value: &Value,
    ) -> Result<TrackerRecord, StorageError> {
        valid_id(id)?;
        let bytes = encode(value)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let now = now_string();
        let revision = if let Some(current) = expected {
            let next = current.checked_add(1).ok_or(StorageError::InvalidData)?;
            if current < 1 || connection.execute(
                "UPDATE tracker_entries SET revision = ?1, entry_json = ?2, updated_at = ?3 WHERE profile_id = ?4 AND entry_id = ?5 AND revision = ?6",
                params![next, bytes, now, profile, id, current],
            ).map_err(|_| StorageError::Unavailable)? != 1 {
                return Err(StorageError::RevisionConflict);
            }
            next
        } else {
            match connection.execute(
                "INSERT INTO tracker_entries (entry_id, profile_id, revision, entry_json, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?4, ?4)",
                params![id, profile, bytes, now],
            ) {
                Ok(1) => 1,
                Err(error) if super::is_constraint_error(&error) => return Err(StorageError::RevisionConflict),
                _ => return Err(StorageError::Unavailable),
            }
        };
        Ok(TrackerRecord {
            id: id.into(),
            revision,
            value: value.clone(),
        })
    }

    /// Deletes an entry only when its loaded revision still matches.
    /// # Errors
    /// Returns a conflict when the entry was already changed or removed.
    pub fn tracker_delete(&self, id: &str, expected: i64) -> Result<(), StorageError> {
        valid_id(id)?;
        if expected < 1 {
            return Err(StorageError::InvalidData);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        if connection.execute(
            "DELETE FROM tracker_entries WHERE profile_id = ?1 AND entry_id = ?2 AND revision = ?3",
            params![self.manifest.profile_id.to_string(), id, expected],
        ).map_err(|_| StorageError::Unavailable)? != 1 {
            return Err(StorageError::RevisionConflict);
        }
        Ok(())
    }

    /// Saves a selected tracker snapshot and clears the temporary workspace in
    /// one SQL transaction. A failed insert or stale revision leaves both intact.
    /// # Errors
    /// Returns a conflict for stale workspace state or a duplicate entry ID.
    pub fn tracker_finish_workspace(
        &self,
        workspace_revision: i64,
        entry: Option<(&str, &Value)>,
    ) -> Result<(), StorageError> {
        if workspace_revision < 1 {
            return Err(StorageError::InvalidData);
        }
        let encoded = entry
            .map(|(id, value)| {
                valid_id(id)?;
                Ok((id, encode(value)?))
            })
            .transpose()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let current = transaction.query_row(
            "SELECT value_json FROM settings WHERE profile_id = ?1 AND setting_key = ?2 AND revision = ?3",
            params![profile, APPLICATION_WORKSPACE_SETTING_KEY, workspace_revision],
            |row| row.get::<_, Vec<u8>>(0),
        ).optional().map_err(|_| StorageError::Unavailable)?;
        let Some(current) = current else {
            return Err(StorageError::RevisionConflict);
        };
        if current == b"null" {
            return Err(StorageError::RevisionConflict);
        }
        if let Some((id, bytes)) = encoded {
            match transaction.execute(
                "INSERT INTO tracker_entries (entry_id, profile_id, revision, entry_json, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?4, ?4)",
                params![id, profile, bytes, now_string()],
            ) {
                Ok(1) => {},
                Err(error) if super::is_constraint_error(&error) => return Err(StorageError::RevisionConflict),
                _ => return Err(StorageError::Unavailable),
            }
        }
        let next = workspace_revision
            .checked_add(1)
            .ok_or(StorageError::InvalidData)?;
        if transaction.execute(
            "UPDATE settings SET revision = ?1, value_json = ?2, updated_at = ?3 WHERE profile_id = ?4 AND setting_key = ?5 AND revision = ?6",
            params![next, b"null", now_string(), profile, APPLICATION_WORKSPACE_SETTING_KEY, workspace_revision],
        ).map_err(|_| StorageError::Unavailable)? != 1 {
            return Err(StorageError::RevisionConflict);
        }
        transaction.execute(
            "UPDATE settings SET revision = revision + 1, value_json = ?1, updated_at = ?2 WHERE profile_id = ?3 AND setting_key = ?4",
            params![b"null", now_string(), profile, APPLICATION_STAGE_ONE_SETTING_KEY],
        ).map_err(|_| StorageError::Unavailable)?;
        transaction.commit().map_err(|_| StorageError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_backup::BackupPassphrase;
    use ort_vault::testing::MemoryDatabaseKeyVault;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn finish_is_atomic_and_preserves_workspace_after_failure() {
        let root = TempDir::new().unwrap();
        let store =
            EncryptedStore::open_or_initialize(root.path(), "test", &MemoryDatabaseKeyVault::new())
                .unwrap();
        let workspace = store
            .save_setting(
                APPLICATION_WORKSPACE_SETTING_KEY,
                None,
                &json!({"job": "synthetic"}),
            )
            .unwrap();
        let stage_one = store
            .save_setting(
                APPLICATION_STAGE_ONE_SETTING_KEY,
                None,
                &json!({"jobDescription":"synthetic"}),
            )
            .unwrap();
        let id = Uuid::now_v7().to_string();
        let entry = json!({"company": "Synthetic"});
        store.tracker_save(&id, None, &entry).unwrap();
        assert_eq!(
            store.tracker_finish_workspace(workspace.revision, Some((&id, &entry))),
            Err(StorageError::RevisionConflict)
        );
        assert_eq!(
            store
                .load_setting(APPLICATION_WORKSPACE_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value,
            workspace.value
        );
        assert_eq!(store.tracker_list().unwrap().len(), 1);
        assert_eq!(
            store
                .load_setting(APPLICATION_STAGE_ONE_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value,
            stage_one.value
        );
        let new_id = Uuid::now_v7().to_string();
        store
            .tracker_finish_workspace(workspace.revision, Some((&new_id, &entry)))
            .unwrap();
        assert_eq!(store.tracker_list().unwrap().len(), 2);
        assert!(
            store
                .load_setting(APPLICATION_WORKSPACE_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value
                .is_null()
        );
        assert!(
            store
                .load_setting(APPLICATION_STAGE_ONE_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value
                .is_null()
        );
        assert_eq!(
            store.tracker_finish_workspace(workspace.revision, None),
            Err(StorageError::RevisionConflict)
        );
    }

    #[test]
    fn capture_resolution_preserves_pending_on_conflict_and_commits_both_changes() {
        let root = TempDir::new().unwrap();
        let store =
            EncryptedStore::open_or_initialize(root.path(), "test", &MemoryDatabaseKeyVault::new())
                .unwrap();
        let pending = store
            .save_setting(
                APPLICATION_CAPTURE_PENDING_SETTING_KEY,
                None,
                &json!({"requestId":"synthetic", "text":"new"}),
            )
            .unwrap();
        let current = store
            .save_setting(
                APPLICATION_STAGE_ONE_SETTING_KEY,
                None,
                &json!({"jobDescription":"old"}),
            )
            .unwrap();
        assert_eq!(
            store.resolve_application_capture(
                pending.revision,
                Some((
                    APPLICATION_STAGE_ONE_SETTING_KEY,
                    Some(current.revision + 1),
                    &json!({"jobDescription":"new"})
                )),
            ),
            Err(StorageError::RevisionConflict)
        );
        assert_eq!(
            store
                .load_setting(APPLICATION_CAPTURE_PENDING_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value,
            pending.value
        );
        assert_eq!(
            store
                .load_setting(APPLICATION_STAGE_ONE_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value,
            current.value
        );
        store
            .resolve_application_capture(
                pending.revision,
                Some((
                    APPLICATION_STAGE_ONE_SETTING_KEY,
                    Some(current.revision),
                    &json!({"jobDescription":"new"}),
                )),
            )
            .unwrap();
        assert!(
            store
                .load_setting(APPLICATION_CAPTURE_PENDING_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value
                .is_null()
        );
        assert_eq!(
            store
                .load_setting(APPLICATION_STAGE_ONE_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value["jobDescription"],
            "new"
        );
    }

    #[test]
    fn entries_survive_reopen_and_enforce_revisions() {
        let root = TempDir::new().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(root.path(), "test", &vault).unwrap();
        let id = Uuid::now_v7().to_string();
        let first = store
            .tracker_save(&id, None, &json!({"company":"A"}))
            .unwrap();
        assert_eq!(
            store.tracker_save(&id, None, &json!({"company":"B"})).err(),
            Some(StorageError::RevisionConflict)
        );
        let second = store
            .tracker_save(&id, Some(first.revision), &json!({"company":"B"}))
            .unwrap();
        assert_eq!(
            store.tracker_delete(&id, first.revision),
            Err(StorageError::RevisionConflict)
        );
        drop(store);
        let reopened = EncryptedStore::open_or_initialize(root.path(), "test", &vault).unwrap();
        assert_eq!(
            reopened.tracker_list().unwrap()[0].value,
            json!({"company":"B"})
        );
        reopened.tracker_delete(&id, second.revision).unwrap();
        assert!(reopened.tracker_list().unwrap().is_empty());
    }

    #[test]
    fn portable_backup_retains_tracker_snapshots() {
        let root = TempDir::new().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let source =
            EncryptedStore::open_or_initialize(&root.path().join("source"), "test", &vault)
                .unwrap();
        let id = Uuid::now_v7().to_string();
        let value = json!({"company":"Synthetic","resume":{"title":"Retained synthetic"}});
        source.tracker_save(&id, None, &value).unwrap();
        let pending = json!({"requestId":"synthetic","text":"Private pending selection"});
        source
            .save_setting(APPLICATION_CAPTURE_PENDING_SETTING_KEY, None, &pending)
            .unwrap();
        let passphrase = BackupPassphrase::new("synthetic tracker backup phrase".into()).unwrap();
        let bytes = source
            .create_portable_backup(&passphrase, "0.0.0-dev")
            .unwrap();
        assert!(
            !bytes
                .windows(b"Retained synthetic".len())
                .any(|part| part == b"Retained synthetic")
        );
        let destination =
            EncryptedStore::open_or_initialize(&root.path().join("destination"), "test", &vault)
                .unwrap();
        destination
            .restore_portable_backup(&bytes, &passphrase)
            .unwrap();
        assert_eq!(destination.tracker_list().unwrap()[0].value, value);
        assert_eq!(
            destination
                .load_setting(APPLICATION_CAPTURE_PENDING_SETTING_KEY)
                .unwrap()
                .unwrap()
                .value,
            pending
        );
    }
}
