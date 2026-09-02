use ort_application::import_review::{
    ContactMode, ImportReview, MAX_REVIEW_CHARACTERS, ReviewDecision, ReviewError, SectionTarget,
    TextTarget,
};
use ort_documents::{
    import::{ContactField, ImportProposal, InputFormat, ValidatedExtraction},
    render_plain_text,
};
use ort_domain::{
    DocumentLimits, EntityId, ResumeDocument, ResumeSection, VersionedResumeResponse,
};
use serde_json::json;

fn proposal(blocks: &[(&str, &str)]) -> ImportProposal {
    let blocks: Vec<_> = blocks
        .iter()
        .map(|(kind, text)| json!({"page":1,"kind":kind,"text":text}))
        .collect();
    let bytes =
        serde_json::to_vec(&json!({"version":1,"format":"docx","pageCount":1,"blocks":blocks}))
            .unwrap();
    ImportProposal::map(ValidatedExtraction::decode(&bytes, InputFormat::Docx).unwrap())
}

fn base() -> VersionedResumeResponse {
    let mut document = ResumeDocument::empty("Synthetic review base");
    "Existing Person".clone_into(&mut document.contact.full_name);
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".to_owned(),
        entries: vec![],
    });
    VersionedResumeResponse {
        revision: 3,
        document,
    }
}

fn accept_suggestions(review: &mut ImportReview) {
    for index in 0..review.proposal().items().len() {
        review
            .decide(index, review.suggested_decision(index).unwrap())
            .unwrap();
    }
}

#[test]
fn review_requires_all_items_and_cannot_write_or_erase_the_source() {
    let base = base();
    let original = base.clone();
    let mut review = ImportReview::new(
        base.clone(),
        proposal(&[
            ("paragraph", "Name: Imported Person"),
            ("heading", "Experience"),
            ("list_item", "- New contribution"),
        ]),
    )
    .unwrap();
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::IncompleteReview
    );
    assert!(review.suggested_decision(0).is_some());
    assert!(review.decision(0).is_none());
    assert_eq!(
        review.decide(usize::MAX, ReviewDecision::Reject),
        Err(ReviewError::UnknownItem)
    );
    review.decide(0, ReviewDecision::Reject).unwrap();
    review
        .decide(
            1,
            ReviewDecision::Section {
                heading: "Experience".to_owned(),
                target: SectionTarget::New,
            },
        )
        .unwrap();
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::IncompleteReview
    );
    review
        .decide(2, review.suggested_decision(2).unwrap())
        .unwrap();
    let candidate = review.prepare(&base).unwrap();
    assert_eq!(candidate.expected_revision, Some(3));
    assert_eq!(candidate.document.contact.full_name, "Existing Person");
    assert_eq!(candidate.document.sections.len(), 2);
    assert_eq!(candidate.document.sections[0], base.document.sections[0]);
    assert!(
        render_plain_text(&candidate.document)
            .unwrap()
            .contains("- New contribution")
    );
    assert_eq!(base, original);
    assert_eq!(
        review.proposal().source().blocks()[0].text,
        "Name: Imported Person"
    );
    review.reset_decision(2).unwrap();
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::IncompleteReview
    );
}

#[test]
fn duplicate_headings_require_explicit_merge_or_keep_both_and_keep_existing_ids() {
    let base = base();
    let id = base.document.sections[0].id;
    let mut review = ImportReview::new(
        base.clone(),
        proposal(&[
            ("heading", "Expérience"),
            ("paragraph", "Synthetic project contribution"),
        ]),
    )
    .unwrap();
    assert_eq!(review.possible_section_duplicates(0), vec![id]);
    accept_suggestions(&mut review);
    let kept = review.prepare(&base).unwrap().document;
    assert_eq!(kept.sections.len(), 2);
    assert_eq!(kept.sections[0].id, id);
    review
        .decide(
            0,
            ReviewDecision::Section {
                heading: "Expérience".to_owned(),
                target: SectionTarget::Existing(id),
            },
        )
        .unwrap();
    let merged = review.prepare(&base).unwrap().document;
    assert_eq!(merged.sections.len(), 1);
    assert_eq!(merged.sections[0].id, id);
    assert_eq!(merged.sections[0].heading, "Experience");
    assert_eq!(merged.sections[0].entries.len(), 1);
    review.decide(0, ReviewDecision::Reject).unwrap();
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::MissingDestination
    );
    // Rejecting a heading cannot silently discard its accepted children. A
    // deliberate move makes the otherwise-orphaned text valid again.
    review
        .decide(
            1,
            ReviewDecision::Text {
                text: "Synthetic project contribution".to_owned(),
                is_bullet: false,
                target: TextTarget::ExistingSection(id),
            },
        )
        .unwrap();
    assert!(review.prepare(&base).is_ok());
}

#[test]
fn contact_conflicts_require_keep_or_replace_including_duplicates_within_import() {
    let base = base();
    let mut review = ImportReview::new(
        base.clone(),
        proposal(&[
            ("paragraph", "Name: New Person"),
            ("paragraph", "Email: e@example.org"),
        ]),
    )
    .unwrap();
    assert!(review.contact_conflicts(0));
    assert!(!review.contact_conflicts(1));
    accept_suggestions(&mut review);
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::ContactConflict
    );
    for (mode, expected) in [
        (ContactMode::KeepExisting, "Existing Person"),
        (ContactMode::Replace, "New Person"),
    ] {
        review
            .decide(
                0,
                ReviewDecision::Contact {
                    field: ContactField::FullName,
                    value: "New Person".to_owned(),
                    mode,
                },
            )
            .unwrap();
        let candidate = review.prepare(&base).unwrap().document;
        assert_eq!(candidate.contact.full_name, expected);
        assert_eq!(candidate.contact.email, "e@example.org");
    }
    let mut repeated = ImportReview::new(
        base.clone(),
        proposal(&[
            ("paragraph", "Email: one@example.org"),
            ("paragraph", "Email: two@example.org"),
        ]),
    )
    .unwrap();
    accept_suggestions(&mut repeated);
    assert_eq!(
        repeated.prepare(&base).unwrap_err(),
        ReviewError::ContactConflict
    );
}

#[test]
fn stale_identity_revision_or_content_never_produces_a_candidate() {
    let base = base();
    let mut review = ImportReview::new(
        base.clone(),
        proposal(&[("paragraph", "Unclassified text")]),
    )
    .unwrap();
    accept_suggestions(&mut review);
    for mut current in [base.clone(), base.clone(), base.clone()] {
        current.revision += 1;
        assert_eq!(
            review.prepare(&current).unwrap_err(),
            ReviewError::StaleDraft
        );
        current.revision = base.revision;
        current.document.title.push_str(" changed");
        assert_eq!(
            review.prepare(&current).unwrap_err(),
            ReviewError::StaleDraft
        );
        current.document = base.document.clone();
        current.document.document_id = EntityId::new();
        assert_eq!(
            review.prepare(&current).unwrap_err(),
            ReviewError::StaleDraft
        );
    }
}

#[test]
fn limits_and_invalid_destinations_preserve_the_complete_review_for_correction() {
    let base = base();
    let long = "界".repeat(2_001);
    let mut review = ImportReview::new(base.clone(), proposal(&[("paragraph", &long)])).unwrap();
    accept_suggestions(&mut review);
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::InvalidContent
    );
    assert_eq!(review.proposal().source().blocks()[0].text, long);
    review
        .decide(
            0,
            ReviewDecision::Text {
                text: "Reviewed shorter user edit".to_owned(),
                is_bullet: false,
                target: TextTarget::NewSection("Community".to_owned()),
            },
        )
        .unwrap();
    assert!(review.prepare(&base).is_ok());
    review
        .decide(
            0,
            ReviewDecision::Text {
                text: "Keep me".to_owned(),
                is_bullet: false,
                target: TextTarget::ExistingSection(EntityId::new()),
            },
        )
        .unwrap();
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::MissingDestination
    );
    review
        .decide(
            0,
            ReviewDecision::Text {
                text: "bad\u{1b}control".to_owned(),
                is_bullet: false,
                target: TextTarget::NewSection("Community".to_owned()),
            },
        )
        .unwrap();
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::InvalidContent
    );
    assert_eq!(
        review.decide(
            0,
            ReviewDecision::Text {
                text: "x".repeat(MAX_REVIEW_CHARACTERS + 1),
                is_bullet: false,
                target: TextTarget::ProposedSection(999)
            }
        ),
        Err(ReviewError::InvalidContent)
    );
}

#[test]
fn unknown_content_and_blank_blocks_remain_reviewable_and_source_is_never_executed() {
    let base = base();
    let text = "<script>do something</script> file:///secret $(command)";
    let mut review = ImportReview::new(
        base.clone(),
        proposal(&[
            ("heading", "Unusual 🌻 heading"),
            ("paragraph", text),
            ("paragraph", "  "),
            ("list_item", "• Text after blank"),
        ]),
    )
    .unwrap();
    accept_suggestions(&mut review);
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::InvalidContent
    );
    review.decide(2, ReviewDecision::Reject).unwrap();
    let candidate = review.prepare(&base).unwrap().document;
    candidate.validate(DocumentLimits::default()).unwrap();
    let output = render_plain_text(&candidate).unwrap();
    assert!(output.contains(text));
    assert!(output.contains("Unusual 🌻 heading"));
    assert!(output.contains("- Text after blank"));
    assert!(!format!("{review:?}").contains(text));
    assert_eq!(review.proposal().source().blocks().len(), 4);
}

#[test]
fn rejecting_everything_keeps_the_draft_identical() {
    let base = base();
    let mut review = ImportReview::new(
        base.clone(),
        proposal(&[("paragraph", "private content"), ("heading", "Unknown")]),
    )
    .unwrap();
    review.decide(0, ReviewDecision::Reject).unwrap();
    review.decide(1, ReviewDecision::Reject).unwrap();
    assert_eq!(review.prepare(&base).unwrap().document, base.document);
}
