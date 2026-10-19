use std::io::Cursor;

use ort_document_worker::extract_docx;
use ort_documents::import::{
    BlockKind, ImportProposal, InputFormat, ProposedContent, ValidatedExtraction,
};
use ort_documents::render_docx_with_style;
use ort_domain::{
    Bullet, DocumentStyle, EntityId, NamedField, ResumeDocument, ResumeEntry, ResumeSection,
};

fn shipping_resume() -> ResumeDocument {
    let mut document = ResumeDocument::empty("internal title");
    document.contact.full_name = "Zoë Example".into();
    document.contact.email = "synthetic@example.org".into();
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![ResumeEntry {
            dates: None,
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

    document
}

const STYLES: [DocumentStyle; 4] = [
    DocumentStyle::Plain,
    DocumentStyle::Technical,
    DocumentStyle::Professional,
    DocumentStyle::Modern,
];

fn extract(document: &ResumeDocument, style: DocumentStyle) -> ValidatedExtraction {
    let docx = render_docx_with_style(document, style).unwrap();
    let wire = extract_docx(&mut Cursor::new(docx)).unwrap();
    ValidatedExtraction::decode(&wire, InputFormat::Docx).unwrap()
}

#[test]
fn constrained_parser_reads_the_shipping_docx_export_shape() {
    let document = shipping_resume();
    for style in STYLES {
        let extraction = extract(&document, style);
        let heading = if style == DocumentStyle::Technical {
            "EXPERIENCE"
        } else {
            "Experience"
        };
        // Independent header columns contain separate paragraphs in reading
        // order. The empty table anchor must not become an extraction block.
        let expected = [
            (BlockKind::Paragraph, "Zoë Example"),
            (BlockKind::Paragraph, "synthetic@example.org"),
            (BlockKind::Heading, heading),
            (BlockKind::Paragraph, "Engineer"),
            (BlockKind::Paragraph, "Synthetic Cooperative"),
            (BlockKind::Paragraph, "2024–2026"),
            (BlockKind::Paragraph, "Remote"),
            (
                BlockKind::ListItem,
                "Preserved Unicode, structure, and review boundaries.",
            ),
        ];
        let actual = extraction
            .blocks()
            .iter()
            .map(|block| (block.kind, block.text.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{style:?}");

        let proposal = ImportProposal::map(extraction);
        assert_eq!(proposal.items().len(), expected.len());
        assert!(matches!(
            &proposal.items()[2].content,
            ProposedContent::Section { heading: value, .. } if value == heading
        ));
        for (item, (kind, value)) in proposal.items()[3..].iter().zip(&expected[3..]) {
            assert!(matches!(
                &item.content,
                ProposedContent::Text { text, is_bullet }
                    if text == value && *is_bullet == (*kind == BlockKind::ListItem)
            ));
            assert_eq!(item.section_index, Some(2), "{style:?}, {value}");
        }
    }
}

#[test]
fn body_only_skills_follow_their_section_without_empty_headers_or_anchors() {
    let mut document = shipping_resume();
    let mut entry = document.sections[0].entries[0].clone();
    entry.id = EntityId::new();
    entry.heading.clear();
    entry.subheading.clear();
    entry.date_range.clear();
    entry.location.clear();
    entry.bullets.clear();
    entry.fields = vec![NamedField {
        id: EntityId::new(),
        order: 0,
        label: "__ort_body_paragraph__".into(),
        value: "**Languages:** Rust, Python\n**Frameworks:** React".into(),
        is_skill: true,
        list_kind: None,
    }];
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 1,
        heading: "Technical Skills".into(),
        entries: vec![entry],
    });
    for style in STYLES {
        let extraction = extract(&document, style);
        let blocks = extraction.blocks();
        assert_eq!(blocks.len(), 10, "{style:?}");
        let heading = if style == DocumentStyle::Technical {
            "TECHNICAL SKILLS"
        } else {
            "Technical Skills"
        };
        assert_eq!(blocks[8].kind, BlockKind::Heading);
        assert_eq!(blocks[8].text, heading);
        assert_eq!(blocks[9].kind, BlockKind::Paragraph);
        assert_eq!(blocks[9].text, "Languages: Rust, Python\nFrameworks: React");
        let proposal = ImportProposal::map(extraction);
        assert_eq!(proposal.items()[9].section_index, Some(8));
        assert!(matches!(
            &proposal.items()[9].content,
            ProposedContent::Text { text, is_bullet: false }
                if text == "Languages: Rust, Python\nFrameworks: React"
        ));
    }
}
