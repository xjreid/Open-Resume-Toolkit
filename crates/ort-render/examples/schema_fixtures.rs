//! Synthetic v2 compatibility output. No profile or input files are accessed.
#[path = "../../ort-documents/tests/support/mod.rs"]
mod support;
use ort_domain::{CalendarDate, DateEnd, DocumentStyle, EntityId, ResumeDate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("provide a new output directory")?,
    );
    std::fs::create_dir(&root)?;
    let mut document = support::fixture(support::OUTPUT_FIXTURE_KINDS[0]).upgraded_v2()?;
    let entry = &mut document.sections[0].entries[0];
    entry.date_range.clear();
    entry.dates = Some(vec![
        ResumeDate {
            id: EntityId::new(),
            order: 0,
            label: "Period".into(),
            start: Some(CalendarDate {
                year: 2020,
                month: None,
                expected: false,
            }),
            end: Some(DateEnd::Present),
        },
        ResumeDate {
            id: EntityId::new(),
            order: 1,
            label: "Graduation".into(),
            start: None,
            end: Some(DateEnd::Date {
                value: CalendarDate {
                    year: 2027,
                    month: Some(6),
                    expected: true,
                },
            }),
        },
        ResumeDate {
            id: EntityId::new(),
            order: 2,
            label: "OMITTED_EMPTY_DATE".into(),
            start: None,
            end: None,
        },
    ]);
    for (name, style) in [
        ("plain", DocumentStyle::Plain),
        ("technical", DocumentStyle::Technical),
        ("professional", DocumentStyle::Professional),
        ("modern", DocumentStyle::Modern),
    ] {
        let pdf = ort_render::render_pdf_with_style(&document, style)?;
        std::fs::write(root.join(format!("{name}.pdf")), pdf.bytes)?;
        std::fs::write(
            root.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&pdf.receipt)?,
        )?;
        std::fs::write(
            root.join(format!("{name}.docx")),
            ort_documents::render_docx_with_style(&document, style)
                .map_err(|_| "synthetic DOCX failure")?,
        )?;
    }
    std::fs::write(
        root.join("expected.txt"),
        ort_documents::render_plain_text(&document).map_err(|_| "synthetic text failure")?,
    )?;
    Ok(())
}
