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
    let newer_published = store.publish_draft(draft.revision).unwrap();
    assert_ne!(newer_published.revision, published.revision);
    let retained = store
        .load_published_revision(published.revision)
        .unwrap()
        .expect("older immutable publication remains retained");
    assert_eq!(retained, published);
    let retained_artifact = render_pdf(&retained.document).unwrap();
    assert_eq!(retained_artifact.receipt.pdf_sha256, hashes[1]);
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

#[test]
fn schema_upgrade_retains_publications_and_round_trips_mixed_versions_through_backup() {
    use ort_domain::{CalendarDate, EntityId, ResumeDate, ResumeEntry, ResumeSection};
    let profile = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let vault = MemoryDatabaseKeyVault::default();
    let mut document = ResumeDocument::empty("Synthetic version upgrade");
    document.contact.full_name = "Synthetic Person".into();
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Education".into(),
        entries: vec![ResumeEntry {
            id: EntityId::new(),
            order: 0,
            heading: "Example school".into(),
            subheading: String::new(),
            date_range: "Approximate date".into(),
            dates: None,
            location: String::new(),
            fields: vec![],
            bullets: vec![],
            links: vec![],
        }],
    });
    let store = EncryptedStore::open_or_initialize(profile.path(), "test", &vault).unwrap();
    let original = store.create_draft(&document).unwrap();
    let publication = store.publish_draft(original.revision).unwrap();
    let legacy_pdf = render_pdf(&publication.document).unwrap();
    let mut upgraded = document.upgraded_v2().unwrap();
    upgraded.sections[0].entries[0].date_range.clear();
    upgraded.sections[0].entries[0].dates = Some(vec![ResumeDate {
        id: EntityId::new(),
        order: 0,
        label: "Graduation".into(),
        start: Some(CalendarDate {
            year: 2027,
            month: Some(6),
            expected: true,
        }),
        end: None,
    }]);
    let saved = store.save_draft(original.revision, &upgraded).unwrap();
    assert!(
        store.save_draft(saved.revision, &document).is_err(),
        "a v1 client cannot discard v2 data"
    );
    let text = ort_documents::render_plain_text(&upgraded).unwrap();
    assert!(text.contains("Graduation: Expected Jun 2027"));
    for style in [
        ort_domain::DocumentStyle::Plain,
        ort_domain::DocumentStyle::Technical,
        ort_domain::DocumentStyle::Professional,
        ort_domain::DocumentStyle::Modern,
    ] {
        assert_eq!(
            ort_render::render_pdf_with_style(&upgraded, style)
                .unwrap()
                .receipt
                .document_schema_version,
            2
        );
        assert!(ort_documents::render_docx_with_style(&upgraded, style).is_ok());
    }
    let passphrase =
        ort_backup::BackupPassphrase::new("synthetic version two backup".into()).unwrap();
    let backup = store
        .create_portable_backup(&passphrase, "0.0.0-test")
        .unwrap();
    assert_eq!(ort_backup::inspect_backup(&backup).unwrap().format_minor, 2);
    let decoded = ort_backup::restore_backup(&backup, &passphrase).unwrap();
    assert_eq!(decoded.manifest.document_schema, 2);
    assert_eq!(decoded.profile.master_draft.unwrap().document, upgraded);
    drop(store);
    let reopened = EncryptedStore::open_or_initialize(profile.path(), "test", &vault).unwrap();
    assert_eq!(reopened.load_draft().unwrap().unwrap(), saved);
    let retained = reopened
        .load_published_revision(publication.revision)
        .unwrap()
        .unwrap();
    assert_eq!(retained, publication);
    assert_eq!(
        render_pdf(&retained.document).unwrap().receipt,
        legacy_pdf.receipt
    );
    let restored = EncryptedStore::open_or_initialize(destination.path(), "test", &vault).unwrap();
    restored
        .restore_portable_backup(&backup, &passphrase)
        .unwrap();
    assert_eq!(restored.load_draft().unwrap().unwrap(), saved);
    assert_eq!(
        restored
            .load_published_revision(publication.revision)
            .unwrap()
            .unwrap(),
        publication
    );
}
