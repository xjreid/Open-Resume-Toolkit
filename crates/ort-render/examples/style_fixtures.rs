//! Synthetic style audit output. Never opens a user profile, vault or input file.
#[path = "../../ort-documents/tests/support/mod.rs"]
mod support;

use ort_domain::{CalendarDate, DateEnd, DocumentStyle, EntityId, ResumeDate};
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(args.next().ok_or("provide a new audit directory")?);
    let schema_v2 = match args.next() {
        None => false,
        Some(value) if value == "schema-v2" => true,
        Some(_) => return Err("optional mode must be schema-v2".into()),
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    fs::create_dir(&root)?;
    for (name, style) in [
        ("technical", DocumentStyle::Technical),
        ("professional", DocumentStyle::Professional),
        ("modern", DocumentStyle::Modern),
    ] {
        let pdf = root.join(name).join("pdf");
        let docx = root.join(name).join("docx");
        fs::create_dir_all(&pdf)?;
        fs::create_dir_all(&docx)?;
        for kind in support::OUTPUT_FIXTURE_KINDS {
            let mut document = support::fixture(kind);
            if schema_v2 {
                document = document.upgraded_v2()?;
                if let Some(entry) = document
                    .sections
                    .first_mut()
                    .and_then(|section| section.entries.first_mut())
                {
                    entry.date_range.clear();
                    entry.dates = Some(audit_dates());
                }
            }
            let source = serde_json::to_vec_pretty(&document)?;
            let text =
                ort_documents::render_plain_text(&document).map_err(|_| "invalid fixture")?;
            let artifact = ort_render::render_pdf_with_style(&document, style)?;
            fs::write(pdf.join(format!("{kind}.pdf")), &artifact.bytes)?;
            fs::write(
                pdf.join(format!("{kind}.json")),
                serde_json::to_vec_pretty(&artifact.receipt)?,
            )?;
            fs::write(pdf.join(format!("{kind}.source.json")), &source)?;
            fs::write(pdf.join(format!("{kind}.txt")), &text)?;
            fs::write(
                docx.join(format!("{kind}.docx")),
                ort_documents::render_docx_with_style(&document, style)
                    .map_err(|_| "invalid DOCX fixture")?,
            )?;
            fs::write(docx.join(format!("{kind}.json")), &source)?;
            fs::write(docx.join(format!("{kind}.txt")), &text)?;
        }
    }
    println!(
        "Three synthetic style corpora written; native reader qualification remains separate."
    );
    Ok(())
}

fn audit_dates() -> Vec<ResumeDate> {
    vec![
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
            label: "Start only".into(),
            start: Some(CalendarDate {
                year: 2019,
                month: Some(9),
                expected: false,
            }),
            end: None,
        },
        ResumeDate {
            id: EntityId::new(),
            order: 3,
            label: "Review reversed range".into(),
            start: Some(CalendarDate {
                year: 2024,
                month: None,
                expected: false,
            }),
            end: Some(DateEnd::Date {
                value: CalendarDate {
                    year: 2023,
                    month: None,
                    expected: false,
                },
            }),
        },
        ResumeDate {
            id: EntityId::new(),
            order: 4,
            label: "OMITTED_EMPTY_DATE".into(),
            start: None,
            end: None,
        },
    ]
}
