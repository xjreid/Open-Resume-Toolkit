//! Encrypted saved/published export journey without native vault or UI access.
use ort_documents::render_docx;
use ort_domain::ResumeDocument;
use ort_platform::{ExportDestination, ExportFileType, ExportWriteError};
use ort_storage::EncryptedStore;
use ort_vault::testing::MemoryDatabaseKeyVault;
use tempfile::TempDir;

#[test]
fn encrypted_restart_to_docx_preserves_saved_and_published_versions_and_no_clobber() {
    let profile = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store = EncryptedStore::open_or_initialize(profile.path(), "test", &vault).unwrap();
    let mut document = ResumeDocument::empty("Internal synthetic export title");
    document.contact.full_name = "SYNTHETIC_PUBLISHED".into();
    let saved = store.create_draft(&document).unwrap();
    let published = store.publish_draft(saved.revision).unwrap();
    document.contact.full_name = "SYNTHETIC_LATER_DRAFT".into();
    let draft = store.save_draft(saved.revision, &document).unwrap();
    drop(store);
    let store = EncryptedStore::open_or_initialize(profile.path(), "test", &vault).unwrap();
    for (name, value) in [
        ("draft.docx", store.load_draft().unwrap().unwrap()),
        (
            "published.docx",
            store.load_latest_published().unwrap().unwrap(),
        ),
    ] {
        let bytes = render_docx(&value.document).unwrap();
        let path = output.path().join(name);
        let receipt = ExportDestination::for_native_dialog(&path, ExportFileType::Docx)
            .unwrap()
            .write(&bytes)
            .unwrap();
        assert!(!receipt.cleanup_pending);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(
            bytes
                .windows(value.document.contact.full_name.len())
                .any(|w| w == value.document.contact.full_name.as_bytes())
        );
        assert_eq!(
            ExportDestination::for_native_dialog(&path, ExportFileType::Docx).err(),
            Some(ExportWriteError::AlreadyExists)
        );
    }
    let after = store.load_draft().unwrap().unwrap();
    let after_published = store.load_latest_published().unwrap().unwrap();
    assert_eq!(after.revision, draft.revision);
    assert_eq!(after.document, draft.document);
    assert_eq!(after_published.revision, published.revision);
    assert_eq!(after_published.document, published.document);
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 2);
}
