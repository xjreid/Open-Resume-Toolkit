//! Synthetic encrypted-profile journey; no OS vault, app or native dialog.
use ort_domain::{ExportSource, ResumeDocument};
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
    for (index, (name, source, saved)) in [
        (
            "draft.pdf",
            ExportSource::SavedDraft,
            store.load_draft().unwrap().unwrap(),
        ),
        (
            "published.pdf",
            ExportSource::PublishedSnapshot,
            store.load_latest_published().unwrap().unwrap(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
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
        store
            .record_render_manifest(
                source,
                saved.revision,
                1_000 + u64::try_from(index).unwrap(),
                &artifact.receipt,
            )
            .unwrap();
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
    drop(store);
    let reopened = EncryptedStore::open_or_initialize(profile.path(), "test", &vault).unwrap();
    let manifests = reopened.load_recent_render_manifests(20).unwrap();
    assert_eq!(manifests.len(), 2);
    assert_eq!(manifests[0].source, ExportSource::PublishedSnapshot);
    assert_eq!(manifests[1].source, ExportSource::SavedDraft);
    assert_eq!(
        [
            manifests[1].receipt.pdf_sha256.clone(),
            manifests[0].receipt.pdf_sha256.clone(),
        ],
        [hashes[0].clone(), hashes[1].clone()]
    );
}
