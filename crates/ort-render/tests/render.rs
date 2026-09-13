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

#[test]
fn styles_are_deterministic_distinct_and_do_not_change_source_identity() {
    use ort_domain::DocumentStyle;
    use ort_render::render_pdf_with_style;
    for kind in support::OUTPUT_FIXTURE_KINDS {
        let document = support::fixture(kind);
        let original = render_pdf(&document).unwrap();
        let mut hashes = std::collections::BTreeSet::new();
        for style in [
            DocumentStyle::Plain,
            DocumentStyle::Technical,
            DocumentStyle::Professional,
            DocumentStyle::Modern,
        ] {
            let output = render_pdf_with_style(&document, style)
                .unwrap_or_else(|error| panic!("{kind} {style:?}: {error}"));
            let repeated = render_pdf_with_style(&document, style).unwrap();
            assert_eq!(output.bytes, repeated.bytes);
            assert_eq!(output.receipt, repeated.receipt);
            assert_eq!(
                output.receipt.document_sha256,
                original.receipt.document_sha256
            );
            assert_eq!(output.receipt.template_id, style.pdf_template_id());
            if style == DocumentStyle::Modern {
                assert_eq!(
                    output.receipt.font_bundle_id,
                    "liberation-sans/pdfjs-6.3.289"
                );
                assert_ne!(
                    output.receipt.font_bundle_sha256,
                    original.receipt.font_bundle_sha256
                );
            } else {
                assert_eq!(
                    output.receipt.font_bundle_sha256,
                    original.receipt.font_bundle_sha256
                );
            }
            assert_eq!(output.receipt.pdf_sha256, sha256(&output.bytes));
            assert!(hashes.insert(output.receipt.pdf_sha256.clone()));
            if style == DocumentStyle::Plain {
                assert_eq!(output.bytes, original.bytes);
            }
        }
    }
}

#[test]
fn every_style_rejects_overflow_missing_glyphs_and_external_links() {
    use ort_domain::DocumentStyle;
    for style in [
        DocumentStyle::Plain,
        DocumentStyle::Technical,
        DocumentStyle::Professional,
        DocumentStyle::Modern,
    ] {
        let mut document = support::fixture("standard");
        document.contact.full_name = "W".repeat(1900);
        let error = ort_render::render_pdf_with_style(&document, style).err();
        assert!(
            matches!(error, Some(PdfRenderError::LayoutLimit)),
            "{style:?}: {error:?}"
        );
        document.contact.full_name = "示例".into();
        assert!(matches!(
            ort_render::render_pdf_with_style(&document, style),
            Err(PdfRenderError::UnsupportedGlyph)
        ));
        document.contact.full_name = "Synthetic".into();
        document.contact.links[0].url = "file:///private/secret".into();
        assert!(matches!(
            ort_render::render_pdf_with_style(&document, style),
            Err(PdfRenderError::InvalidContent)
        ));
    }
}

#[test]
fn aligned_dates_still_reject_unsupported_glyphs() {
    for style in [
        ort_domain::DocumentStyle::Technical,
        ort_domain::DocumentStyle::Professional,
        ort_domain::DocumentStyle::Modern,
    ] {
        let mut document = support::fixture("standard");
        document.sections[0].entries[0].date_range = "示例".into();
        assert_eq!(
            ort_render::render_pdf_with_style(&document, style).err(),
            Some(PdfRenderError::UnsupportedGlyph)
        );
    }
}
