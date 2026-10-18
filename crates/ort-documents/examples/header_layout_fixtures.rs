//! Synthetic wrapping regression fixtures; never reads a user's resume.
#[path = "../tests/support/mod.rs"]
#[allow(dead_code)]
mod support;
use ort_domain::{DocumentStyle, EntityId, NamedField, ResumeSection};
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("new output directory required")?,
    );
    fs::create_dir(&root)?;
    let mut document = support::fixture("standard").upgraded_v2()?;
    document.sections[0].heading = "Education".into();
    let mut entry = document.sections[0].entries[0].clone();
    entry.heading = "Example State University".into();
    entry.subheading = "*B.S. in Computer Science*\n*Advanced Cybersecurity Experience for Students, Honors College*".into();
    entry.location = "College Park, MD".into();
    entry.date_range = "Aug. 2026–Dec. 2029".into();
    entry.dates = Some(vec![]);
    entry.fields = vec![NamedField {
        id: EntityId::new(),
        order: 0,
        is_skill: false,
        list_kind: None,
        label: "Extra".into(),
        value: "*GPA: 4.0/4.0*".into(),
    }];
    entry.bullets.truncate(1);
    entry.bullets[0].text = "Body content follows the full height of both header columns.".into();
    entry.links.clear();
    document.sections[0].entries = vec![entry.clone()];
    entry.id = EntityId::new();
    for field in &mut entry.fields {
        field.id = EntityId::new();
    }
    for bullet in &mut entry.bullets {
        bullet.id = EntityId::new();
    }
    entry.order = 1;
    entry.heading = "A deliberately long organization name with an extended division and research program that wraps onto another line".into();
    entry.subheading = "Full Stack Engineer with a deliberately long role description that wraps naturally without a manual line break and continues onto multiple lines".into();
    document.sections[0].entries.push(entry);
    let mut body_only = document.sections[0].entries[0].clone();
    body_only.id = EntityId::new();
    body_only.order = 0;
    body_only.heading.clear();
    body_only.subheading.clear();
    body_only.date_range.clear();
    body_only.location.clear();
    body_only.bullets.clear();
    body_only.fields = vec![NamedField {
        id: EntityId::new(),
        order: 0,
        is_skill: true,
        list_kind: None,
        label: "__ort_body_paragraph__".into(),
        value: "**Languages:** Java, Python, C, JavaScript, TypeScript, SQL, HTML/CSS\n**Frameworks:** React, Vite, Spring Boot, FastAPI".into(),
    }];
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 1,
        heading: "Technical Skills".into(),
        entries: vec![body_only],
    });
    for (name, style) in [
        ("plain", DocumentStyle::Plain),
        ("technical", DocumentStyle::Technical),
        ("professional", DocumentStyle::Professional),
        ("modern", DocumentStyle::Modern),
    ] {
        fs::write(
            root.join(format!("{name}.docx")),
            ort_documents::render_docx_with_style(&document, style)
                .map_err(|error| format!("render failed: {error:?}"))?,
        )?;
    }
    fs::write(
        root.join("source.json"),
        serde_json::to_vec_pretty(&document)?,
    )?;
    Ok(())
}
