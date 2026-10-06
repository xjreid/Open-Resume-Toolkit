//! Synthetic regression artifacts for independent PDF/DOCX/text extraction.
use ort_domain::{
    Bullet, DocumentStyle, EntityId, NamedField, PARAGRAPH_FIELD_LABEL, ResumeDocument,
    ResumeEntry, ResumeSection,
};
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("provide a new output directory")?,
    );
    fs::create_dir(&root)?;
    let mut document = ResumeDocument::empty("Lossless field audit");
    document.contact.full_name = "Synthetic Applicant".into();
    let labels = [
        "Language",
        "Certification",
        "Extra",
        " Extra ",
        PARAGRAPH_FIELD_LABEL,
        PARAGRAPH_FIELD_LABEL,
    ];
    let markers = [
        "LANGUAGE-MARKER",
        "CERTIFICATION-MARKER",
        "EXTRA-ONE-MARKER",
        "EXTRA-TWO-MARKER",
        "PARAGRAPH-ONE-MARKER",
        "PARAGRAPH-TWO-MARKER",
        "BULLET-MARKER",
    ];
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
            fields: labels
                .into_iter()
                .zip(markers)
                .zip(0..)
                .map(|((label, value), order)| NamedField {
                    id: EntityId::new(),
                    order,
                    label: label.into(),
                    value: value.into(),
                    is_skill: false,
                })
                .collect(),
            bullets: vec![Bullet {
                id: EntityId::new(),
                order: 0,
                text: markers[6].into(),
            }],
            links: vec![],
        }],
    });
    document.validate(ort_domain::DocumentLimits::default())?;
    fs::write(
        root.join("markers.json"),
        serde_json::to_vec_pretty(&markers)?,
    )?;
    fs::write(
        root.join("expected.txt"),
        ort_documents::render_plain_text(&document)
            .map_err(|_| "invalid synthetic text fixture")?,
    )?;
    for (name, style) in [
        ("plain", DocumentStyle::Plain),
        ("technical", DocumentStyle::Technical),
        ("professional", DocumentStyle::Professional),
        ("modern", DocumentStyle::Modern),
    ] {
        fs::write(
            root.join(format!("{name}.pdf")),
            ort_render::render_pdf_with_style(&document, style)?.bytes,
        )?;
        fs::write(
            root.join(format!("{name}.docx")),
            ort_documents::render_docx_with_style(&document, style)
                .map_err(|_| "invalid synthetic DOCX fixture")?,
        )?;
    }
    Ok(())
}
