use super::*;

fn test_pdf_receipt() -> ort_domain::PdfRenderReceipt {
    ort_domain::PdfRenderReceipt {
        document_sha256: "a".repeat(64),
        document_schema_version: 1,
        pdf_sha256: "b".repeat(64),
        renderer_version: "test".into(),
        template_id: "test".into(),
        template_sha256: "c".repeat(64),
        font_bundle_id: "test".into(),
        font_bundle_sha256: "d".repeat(64),
        page_count: 1,
        byte_count: 3,
    }
}

#[cfg(target_os = "macos")]
#[test]
fn drag_files_are_private_and_finish_cleanup_waits_for_active_drag() {
    use std::os::unix::fs::PermissionsExt;

    let files = DragFiles::default();
    let (pdf, drag_lease) = files
        .materialize(b"%PDF-1.7\nfixture", "tailored-resume.pdf")
        .unwrap();
    assert_eq!(std::fs::read(&pdf).unwrap(), b"%PDF-1.7\nfixture");
    assert_eq!(
        std::fs::metadata(pdf.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(&pdf).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let session = pdf.parent().unwrap().parent().unwrap().to_path_buf();
    files.clear();
    assert!(session.exists());
    drop(drag_lease);
    assert!(!session.exists());
}

#[test]
fn prepared_exports_are_scoped_to_the_exact_workspace_revision_and_material() {
    let exports = ApplicationExportState::default();
    assert!(exports.replace(PreparedApplicationExports {
        profile_id: uuid::Uuid::nil(),
        revision: 9,
        kind: MaterialKind::Resume,
        pdf: b"pdf".to_vec(),
        receipt: test_pdf_receipt(),
        docx: b"docx".to_vec(),
    }));
    assert_eq!(
        exports.bytes(
            uuid::Uuid::nil(),
            9,
            MaterialKind::Resume,
            ApplicationExportFormat::Pdf
        ),
        Some(b"pdf".to_vec())
    );
    assert_eq!(
        exports.bytes(
            uuid::Uuid::nil(),
            9,
            MaterialKind::Resume,
            ApplicationExportFormat::Docx
        ),
        Some(b"docx".to_vec())
    );
    assert!(
        exports
            .bytes(
                uuid::Uuid::nil(),
                8,
                MaterialKind::Resume,
                ApplicationExportFormat::Pdf
            )
            .is_none()
    );
    assert!(
        exports
            .bytes(
                uuid::Uuid::nil(),
                9,
                MaterialKind::CoverLetter,
                ApplicationExportFormat::Pdf
            )
            .is_none()
    );
    assert!(exports.replace(PreparedApplicationExports {
        profile_id: uuid::Uuid::nil(),
        revision: 9,
        kind: MaterialKind::CoverLetter,
        pdf: b"cover-pdf".to_vec(),
        receipt: test_pdf_receipt(),
        docx: b"cover-docx".to_vec(),
    }));
    assert_eq!(
        exports.bytes(
            uuid::Uuid::nil(),
            9,
            MaterialKind::Resume,
            ApplicationExportFormat::Pdf
        ),
        Some(b"pdf".to_vec())
    );
    assert_eq!(
        exports.bytes(
            uuid::Uuid::nil(),
            9,
            MaterialKind::CoverLetter,
            ApplicationExportFormat::Docx
        ),
        Some(b"cover-docx".to_vec())
    );
    assert!(!exports.replace(PreparedApplicationExports {
        profile_id: uuid::Uuid::nil(),
        revision: 8,
        kind: MaterialKind::Resume,
        pdf: b"stale".to_vec(),
        receipt: test_pdf_receipt(),
        docx: b"stale".to_vec(),
    }));
    assert_eq!(
        exports.bytes(
            uuid::Uuid::nil(),
            9,
            MaterialKind::Resume,
            ApplicationExportFormat::Pdf
        ),
        Some(b"pdf".to_vec())
    );
}

#[test]
fn a_blank_section_preserves_printable_resume_and_prepared_downloads() {
    let mut prior = ResumeDocument::empty("Resume");
    prior.contact.full_name = "Alex Rivera".into();
    prior.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![ResumeEntry {
            id: EntityId::new(),
            order: 0,
            heading: "Engineer".into(),
            subheading: String::new(),
            date_range: String::new(),
            dates: None,
            location: String::new(),
            fields: vec![],
            bullets: vec![],
            links: vec![],
        }],
    });
    let mut edited = prior.clone();
    edited.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 1,
        heading: "Custom Section".into(),
        entries: vec![],
    });
    edited.validate(DocumentLimits::default()).unwrap();
    assert!(printable_resume_unchanged(&prior, &edited));
    assert_eq!(
        ort_render::render_pdf_with_style(&prior, DocumentStyle::Technical)
            .unwrap()
            .bytes,
        ort_render::render_pdf_with_style(&edited, DocumentStyle::Technical)
            .unwrap()
            .bytes
    );
    let temp = TempDir::new().unwrap();
    let store = ort_storage::EncryptedStore::open_or_initialize(
        temp.path(),
        "blank-section",
        &MemoryDatabaseKeyVault::new(),
    )
    .unwrap();
    let mut current = workspace();
    current.resume = prior.clone();
    let saved = save(&store, None, &current).unwrap();
    current.resume = edited.clone();
    let saved = save(&store, Some(saved.revision), &current).unwrap();
    assert_eq!(saved.workspace.resume, edited);

    let exports = ApplicationExportState::default();
    assert!(exports.replace(PreparedApplicationExports {
        profile_id: uuid::Uuid::nil(),
        revision: 7,
        kind: MaterialKind::Resume,
        pdf: b"existing pdf".to_vec(),
        receipt: test_pdf_receipt(),
        docx: b"existing docx".to_vec(),
    }));
    exports.promote_unchanged(uuid::Uuid::nil(), 7, 8, MaterialKind::Resume);
    assert!(exports.is_prepared(uuid::Uuid::nil(), 8, MaterialKind::Resume));
    assert_eq!(
        exports.bytes(
            uuid::Uuid::nil(),
            8,
            MaterialKind::Resume,
            ApplicationExportFormat::Pdf
        ),
        Some(b"existing pdf".to_vec())
    );
    assert!(!exports.is_prepared(uuid::Uuid::nil(), 7, MaterialKind::Resume));

    edited.sections[1].entries = prior.sections[0].entries.clone();
    edited.sections[1].entries[0].id = EntityId::new();
    assert!(!printable_resume_unchanged(&prior, &edited));
}

#[test]
fn prepared_pdf_and_docx_follow_the_saved_content_and_style() {
    let temp = TempDir::new().unwrap();
    let store = ort_storage::EncryptedStore::open_or_initialize(
        temp.path(),
        "exports",
        &MemoryDatabaseKeyVault::new(),
    )
    .unwrap();
    let mut current = workspace();
    current.resume.contact.full_name = "Alex Rivera".into();
    let first = save(&store, None, &current).unwrap();
    let state = DesktopState {
        storage: Mutex::new(crate::DesktopStorage::Ready(store)),
        reviews: Arc::new(crate::import_review::ReviewState::default()),
    };
    let original = render_application_exports(
        &state,
        state
            .with_store(|store| Ok(store.manifest().profile_id))
            .unwrap(),
        first.revision,
        MaterialKind::Resume,
    )
    .unwrap();
    assert!(original.pdf.starts_with(b"%PDF-"));
    assert!(original.docx.starts_with(b"PK\x03\x04"));
    current.resume.contact.full_name = "Alex Morgan".into();
    current.style = DocumentStyle::Modern;
    let edited = state
        .with_store(|store| save(store, Some(first.revision), &current))
        .unwrap();
    assert!(matches!(
        render_application_exports(
            &state,
            state
                .with_store(|store| Ok(store.manifest().profile_id))
                .unwrap(),
            first.revision,
            MaterialKind::Resume
        ),
        Err(StorageError::RevisionConflict)
    ));
    let updated = render_application_exports(
        &state,
        state
            .with_store(|store| Ok(store.manifest().profile_id))
            .unwrap(),
        edited.revision,
        MaterialKind::Resume,
    )
    .unwrap();
    assert_ne!(original.pdf, updated.pdf);
    assert_ne!(original.docx, updated.docx);
    let cover = render_application_exports(
        &state,
        state
            .with_store(|store| Ok(store.manifest().profile_id))
            .unwrap(),
        edited.revision,
        MaterialKind::CoverLetter,
    )
    .unwrap();
    assert!(cover.pdf.starts_with(b"%PDF-"));
    assert!(cover.docx.starts_with(b"PK\x03\x04"));
}

fn write_cover_preview_qa(name: &str, preview: &ort_domain::PdfPreviewResponse) {
    if let Some(directory) = std::env::var_os("ORT_COVER_PREVIEW_QA_DIRECTORY") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{name}.pdf")),
            STANDARD.decode(&preview.pdf_base64).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join(format!("{name}.json")),
            serde_json::to_vec(preview).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn cover_pdf_preview_matches_download_and_rejects_stale_revisions() {
    let temp = TempDir::new().unwrap();
    let store = ort_storage::EncryptedStore::open_or_initialize(
        temp.path(),
        "cover-preview",
        &MemoryDatabaseKeyVault::new(),
    )
    .unwrap();
    let mut current = workspace();
    current.resume.contact.full_name = "Alex Rivera".into();
    current.cover_letter = Some("Dear Hiring Team,\n\nI am applying for the engineer position. My documented Rust experience aligns with this role.\n\nThank you for considering my application.\n\nAlex Rivera".into());
    let saved = save(&store, None, &current).unwrap();
    let state = DesktopState {
        storage: Mutex::new(crate::DesktopStorage::Ready(store)),
        reviews: Arc::new(crate::import_review::ReviewState::default()),
    };
    let exports = ApplicationExportState::default();
    assert!(
        prepared_application_pdf(&state, &exports, saved.revision, MaterialKind::CoverLetter)
            .unwrap()
            .is_none()
    );
    let prepared = render_application_exports(
        &state,
        state
            .with_store(|store| Ok(store.manifest().profile_id))
            .unwrap(),
        saved.revision,
        MaterialKind::CoverLetter,
    )
    .unwrap();
    let receipt = prepared.receipt.clone();
    assert!(exports.replace(prepared));
    let preview =
        prepared_application_pdf(&state, &exports, saved.revision, MaterialKind::CoverLetter)
            .unwrap()
            .unwrap();
    assert_eq!(
        STANDARD.decode(&preview.pdf_base64).unwrap(),
        exports
            .bytes(
                state
                    .with_store(|store| Ok(store.manifest().profile_id))
                    .unwrap(),
                saved.revision,
                MaterialKind::CoverLetter,
                ApplicationExportFormat::Pdf
            )
            .unwrap()
    );
    assert_eq!(preview.receipt, receipt);
    assert_eq!(preview.revision, saved.revision);
    assert!(
        prepared_application_pdf(&state, &exports, saved.revision, MaterialKind::Resume)
            .unwrap()
            .is_none()
    );
    write_cover_preview_qa("cover-preview", &preview);
    current.cover_letter = Some("Edited cover letter".into());
    let updated = state
        .with_store(|store| save(store, Some(saved.revision), &current))
        .unwrap();
    assert!(matches!(
        prepared_application_pdf(&state, &exports, saved.revision, MaterialKind::CoverLetter),
        Err(StorageError::RevisionConflict)
    ));
    assert!(
        prepared_application_pdf(
            &state,
            &exports,
            updated.revision,
            MaterialKind::CoverLetter
        )
        .unwrap()
        .is_none()
    );
    let edited = render_application_exports(
        &state,
        state
            .with_store(|store| Ok(store.manifest().profile_id))
            .unwrap(),
        updated.revision,
        MaterialKind::CoverLetter,
    )
    .unwrap();
    assert!(exports.replace(edited));
    let edited_preview = prepared_application_pdf(
        &state,
        &exports,
        updated.revision,
        MaterialKind::CoverLetter,
    )
    .unwrap()
    .unwrap();
    assert_ne!(
        edited_preview.receipt.pdf_sha256,
        preview.receipt.pdf_sha256
    );
    write_cover_preview_qa("cover-preview-edited", &edited_preview);
}

#[test]
fn cover_letter_pdf_accepts_v2_contact_schema() {
    let mut workspace = workspace();
    workspace.resume = workspace.resume.upgraded_v2().unwrap();
    let document = document_for(
        &workspace.resume,
        workspace.cover_letter.as_deref(),
        MaterialKind::CoverLetter,
    )
    .unwrap();
    assert_eq!(document.schema_version, 2);
    assert!(document.sections[0].entries[0].dates.is_some());
    document.validate(DocumentLimits::default()).unwrap();
    preflight_pdf(uuid::Uuid::now_v7(), &workspace, MaterialKind::CoverLetter).unwrap();
}

#[test]
fn pdf_preflight_rejects_a_valid_resume_with_an_unsupported_glyph() {
    let mut workspace = workspace();
    workspace.resume.contact.full_name = "Alex 示例".into();
    workspace
        .resume
        .validate(DocumentLimits::default())
        .unwrap();
    assert_eq!(
        preflight_pdf(uuid::Uuid::now_v7(), &workspace, MaterialKind::Resume),
        Err("PDF_UNAVAILABLE")
    );
}

#[test]
fn fresh_profile_never_reads_or_promotes_retired_profile_exports() {
    let exports = ApplicationExportState::default();
    let old_profile = uuid::Uuid::now_v7();
    let fresh_profile = uuid::Uuid::now_v7();
    assert!(exports.replace(PreparedApplicationExports {
        profile_id: old_profile,
        revision: 9,
        kind: MaterialKind::Resume,
        pdf: b"old".to_vec(),
        docx: b"old".to_vec(),
        receipt: test_pdf_receipt()
    }));
    assert!(!exports.is_prepared(fresh_profile, 9, MaterialKind::Resume));
    assert!(
        exports
            .bytes(
                fresh_profile,
                9,
                MaterialKind::Resume,
                ApplicationExportFormat::Pdf
            )
            .is_none()
    );
    assert!(
        exports
            .preview(fresh_profile, 9, MaterialKind::Resume)
            .is_none()
    );
    exports.promote_unchanged(fresh_profile, 9, 10, MaterialKind::Resume);
    assert!(!exports.is_prepared(fresh_profile, 10, MaterialKind::Resume));
    assert!(exports.replace(PreparedApplicationExports {
        profile_id: fresh_profile,
        revision: 1,
        kind: MaterialKind::Resume,
        pdf: b"fresh".to_vec(),
        docx: b"fresh".to_vec(),
        receipt: test_pdf_receipt()
    }));
    assert_eq!(
        exports.bytes(
            fresh_profile,
            1,
            MaterialKind::Resume,
            ApplicationExportFormat::Pdf
        ),
        Some(b"fresh".to_vec())
    );
    assert!(!exports.is_prepared(old_profile, 9, MaterialKind::Resume));
    exports.clear();
    assert!(!exports.is_prepared(fresh_profile, 1, MaterialKind::Resume));
}

#[test]
fn profile_replacement_rejects_same_revision_cache_hits_with_real_artifacts() {
    let old_root = TempDir::new().unwrap();
    let new_root = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let old_store =
        ort_storage::EncryptedStore::open_or_initialize(old_root.path(), "test", &vault).unwrap();
    let mut old_source = workspace();
    old_source.resume.contact.full_name = "Old Synthetic Applicant".into();
    let old = save(&old_store, None, &old_source).unwrap();
    let state = DesktopState {
        storage: Mutex::new(crate::DesktopStorage::Ready(old_store)),
        reviews: Arc::default(),
    };
    let exports = ApplicationExportState::default();
    let old_render = render_application_exports(
        &state,
        state
            .with_store(|store| Ok(store.manifest().profile_id))
            .unwrap(),
        old.revision,
        MaterialKind::Resume,
    )
    .unwrap();
    let old_profile = old_render.profile_id;
    let old_bytes = old_render.pdf.clone();
    assert!(exports.replace(old_render));
    let fresh =
        ort_storage::EncryptedStore::open_or_initialize(new_root.path(), "test", &vault).unwrap();
    let mut new_source = old_source;
    new_source.resume.contact.full_name = "New Synthetic Applicant".into();
    let new = save(&fresh, None, &new_source).unwrap();
    assert_eq!(old.revision, new.revision);
    let new_profile = fresh.manifest().profile_id;
    state
        .replace_storage(crate::DesktopStorage::Ready(fresh))
        .unwrap();
    assert!(matches!(
        render_application_exports(&state, old_profile, new.revision, MaterialKind::Resume),
        Err(StorageError::RevisionConflict)
    ));
    assert!(
        prepared_application_pdf(&state, &exports, new.revision, MaterialKind::Resume)
            .unwrap()
            .is_none()
    );
    let new_render = render_application_exports(
        &state,
        state
            .with_store(|store| Ok(store.manifest().profile_id))
            .unwrap(),
        new.revision,
        MaterialKind::Resume,
    )
    .unwrap();
    assert_ne!(new_render.pdf, old_bytes);
    let new_bytes = new_render.pdf.clone();
    assert!(exports.replace(new_render));
    assert_eq!(
        exports.bytes(
            new_profile,
            new.revision,
            MaterialKind::Resume,
            ApplicationExportFormat::Pdf
        ),
        Some(new_bytes)
    );
}
