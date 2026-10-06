use ort_backup::BackupPassphrase;
use ort_domain::{DocumentLimits, ResumeDocument};
use ort_storage::EncryptedStore;
use ort_vault::testing::MemoryDatabaseKeyVault;
use tempfile::TempDir;
#[test]
fn all_publications_round_trip_beyond_the_old_count_limit() {
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let mut draft = store
        .create_draft(&ResumeDocument::empty("Synthetic 0"))
        .unwrap();
    for i in 0..101 {
        if i > 0 {
            let mut document = draft.document.clone();
            document.title = format!("Synthetic {i}");
            draft = store.save_draft(draft.revision, &document).unwrap();
        }
        draft.document.validate(DocumentLimits::default()).unwrap();
        assert_eq!(store.publish_draft(draft.revision).unwrap().revision, i + 1);
    }
    let passphrase = BackupPassphrase::new("synthetic review passphrase".into()).unwrap();
    let bytes = store
        .create_portable_backup(&passphrase, "0.0.0-dev")
        .unwrap();
    let restored = ort_backup::restore_backup(&bytes, &passphrase).unwrap();
    assert_eq!(restored.profile.published_resumes.len(), 101);
    assert_eq!(
        restored
            .profile
            .published_resumes
            .last()
            .unwrap()
            .published_revision,
        101
    );
    assert_eq!(
        store.load_latest_published().unwrap().unwrap().revision,
        101
    );
}
