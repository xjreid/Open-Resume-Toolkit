//! Synthetic encrypted-profile journey; no OS vault, app or native dialog.
use ort_domain::ResumeDocument;
use ort_platform::{ExportDestination, ExportFileType, ExportWriteError};
use ort_render::{render_pdf, sha256};
use ort_storage::EncryptedStore;
use ort_vault::testing::MemoryDatabaseKeyVault;
use tempfile::TempDir;

#[test]
fn restart_render_and_export_preserve_exact_saved_and_published_revisions() {
    let profile = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store = EncryptedStore::open_or_initialize(profile.path(), "test", &vault).unwrap();
    let mut document = ResumeDocument::empty("Internal synthetic title");
    document.contact.full_name = "Synthetic Published".into();
    let saved = store.create_draft(&document).unwrap();
    let published = store.publish_draft(saved.revision).unwrap();
    document.contact.full_name = "Synthetic Later Draft".into();
    let draft = store.save_draft(saved.revision, &document).unwrap();
    drop(store);
    let store = EncryptedStore::open_or_initialize(profile.path(), "test", &vault).unwrap();
    let mut hashes = Vec::new();
    for (name, saved) in [
        ("draft.pdf", store.load_draft().unwrap().unwrap()),
        (
            "published.pdf",
            store.load_latest_published().unwrap().unwrap(),
        ),
    ] {
        let artifact = render_pdf(&saved.document).unwrap();
        assert_eq!(
            artifact.receipt.document_sha256,
            sha256(&serde_json::to_vec(&saved.document).unwrap())
        );
        let path = output.path().join(name);
        ExportDestination::for_native_dialog(&path, ExportFileType::Pdf)
            .unwrap()
            .write(&artifact.bytes)
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), artifact.bytes);
        assert_eq!(
            sha256(&std::fs::read(&path).unwrap()),
            artifact.receipt.pdf_sha256
        );
        assert!(matches!(
            ExportDestination::for_native_dialog(&path, ExportFileType::Pdf),
            Err(ExportWriteError::AlreadyExists)
        ));
        hashes.push(artifact.receipt.pdf_sha256);
    }
    assert_ne!(hashes[0], hashes[1]);
    let after = store.load_draft().unwrap().unwrap();
    let after_published = store.load_latest_published().unwrap().unwrap();
    assert_eq!(after.revision, draft.revision);
    assert_eq!(after.document, draft.document);
    assert_eq!(after_published.revision, published.revision);
    assert_eq!(after_published.document, published.document);
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 2);
}
