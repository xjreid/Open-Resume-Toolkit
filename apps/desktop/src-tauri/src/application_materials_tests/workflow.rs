use super::*;

#[test]
fn retaining_current_answer_keeps_only_the_final_revision() {
    let mut current = workspace();
    current.question = "Why this role?".into();
    current.answer = "Final edited answer".into();
    retain_current_answer(&mut current).unwrap();
    assert_eq!(current.approved_answers.len(), 1);
    assert_eq!(current.approved_answers[0].answer, "Final edited answer");
    assert!(current.answer.is_empty());
    retain_current_answer(&mut current).unwrap();
    assert_eq!(current.approved_answers.len(), 1);
}

#[test]
fn qualification_alerts_survive_refinement_and_encrypted_workspace_reload() {
    let mut current = workspace();
    current.job_description = "Required qualifications:\nPython\nR".into();
    current.resume.sections = vec![ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![ResumeEntry {
            id: EntityId::new(),
            order: 0,
            heading: "Engineer".into(),
            subheading: "Example Co".into(),
            date_range: "2021–2024".into(),
            dates: None,
            location: String::new(),
            fields: vec![],
            links: vec![],
            bullets: vec![Bullet {
                id: EntityId::new(),
                order: 0,
                text: "Built Rust tools.".into(),
            }],
        }],
    }];
    let source = current.resume.clone();
    let raw = json!({"schemaVersion":5,"tailoringPlan":["Preserve the published Rust tooling experience."],"roleInfo":null,
        "templateSections":[{"sectionId":source.sections[0].id,"entries":[{
            "entryId":source.sections[0].entries[0].id,"sourceEntryIds":[source.sections[0].entries[0].id],
            "mainInfo":{"format":"bullets","items":["Built Rust tools."]}
        }]}],"alerts":[
        {"kind":"not_found","category":"named_skill_or_technology","requirement":"Python","target":"Python","jobExcerpt":"Python","resumeEvidence":null},
        {"kind":"not_found","category":"named_skill_or_technology","requirement":"R","target":"R","jobExcerpt":"R","resumeEvidence":null}
    ]}).to_string();
    let mut alerts =
        materials::validate_template_tailoring(&source, &current.job_description, &raw, 1)
            .unwrap()
            .alerts;
    assert_eq!(alerts.len(), 2);
    let second = alerts.pop().unwrap();
    current.alerts = alerts;
    let first_id = current.alerts[0].id.clone();
    current.dismissed_alert_ids = vec![first_id.clone()];
    current.ignore_all_alerts = true;
    merge_refinement_alerts(&mut current, vec![], false);
    assert_eq!(current.alerts.len(), 1);
    // A regenerated duplicate has a new provider-side validation UUID;
    // retain the existing alert identity and its dismissal state.
    let mut duplicate = current.alerts[0].clone();
    duplicate.id = EntityId::new().to_string();
    merge_refinement_alerts(&mut current, vec![duplicate, second], false);
    assert_eq!(current.alerts.len(), 2);
    assert_eq!(current.alerts[0].id, first_id);
    assert_eq!(current.dismissed_alert_ids, vec![first_id]);
    assert!(current.ignore_all_alerts);
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let saved = save(&store, None, &current).unwrap();
    let loaded = load(&store).unwrap().unwrap();
    assert_eq!(loaded.revision, saved.revision);
    assert_eq!(
        serde_json::to_value(&loaded.workspace.alerts).unwrap(),
        serde_json::to_value(&current.alerts).unwrap()
    );
    assert_eq!(
        loaded.workspace.dismissed_alert_ids,
        current.dismissed_alert_ids
    );
    assert!(loaded.workspace.ignore_all_alerts);
}

#[test]
fn encrypted_workspace_persists_and_rejects_stale_writes() {
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let saved = save(&store, None, &workspace()).unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(
        load(&store).unwrap().unwrap().workspace.job_description,
        "Rust required"
    );
    assert_eq!(
        load(&store).unwrap().unwrap().workspace.role_info.company,
        "Example Company"
    );
    assert!(save(&store, Some(1), &workspace()).is_ok());
    assert!(matches!(
        save(&store, Some(1), &workspace()),
        Err(StorageError::RevisionConflict)
    ));
}

#[test]
fn edited_tracker_details_survive_restart_without_changing_capture_provenance() {
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let original = workspace();
    // Existing saved workspaces omit the new optional field.
    let legacy = serde_json::to_value(&original).unwrap();
    assert!(legacy.get("trackerMetadata").is_none());
    let mut edited: ApplicationWorkspace = serde_json::from_value(legacy).unwrap();
    let saved = save(&store, None, &original).unwrap();
    edited.tracker_metadata = Some(ort_domain::TrackerMetadata {
        company: "Edited company".into(),
        title: "Senior engineer".into(),
        location: "Boston".into(),
        date_applied: "2026-10-05".into(),
        status: "other".into(),
        custom_status: "Recruiter follow-up".into(),
        source_url: "Recruiter referral".into(),
    });
    edited.role_info.company = "Edited company".into();
    let saved = save_reviewed(&store, saved.revision, &edited).unwrap();
    drop(store);
    let reopened =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let loaded = load(&reopened).unwrap().unwrap();
    assert_eq!(loaded.revision, saved.revision);
    assert_eq!(
        serde_json::to_value(&loaded.workspace.tracker_metadata).unwrap(),
        serde_json::to_value(&edited.tracker_metadata).unwrap()
    );
    assert_eq!(loaded.workspace.job_url, original.job_url);
    assert_eq!(loaded.workspace.job_description, original.job_description);
    edited.job_url = "https://example.org/replaced-provenance".into();
    assert!(matches!(
        save_reviewed(&reopened, saved.revision, &edited),
        Err(StorageError::InvalidData)
    ));
    edited.job_url = original.job_url;
    edited.tracker_metadata.as_mut().unwrap().date_applied = "2026-02-30".into();
    assert!(matches!(
        save_reviewed(&reopened, saved.revision, &edited),
        Err(StorageError::InvalidData)
    ));
    edited.tracker_metadata.as_mut().unwrap().date_applied = "2026-10-05".into();
    edited.tracker_metadata.as_mut().unwrap().status = "applied".into();
    assert!(matches!(
        save_reviewed(&reopened, saved.revision, &edited),
        Err(StorageError::InvalidData)
    ));
}

#[test]
fn resume_prompt_contract_makes_headers_read_only_for_all_providers() {
    for instructions in [TAILOR_SYSTEM, REFINE_SYSTEM] {
        let prompt = resume_system(instructions);
        for region in [
            "section heading",
            "title",
            "role",
            "details",
            "date",
            "location",
            "extra",
            "mainInfo",
        ] {
            assert!(prompt.contains(region), "missing {region}");
        }
        assert!(prompt.contains("\"schemaVersion\":5"));
        assert!(prompt.contains("tailoringPlan"));
        assert!(prompt.contains("1–3"));
        assert!(prompt.contains("only editable entry region is mainInfo"));
        assert!(prompt.contains("Do not write or return any of these fields"));
        assert!(prompt.contains("Prioritize Education as the first resume section"));
        assert!(prompt.contains("strong, job-specific reason"));
        assert!(
            prompt.len() <= 12_000,
            "prompt exceeds material request bound"
        );
        assert!(prompt.contains("sourceEntryIds"));
        assert!(prompt.contains("Do not transplant accomplishments"));
    }
}

#[test]
fn reviewed_stage_one_survives_restart_and_rejects_stale_writes() {
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let draft = StageOneDraft {
        job_description: "Rust required".into(),
        job_url: "https://example.org/jobs/1".into(),
        style: DocumentStyle::Technical,
    };
    let first = save_stage_one(&store, None, &draft).unwrap();
    assert_eq!(first.revision, 1);
    drop(store);

    let reopened =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let restored = load_stage_one(&reopened).unwrap().unwrap();
    assert_eq!(restored.draft.job_description, draft.job_description);
    assert_eq!(restored.draft.job_url, draft.job_url);
    assert_eq!(restored.revision, first.revision);
    assert!(matches!(
        save_stage_one(&reopened, None, &draft),
        Err(StorageError::RevisionConflict)
    ));
    save_stage_one(&reopened, Some(restored.revision), &draft).unwrap();
    assert!(matches!(
        save_stage_one(&reopened, Some(restored.revision), &draft),
        Err(StorageError::RevisionConflict)
    ));
    save(&reopened, None, &workspace()).unwrap();
    assert!(matches!(
        save_stage_one(&reopened, Some(restored.revision + 1), &draft),
        Err(StorageError::RevisionConflict)
    ));
}

#[test]
fn stage_one_can_review_capture_larger_than_ai_input_without_sending_it() {
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let mut draft = StageOneDraft {
        job_description: "x".repeat(100_000),
        job_url: String::new(),
        style: DocumentStyle::Technical,
    };
    save_stage_one(&store, None, &draft).unwrap();
    assert_eq!(
        load_stage_one(&store)
            .unwrap()
            .unwrap()
            .draft
            .job_description
            .len(),
        100_000
    );
    draft.job_description = "x".repeat(MAX_CAPTURE_TEXT_BYTES + 1);
    assert!(matches!(
        save_stage_one(&store, Some(1), &draft),
        Err(StorageError::InvalidData)
    ));
}

#[test]
fn new_item_in_v2_resume_requires_an_empty_dates_array() {
    let mut workspace = workspace();
    workspace.resume.contact.full_name = "Alex Rivera".into();
    workspace.resume.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![],
    });
    workspace.resume = workspace.resume.upgraded_v2().unwrap();
    workspace.resume.sections[0].entries.push(ResumeEntry {
        id: EntityId::new(),
        order: 0,
        heading: String::new(),
        subheading: String::new(),
        date_range: String::new(),
        dates: None,
        location: String::new(),
        fields: vec![],
        bullets: vec![],
        links: vec![],
    });
    assert_eq!(
        preflight_pdf(&workspace, MaterialKind::Resume),
        Err("RESUME_INVALID")
    );
    workspace.resume.sections[0].entries[0].dates = Some(vec![]);
    preflight_pdf(&workspace, MaterialKind::Resume).unwrap();
    let temp = TempDir::new().unwrap();
    let store = ort_storage::EncryptedStore::open_or_initialize(
        temp.path(),
        "v2-new-item",
        &MemoryDatabaseKeyVault::new(),
    )
    .unwrap();
    let saved = save(&store, None, &workspace).unwrap();
    let state = DesktopState {
        storage: Mutex::new(crate::DesktopStorage::Ready(store)),
        reviews: Arc::new(crate::import_review::ReviewState::default()),
    };
    let exports = render_application_exports(&state, saved.revision, MaterialKind::Resume).unwrap();
    assert!(exports.pdf.starts_with(b"%PDF-"));
    assert!(exports.docx.starts_with(b"PK\x03\x04"));
}

#[test]
fn reviewed_generation_edit_and_pdf_render_journey() {
    use ort_domain::{Bullet, ResumeEntry};

    let mut source = ResumeDocument::empty("Published resume");
    source.contact.full_name = "Alex Rivera".into();
    let bullet = Bullet {
        id: EntityId::new(),
        order: 0,
        text: "Built Rust services".into(),
    };
    let entry = ResumeEntry {
        id: EntityId::new(),
        order: 0,
        heading: "Engineer".into(),
        subheading: "North Co".into(),
        date_range: "2021–2024".into(),
        dates: None,
        location: String::new(),
        fields: vec![],
        bullets: vec![bullet.clone()],
        links: vec![],
    };
    let section = ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![entry.clone()],
    };
    source.sections = vec![section.clone()];
    let proposal = json!({"schemaVersion":1,"selectedSections":[{
        "sectionId":section.id,"entries":[{"entryId":entry.id,"bulletIds":[bullet.id]}]
    }],"alerts":[]})
    .to_string();
    let generated = materials::validate_tailoring(&source, "Rust required", &proposal, 1).unwrap();
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let mut workspace = workspace();
    workspace.resume = generated.resume;
    workspace.change_points = generated.change_points;
    preflight_pdf(&workspace, MaterialKind::Resume).unwrap();
    let saved = save(&store, None, &workspace).unwrap();
    workspace.resume.sections[0].entries[0].bullets[0].text = "Built reliable Rust services".into();
    save(&store, Some(saved.revision), &workspace).unwrap();
    let recovered = load(&store).unwrap().unwrap();
    assert_eq!(
        recovered.workspace.resume.sections[0].entries[0].bullets[0].text,
        "Built reliable Rust services"
    );
    assert_eq!(
        source.sections[0].entries[0].bullets[0].text,
        "Built Rust services"
    );
    let pdf = ort_render::render_pdf_with_style(
        &document_for(
            &recovered.workspace.resume,
            recovered.workspace.cover_letter.as_deref(),
            MaterialKind::Resume,
        )
        .unwrap(),
        recovered.workspace.style,
    )
    .unwrap();
    assert!(pdf.bytes.starts_with(b"%PDF-"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn body_only_draft_with_plan_and_read_only_headers_preflights_and_renders() {
    let mut source = ResumeDocument::empty("Published resume");
    source.contact.full_name = "Alex Rivera".into();
    let entry = ResumeEntry {
        id: EntityId::new(),
        order: 0,
        heading: "Platform Engineer".into(),
        subheading: "North Co".into(),
        date_range: "2021–2024".into(),
        dates: None,
        location: "New York, NY".into(),
        fields: vec![
            NamedField {
                id: EntityId::new(),
                order: 0,
                label: "Details".into(),
                value: "Distributed systems".into(),
                is_skill: false,
            },
            NamedField {
                id: EntityId::new(),
                order: 1,
                label: "Extra".into(),
                value: "Rust, PostgreSQL".into(),
                is_skill: true,
            },
        ],
        bullets: vec![Bullet {
            id: EntityId::new(),
            order: 0,
            text: "Built dependable Rust services for customer workflows.".into(),
        }],
        links: vec![],
    };
    source.sections = vec![ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![entry.clone()],
    }];
    let draft = json!({
        "schemaVersion": 5,
        "tailoringPlan": [
            "Rust platform role — Built dependable Rust services — foreground the service delivery bullet.",
            "Distributed-systems need — source details name distributed systems — retain that context beside the role.",
            "Remote collaboration context — New York location is published — retain it accurately in metadata."
        ],
        "roleInfo": {"company":"Example Co","title":"Platform Engineer","location":"Remote"},
        "templateSections": [{
            "sectionId": source.sections[0].id,
            "entries": [{
                "entryId": entry.id,
                "sourceEntryIds": [entry.id],
                "mainInfo": {"format":"paragraph","items":["Built dependable Rust services for customer workflows."]}
            }]
        }],
        "alerts": [{"kind":"not_found","category":"named_skill_or_technology",
            "requirement":"Python","target":"Python","jobExcerpt":"Python","resumeEvidence":null}]
    })
    .to_string();
    let job = "Rust platform role\nRequired qualifications:\nPython";
    let generated = materials::validate_template_tailoring(&source, job, &draft, 1).unwrap();
    assert_eq!(generated.alerts.len(), 1);
    assert_eq!(generated.alerts[0].requirement, "Python");
    assert_eq!(
        generated.change_points,
        vec![
            "Rust platform role — Built dependable Rust services — foreground the service delivery bullet.",
            "Distributed-systems need — source details name distributed systems — retain that context beside the role.",
            "Remote collaboration context — New York location is published — retain it accurately in metadata."
        ]
    );
    let retained = &generated.resume.sections[0].entries[0];
    assert_eq!(retained.heading, entry.heading);
    assert_eq!(retained.subheading, entry.subheading);
    assert_eq!(retained.date_range, entry.date_range);
    assert_eq!(retained.location, entry.location);
    assert_eq!(&retained.fields[..2], entry.fields.as_slice());
    let mut generated_workspace = workspace();
    generated_workspace.resume = generated.resume;
    generated_workspace.role_info = generated.role_info.unwrap();
    generated_workspace.job_description = job.into();
    generated_workspace.alerts = generated.alerts;
    let temp = TempDir::new().unwrap();
    let store = ort_storage::EncryptedStore::open_or_initialize(
        temp.path(),
        "test",
        &MemoryDatabaseKeyVault::new(),
    )
    .unwrap();
    save(&store, None, &generated_workspace).unwrap();
    let serialized = serde_json::to_value(load(&store).unwrap().unwrap()).unwrap();
    assert_eq!(
        serialized["workspace"]["alerts"][0]["requirement"],
        "Python"
    );
    assert_eq!(serialized["workspace"]["alerts"][0]["jobExcerpt"], "Python");
    assert_eq!(serialized["workspace"]["alerts"][0]["target"], "Python");
    preflight_pdf(&generated_workspace, MaterialKind::Resume).unwrap();
    let pdf = ort_render::render_pdf_with_style(
        &document_for(
            &generated_workspace.resume,
            generated_workspace.cover_letter.as_deref(),
            MaterialKind::Resume,
        )
        .unwrap(),
        generated_workspace.style,
    )
    .unwrap();
    assert!(pdf.bytes.starts_with(b"%PDF-"));
    let docx = render_docx_with_style(
        &document_for(
            &generated_workspace.resume,
            generated_workspace.cover_letter.as_deref(),
            MaterialKind::Resume,
        )
        .unwrap(),
        generated_workspace.style,
    )
    .unwrap();
    assert!(docx.starts_with(b"PK\x03\x04"));
}
