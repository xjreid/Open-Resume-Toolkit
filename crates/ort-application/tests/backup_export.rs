//! Encrypted profile-to-portable-file-to-fresh-profile journey without native
//! vault or UI access.
use ort_backup::BackupPassphrase;
use ort_domain::ResumeDocument;
use ort_platform::{ExportDestination, ExportFileType, ExportWriteError};
use ort_storage::EncryptedStore;
use ort_vault::testing::MemoryDatabaseKeyVault;
use tempfile::TempDir;

const MARKER: &str = "SYNTHETIC_BACKUP_PRIVATE_MARKER_5fcba2";

#[test]
fn encrypted_backup_publishes_exact_bytes_and_restores_into_a_fresh_keyed_profile() {
    let source_root = TempDir::new().unwrap();
    let restored_root = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let source_vault = MemoryDatabaseKeyVault::new();
    let restored_vault = MemoryDatabaseKeyVault::new();
    let source =
        EncryptedStore::open_or_initialize(source_root.path(), "test", &source_vault).unwrap();
    let mut document = ResumeDocument::empty("Synthetic portable export");
    document.contact.full_name = MARKER.into();
    let draft = source.create_draft(&document).unwrap();
    let published = source.publish_draft(draft.revision).unwrap();
    let passphrase = BackupPassphrase::new("synthetic unique backup phrase".into()).unwrap();

    let bytes = source
        .create_portable_backup(&passphrase, "0.0.0-dev")
        .unwrap();
    assert!(
        !bytes
            .windows(MARKER.len())
            .any(|value| value == MARKER.as_bytes())
    );
    let path = output.path().join("profile.ort-backup");
    let receipt = ExportDestination::for_native_dialog(&path, ExportFileType::Backup)
        .unwrap()
        .write(&bytes)
        .unwrap();
    assert!(!receipt.cleanup_pending);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        ExportDestination::for_native_dialog(&path, ExportFileType::Backup).err(),
        Some(ExportWriteError::AlreadyExists)
    );

    let restored =
        EncryptedStore::open_or_initialize(restored_root.path(), "test", &restored_vault).unwrap();
    restored
        .restore_portable_backup(&std::fs::read(path).unwrap(), &passphrase)
        .unwrap();
    assert_eq!(restored.load_draft().unwrap().unwrap().document, document);
    assert_eq!(
        restored.load_latest_published().unwrap().unwrap().revision,
        published.revision
    );
    restored.verify_integrity().unwrap();
    assert_eq!(
        source.load_draft().unwrap().unwrap().revision,
        draft.revision
    );
}
