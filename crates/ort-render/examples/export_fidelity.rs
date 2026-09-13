//! Synthetic wrapping/reading-order checks; never reads a user's profile.
use ort_domain::{
    Bullet, CalendarDate, DocumentStyle, EntityId, NamedField, ResumeDate, ResumeDocument,
    ResumeEntry, ResumeSection,
};
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Provide an output directory")?,
    );
    fs::create_dir_all(&root)?;
    let mut document = ResumeDocument::empty("Export layout verification");
    document.schema_version = 2;
    document.contact.full_name = "Jordan Ellis".into();
    document.contact.email = "jordan.ellis@example.org".into();
    document.contact.location = "Portland, OR".into();
    let make_entry = |order, heading: &str, subtitle: &str| {
        ResumeEntry {
        id: EntityId::new(), order, heading: heading.into(), subheading: subtitle.into(),
        date_range: String::new(), location: "Portland metropolitan area, Oregon".into(),
        dates: Some(vec![ResumeDate { id: EntityId::new(), order: 0, label: "Graduation".into(),
            start: Some(CalendarDate { year: 2027, month: Some(6), expected: true }), end: None }]),
        fields: vec![NamedField { id: EntityId::new(), order: 0, label: "Extra".into(), value: "Remote work eligibility".into(), is_skill: false }],
        links: vec![], bullets: vec![Bullet { id: EntityId::new(), order: 0,
            text: "Built accessible reporting tools that preserve every customer record, improve review speed, and provide clear documentation for distributed engineering teams.".into() }],
    }
    };
    document.sections = vec![ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![
            make_entry(
                0,
                "Senior Software Engineer for Infrastructure and Developer Experience",
                "Engineering Systems Group with a role description that wraps onto another line",
            ),
            make_entry(1, "Community Coding", ""),
        ],
    }];
    let mut paragraph = make_entry(0, "", "");
    paragraph.location.clear();
    paragraph.dates = Some(vec![]);
    paragraph.bullets.clear();
    paragraph.fields = vec![NamedField {
        id: EntityId::new(),
        order: 0,
        label: "__ort_body_paragraph__".into(),
        value: "Languages: Rust, Python, TypeScript\nTools: Git, SQL, Linux".into(),
        is_skill: true,
    }];
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 1,
        heading: "Skills".into(),
        entries: vec![paragraph],
    });
    fs::write(
        root.join("edge-cases.source.json"),
        serde_json::to_vec_pretty(&document)?,
    )?;
    for (name, style) in [
        ("technical", DocumentStyle::Technical),
        ("professional", DocumentStyle::Professional),
        ("modern", DocumentStyle::Modern),
    ] {
        let pdf = ort_render::render_pdf_with_style(&document, style)?;
        fs::write(root.join(format!("edge-{name}.pdf")), pdf.bytes)?;
        fs::write(
            root.join(format!("edge-{name}.docx")),
            ort_documents::render_docx_with_style(&document, style)
                .map_err(|_| "DOCX rendering failed")?,
        )?;
        println!("{name}: {} PDF page(s)", pdf.receipt.page_count);
    }
    Ok(())
}
