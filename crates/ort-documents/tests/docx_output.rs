mod support;
use ort_documents::{DocxExportError, MAX_DOCX_BYTES, render_docx};
use ort_domain::{Link, ResumeDocument};
use quick_xml::{Reader, events::Event};
use std::collections::BTreeMap;

// Test-only local-header inspection. Independent central directory + ZIP/CRC
// validation also runs through Python's standard-library zipfile in CI.
fn parts(bytes: &[u8]) -> BTreeMap<String, String> {
    let mut at = 0;
    let mut parts = BTreeMap::new();
    while bytes[at..].starts_with(b"PK\x03\x04") {
        let size = u32::from_le_bytes(bytes[at + 18..at + 22].try_into().unwrap()) as usize;
        let len = usize::from(u16::from_le_bytes(
            bytes[at + 26..at + 28].try_into().unwrap(),
        ));
        let start = at + 30 + len;
        assert_eq!(&bytes[at + 8..at + 10], &[0, 0]);
        assert_eq!(&bytes[at + 28..at + 30], &[0, 0]);
        let body = &bytes[start..start + size];
        assert_eq!(crc32fast::hash(body).to_le_bytes(), bytes[at + 14..at + 18]);
        let name = std::str::from_utf8(&bytes[at + 30..start])
            .unwrap()
            .to_owned();
        assert!(
            parts
                .insert(name, std::str::from_utf8(body).unwrap().to_owned())
                .is_none()
        );
        at = start + size;
    }
    assert!(bytes[at..].starts_with(b"PK\x01\x02"));
    assert!(bytes[bytes.len() - 22..].starts_with(b"PK\x05\x06"));
    parts
}

#[test]
fn fixed_parts_well_formed_xml_and_deterministic_bytes_for_whole_corpus() {
    for kind in ["standard", "sparse", "unicode", "hostile", "dense"] {
        let doc = support::fixture(kind);
        let bytes = render_docx(&doc).unwrap();
        assert_eq!(bytes, render_docx(&doc).unwrap());
        assert!(bytes.len() <= MAX_DOCX_BYTES);
        let parts = parts(&bytes);
        assert_eq!(
            parts.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "[Content_Types].xml",
                "_rels/.rels",
                "word/_rels/document.xml.rels",
                "word/document.xml",
                "word/numbering.xml",
                "word/styles.xml"
            ]
        );
        for xml in parts.values() {
            let mut reader = Reader::from_str(xml);
            let mut depth = 0;
            loop {
                match reader.read_event().unwrap() {
                    Event::Start(_) => depth += 1,
                    Event::End(_) => depth -= 1,
                    Event::DocType(_) => panic!("no DTD"),
                    Event::Eof => break,
                    _ => {}
                }
            }
            assert_eq!(depth, 0);
            assert!(!xml.contains(&doc.title));
            assert!(!xml.contains(&doc.document_id.to_string()));
        }
    }
}

#[test]
fn semantic_headings_lists_line_breaks_and_safe_visible_links() {
    let values = parts(&render_docx(&support::fixture("standard")).unwrap());
    let body = &values["word/document.xml"];
    for required in [
        "w:val=\"Heading1\"",
        "w:val=\"Heading2\"",
        "<w:numPr>",
        "<w:br/>",
        "<w:tab/>",
        "Portfolio: https://example.org/work?a=1&amp;b=2",
        "Tools: Rust, TypeScript &amp; SQL",
    ] {
        assert!(body.contains(required), "{required}");
    }
    assert!(
        values["word/_rels/document.xml.rels"]
            .contains("Target=\"https://example.org/work?a=1&amp;b=2\"")
    );
    let hostile = parts(&render_docx(&support::fixture("hostile")).unwrap());
    assert!(!hostile["word/document.xml"].contains("<w:object>"));
    assert!(hostile["word/document.xml"].contains("&lt;w:object&gt;"));
    assert!(hostile["word/document.xml"].contains("&quot;quoted&quot;"));
}

#[test]
fn empty_invalid_controls_noncharacters_and_unsafe_links_fail_closed() {
    assert_eq!(
        render_docx(&ResumeDocument::empty("Only internal title")),
        Err(DocxExportError::EmptyContent)
    );
    for c in ['\0', '\u{1b}', '\u{85}', '\u{fffe}', '\u{ffff}'] {
        let mut doc = support::fixture("standard");
        doc.contact.full_name.push(c);
        assert_eq!(
            render_docx(&doc),
            Err(DocxExportError::UnsupportedCharacter)
        );
    }
    for url in [
        "file:///private/path",
        "javascript:alert(1)",
        "data:text/html,hi",
        "\\\\server\\share",
        "https://example.org/a\nb",
        "https://example.org/a b",
    ] {
        let mut doc = support::fixture("standard");
        doc.contact.links = vec![Link {
            label: "untrusted".into(),
            url: url.into(),
        }];
        assert!(render_docx(&doc).is_err(), "{url}");
    }
    let mut invalid = support::fixture("standard");
    invalid.sections[0].order = 1;
    assert_eq!(render_docx(&invalid), Err(DocxExportError::InvalidDocument));
}

#[test]
fn escaping_cannot_inject_a_relationship_or_hide_content() {
    let mut doc = support::fixture("standard");
    doc.contact.links[0].url = "https://example.org/\"/><Relationship/>&'".into();
    let values = parts(&render_docx(&doc).unwrap());
    assert_eq!(
        values["word/_rels/document.xml.rels"]
            .matches("<Relationship ")
            .count(),
        4
    );
    assert!(
        values["word/_rels/document.xml.rels"]
            .contains("&quot;/&gt;&lt;Relationship/&gt;&amp;&apos;")
    );
}

#[test]
fn newline_heavy_legal_input_hits_xml_limit_without_partial_output() {
    let mut doc = support::fixture("standard");
    for index in 0..58 {
        doc.sections[0].entries[0].bullets.push(ort_domain::Bullet {
            id: ort_domain::EntityId::new(),
            order: index + 2,
            text: format!("x{}x", "\n".repeat(498)),
        });
    }
    assert_eq!(doc.validate(ort_domain::DocumentLimits::default()), Ok(()));
    assert_eq!(
        render_docx(&doc).err(),
        Some(DocxExportError::OutputTooLarge)
    );
}
