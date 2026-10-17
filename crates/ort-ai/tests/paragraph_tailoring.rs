use ort_domain::{DocumentLimits, EntityId, NamedField, PARAGRAPH_FIELD_LABEL, ResumeDocument};
use serde_json::{Value, json};

fn fixture() -> (ResumeDocument, String, Value) {
    let value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/ai/tailoring-v7/student-backend.json"
    ))
    .unwrap();
    (
        serde_json::from_value(value["source"].clone()).unwrap(),
        value["job"].as_str().unwrap().into(),
        value["candidate"].clone(),
    )
}

#[test]
fn valid_multiple_paragraph_fields_survive_tailoring() {
    let (mut source, job, mut candidate) = fixture();
    source.contact.divider = ort_domain::ContactDivider::Bar;
    let entry = &mut source.sections[0].entries[0];
    entry.bullets.clear();
    entry.fields = (0..2)
        .map(|order| NamedField {
            id: EntityId::new(),
            order,
            label: PARAGRAPH_FIELD_LABEL.into(),
            value: "Source paragraph. ".repeat(75),
            is_skill: false,
            list_kind: None,
        })
        .collect();
    candidate["templateSections"][0]["entries"][0]["mainInfo"] = json!({
        "format": "paragraph", "items": entry.fields.iter().map(|field| json!({"text": field.value, "sourceRefs": [field.id]})).collect::<Vec<_>>()
    });
    source.validate(DocumentLimits::default()).unwrap();
    let result = ort_ai::materials::validate_refinement_detailed(
        &source,
        &source,
        &job,
        &candidate.to_string(),
        1,
    );
    let result = result.expect("valid unchanged paragraph fields");
    result.resume.validate(DocumentLimits::default()).unwrap();
    let paragraphs: Vec<_> = result.resume.sections[0].entries[0]
        .fields
        .iter()
        .filter(|field| field.label == PARAGRAPH_FIELD_LABEL)
        .collect();
    assert_eq!(paragraphs.len(), 2);
    for (actual, expected) in paragraphs.iter().zip(&source.sections[0].entries[0].fields) {
        assert_eq!(actual.value, expected.value.trim());
    }
    assert_eq!(result.resume.contact.divider, source.contact.divider);

    candidate["templateSections"][0]["entries"][0]["mainInfo"]["items"][0]["text"] =
        json!("x".repeat(DocumentLimits::default().field_characters + 1));
    assert!(
        ort_ai::materials::validate_refinement_detailed(
            &source,
            &source,
            &job,
            &candidate.to_string(),
            1,
        )
        .is_err()
    );
}
