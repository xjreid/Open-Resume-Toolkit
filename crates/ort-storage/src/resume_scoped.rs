//! Resume writes bind optimistic revisions to the profile that supplied them.
use crate::{EncryptedStore, StorageError, VersionedResume};
use ort_domain::ResumeDocument;
use uuid::Uuid;
impl EncryptedStore {
    /// Checks the stable profile identity shared by every scoped workflow.
    /// # Errors
    /// Rejects a request belonging to a retired or foreign profile.
    pub fn ensure_profile(&self, expected: Uuid) -> Result<(), StorageError> {
        if self.manifest.profile_id == expected {
            Ok(())
        } else {
            Err(StorageError::RevisionConflict)
        }
    }

    /// Saves or creates only in the profile loaded by the editor.
    /// # Errors
    /// Rejects a retired profile identity before reading or writing resume data.
    pub fn save_resume_for_profile(
        &self,
        expected_profile: Uuid,
        expected_revision: Option<i64>,
        document: &ResumeDocument,
    ) -> Result<VersionedResume, StorageError> {
        self.ensure_profile(expected_profile)?;
        match expected_revision {
            Some(revision) => self.save_draft(revision, document),
            None => self.create_draft(document),
        }
    }
    /// Publishes only in the profile loaded by the editor.
    /// # Errors
    /// Rejects foreign profiles and stale draft revisions.
    pub fn publish_resume_for_profile(
        &self,
        expected_profile: Uuid,
        expected_revision: i64,
    ) -> Result<VersionedResume, StorageError> {
        self.ensure_profile(expected_profile)?;
        self.publish_draft(expected_revision)
    }
}
