use ort_domain::{
    DocumentLimits, DocumentStyle, EntityId, NamedField, ResumeDocument, ResumeEntry, ResumeSection,
};
#[test]
fn docx_preserves_all_valid_named_fields() {
    let mut document = ResumeDocument::empty("Synthetic review");
    document.contact.full_name = "Synthetic Applicant".into();
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![ResumeEntry {
            id: EntityId::new(),
            order: 0,
            heading: "Synthetic Position".into(),
            subheading: String::new(),
            date_range: String::new(),
            dates: None,
            location: String::new(),
            fields: vec![
                NamedField {
                    id: EntityId::new(),
                    order: 0,
                    label: "Language".into(),
                    value: "Rust".into(),
                    is_skill: false,
                },
                NamedField {
                    id: EntityId::new(),
                    order: 1,
                    label: "Certification".into(),
                    value: "CERTIFICATION-PRESERVATION-MARKER".into(),
                    is_skill: false,
                },
            ],
            bullets: vec![],
            links: vec![],
        }],
    });
    document.validate(DocumentLimits::default()).unwrap();
    assert!(
        ort_documents::render_plain_text(&document)
            .unwrap()
            .contains("CERTIFICATION-PRESERVATION-MARKER")
    );
    for style in [
        DocumentStyle::Plain,
        DocumentStyle::Technical,
        DocumentStyle::Professional,
        DocumentStyle::Modern,
    ] {
        let bytes = ort_documents::render_docx_with_style(&document, style).unwrap();
        assert!(bytes.windows(4).any(|part| part == b"Rust"));
        assert!(
            bytes
                .windows(b"CERTIFICATION-PRESERVATION-MARKER".len())
                .any(|part| part == b"CERTIFICATION-PRESERVATION-MARKER")
        );
    }
}
