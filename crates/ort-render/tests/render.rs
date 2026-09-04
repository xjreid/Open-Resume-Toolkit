#[path = "../../ort-documents/tests/support/mod.rs"]
mod support;

use ort_render::{PdfRenderError, render_pdf, sha256};

#[test]
fn fixed_output_is_repeatable_and_receipt_describes_exact_bytes() {
    for kind in support::OUTPUT_FIXTURE_KINDS {
        let document = support::fixture(kind);
        let first = render_pdf(&document).unwrap_or_else(|error| panic!("{kind}: {error}"));
        let second = render_pdf(&document).unwrap();
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(first.receipt, second.receipt);
        assert_eq!(first.receipt.pdf_sha256, sha256(&first.bytes));
        assert_eq!(first.receipt.byte_count, first.bytes.len());
        let expected_pages = match kind {
            "dense" => 4,
            "paginated" => 2,
            _ => 1,
        };
        assert_eq!(first.receipt.page_count, expected_pages, "{kind}");
        assert!(first.bytes.starts_with(b"%PDF-"));
    }
}

#[test]
fn refuses_missing_glyphs_invalid_documents_and_excess_pages() {
    let mut missing_glyph = support::fixture("unicode");
    missing_glyph.contact.full_name.push_str(" 示例");
    assert!(matches!(
        render_pdf(&missing_glyph),
        Err(PdfRenderError::UnsupportedGlyph)
    ));
    let mut doc = support::fixture("standard");
    doc.contact.full_name.push('\0');
    assert!(matches!(
        render_pdf(&doc),
        Err(PdfRenderError::InvalidContent)
    ));
    let mut doc = support::fixture("dense");
    for index in 42..60 {
        doc.sections[0].entries[0].bullets.push(ort_domain::Bullet {
            id: ort_domain::EntityId::new(),
            order: index,
            text: "Synthetic".into(),
        });
    }
    for bullet in &mut doc.sections[0].entries[0].bullets {
        bullet.text = "Long synthetic content with line wrapping. ".repeat(11);
    }
    assert!(ort_documents::render_plain_text(&doc).is_ok());
    assert!(matches!(render_pdf(&doc), Err(PdfRenderError::LayoutLimit)));
}

#[test]
fn typst_code_is_literal_data_and_private_titles_are_not_rendered() {
    let mut doc = support::fixture("standard");
    doc.contact.full_name = r#"#read("/secret") #include "secret" #panic("fail")"#.into();
    let before = render_pdf(&doc).unwrap();
    doc.title = "A different internal title".into();
    let after = render_pdf(&doc).unwrap();
    assert_eq!(before.bytes, after.bytes);
    assert_ne!(
        before.receipt.document_sha256,
        after.receipt.document_sha256
    );
}

#[test]
fn overwide_unbreakable_content_is_rejected_not_clipped() {
    let mut doc = support::fixture("standard");
    doc.contact.full_name = "W".repeat(1900);
    assert!(matches!(render_pdf(&doc), Err(PdfRenderError::LayoutLimit)));
}
