use ort_application::import_review::{ImportReview, ReviewDecision, ReviewError};
use ort_documents::import::{ImportProposal, InputFormat, ValidatedExtraction};
use ort_domain::{ImportChoices, ResumeDocument, VersionedResumeResponse};

const FIXTURE: &[u8] = include_bytes!("../../../fixtures/documents/import-review-choices.json");
fn review() -> (ImportReview, VersionedResumeResponse) {
    let base = VersionedResumeResponse {
        revision: 1,
        document: ResumeDocument::empty("Synthetic"),
    };
    let source = ValidatedExtraction::decode(br#"{"version":1,"format":"docx","pageCount":1,"blocks":[{"page":1,"kind":"heading","text":"Projects"},{"page":1,"kind":"list_item","text":"Synthetic contribution"},{"page":1,"kind":"paragraph","text":"Name: Synthetic Person"}]}"#, InputFormat::Docx).unwrap();
    (
        ImportReview::new(base.clone(), ImportProposal::map(source)).unwrap(),
        base,
    )
}
#[test]
fn shared_frontend_fixture_maps_to_authoritative_native_review() {
    let (mut review, base) = review();
    review
        .replace_choices(ImportChoices::decode(FIXTURE).unwrap())
        .unwrap();
    let candidate = review.prepare(&base).unwrap();
    assert_eq!(candidate.expected_revision, Some(1));
    assert_eq!(candidate.document.contact.full_name, "Synthetic Person");
    assert_eq!(candidate.document.sections.len(), 1);
    assert_eq!(
        candidate.document.sections[0].entries[0].bullets[0].text,
        "Synthetic contribution"
    );
    assert_eq!(review.proposal().source().blocks()[0].text, "Projects");
}
#[test]
fn invalid_batches_leave_all_prior_decisions_intact_and_pending_resets_are_explicit() {
    let (mut review, base) = review();
    review
        .replace_choices(ImportChoices::decode(FIXTURE).unwrap())
        .unwrap();
    let mut oversized = ImportChoices::decode(FIXTURE).unwrap();
    if let ort_domain::ImportChoice::Section { heading, .. } = &mut oversized.choices[0] {
        *heading = "x".repeat(100_001);
    }
    assert_eq!(
        review.replace_choices(oversized),
        Err(ReviewError::InvalidContent)
    );
    assert_eq!(
        review.replace_choices(ImportChoices::decode(br#"{"choices":[]}"#).unwrap()),
        Err(ReviewError::IncompleteReview)
    );
    assert!(review.prepare(&base).is_ok());
    assert!(matches!(
        review.decision(0),
        Some(ReviewDecision::Section { .. })
    ));
    let mut pending = ImportChoices::decode(FIXTURE).unwrap();
    pending.choices[0] = ort_domain::ImportChoice::Pending {};
    review.replace_choices(pending).unwrap();
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::IncompleteReview
    );
}
#[test]
fn decoding_refuses_unknown_authority_fields_ambiguous_targets_and_resource_abuse() {
    for input in [
        r#"{"choices":[],"owner":"main"}"#,
        r#"{"choices":[{"kind":"reject","text":"smuggled"}]}"#,
        r#"{"choices":[{"kind":"text","text":"x","bullet":false,"target":{"kind":"new","heading":"X","id":"extra"}}]}"#,
        r#"{"choices":[{"kind":"text","text":"x","bullet":false,"target":{"kind":"proposed","index":-1}}]}"#,
        r#"{"choices":[{"kind":"text","text":"x","bullet":false,"target":{"kind":"proposed","index":1000}}]}"#,
        r#"{"choices":[{"kind":"contact","field":"fullName","value":"\u0000","mode":"replace"}]}"#,
    ] {
        assert!(ImportChoices::decode(input.as_bytes()).is_err(), "{input}");
    }
    assert!(ImportChoices::decode(&vec![b' '; ort_domain::MAX_IMPORT_CHOICES_BYTES + 1]).is_err());
    let many = format!(
        "{{\"choices\":[{}]}}",
        vec![r#"{"kind":"reject"}"#; 1001].join(",")
    );
    assert!(ImportChoices::decode(many.as_bytes()).is_err());
}

#[test]
fn native_snapshot_keeps_source_and_suggestions_without_accepting_decisions() {
    let (review, base) = review();
    let snapshot = review.snapshot(ort_domain::EntityId::new().as_uuid().to_string());
    assert_eq!(snapshot.base_revision, base.revision);
    assert_eq!(snapshot.blocks.len(), 3);
    assert_eq!(snapshot.blocks[0].proposed_section, None);
    assert_eq!(snapshot.blocks[1].proposed_section, Some(0));
    assert_eq!(snapshot.blocks[0].source, "Projects");
    assert!(review.decision(0).is_none());
    assert_eq!(
        review.prepare(&base).unwrap_err(),
        ReviewError::IncompleteReview
    );
}
