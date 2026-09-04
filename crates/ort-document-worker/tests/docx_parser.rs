use std::io::Cursor;

use ort_document_worker::extract_docx;
use ort_documents::import::{ImportProposal, InputFormat, ProposedContent, ValidatedExtraction};
use ort_documents::render_docx;
use ort_domain::{Bullet, EntityId, ResumeDocument, ResumeEntry, ResumeSection};

#[test]
fn constrained_parser_reads_the_shipping_docx_export_shape() {
    let mut document = ResumeDocument::empty("internal title");
    document.contact.full_name = "Zoë Example".into();
    document.contact.email = "synthetic@example.org".into();
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![ResumeEntry {
            id: EntityId::new(),
            order: 0,
            heading: "Engineer".into(),
            subheading: "Synthetic Cooperative".into(),
            date_range: "2024–2026".into(),
            location: "Remote".into(),
            fields: Vec::new(),
            bullets: vec![Bullet {
                id: EntityId::new(),
                order: 0,
                text: "Preserved Unicode, structure, and review boundaries.".into(),
            }],
            links: Vec::new(),
        }],
    });

    let docx = render_docx(&document).unwrap();
    let wire = extract_docx(&mut Cursor::new(docx)).unwrap();
    let extraction = ValidatedExtraction::decode(&wire, InputFormat::Docx).unwrap();
    let blocks = extraction.blocks();
    assert_eq!(blocks.len(), 8);
    assert_eq!(blocks[0].text, "Zoë Example");
    assert_eq!(blocks[2].text, "Experience");
    assert_eq!(
        blocks[7].text,
        "Preserved Unicode, structure, and review boundaries."
    );

    let proposal = ImportProposal::map(extraction);
    assert!(matches!(
        &proposal.items()[2].content,
        ProposedContent::Section { heading, .. } if heading == "Experience"
    ));
    assert!(matches!(
        &proposal.items()[3].content,
        ProposedContent::Text { text, .. } if text == "Engineer"
    ));
    assert_eq!(proposal.items()[3].section_index, Some(2));
}
