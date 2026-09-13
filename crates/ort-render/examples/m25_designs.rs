//! Small, synthetic visual-review set; never reads a profile or credentials.
use ort_domain::{
    Bullet, DocumentStyle, EntityId, Link, NamedField, ResumeDocument, ResumeEntry, ResumeSection,
};
use std::{fs, path::PathBuf};

fn entry(title: &str, subtitle: &str, dates: &str, bullets: &[&str]) -> ResumeEntry {
    ResumeEntry {
        id: EntityId::new(),
        order: 0,
        heading: title.into(),
        subheading: subtitle.into(),
        date_range: dates.into(),
        location: "Portland, OR".into(),
        dates: None,
        fields: vec![
            NamedField {
                id: EntityId::new(),
                order: 0,
                label: "Skills / details".into(),
                value: "**Rust**, *Typst*, SQL, Spring Boot, FastAPI, PostgreSQL".into(),
                is_skill: true,
            },
            NamedField {
                id: EntityId::new(),
                order: 1,
                label: "Extra".into(),
                value: "Remote".into(),
                is_skill: false,
            },
        ],
        links: vec![],
        bullets: bullets
            .iter()
            .enumerate()
            .map(|(order, text)| Bullet {
                id: EntityId::new(),
                order: u16::try_from(order).expect("bounded synthetic fixture"),
                text: (*text).into(),
            })
            .collect(),
    }
}
fn section(order: u16, heading: &str, mut entries: Vec<ResumeEntry>) -> ResumeSection {
    for (index, entry) in entries.iter_mut().enumerate() {
        entry.order = u16::try_from(index).expect("bounded synthetic fixture");
    }
    ResumeSection {
        id: EntityId::new(),
        order,
        heading: heading.into(),
        entries,
    }
}
fn representative() -> ResumeDocument {
    let mut doc = ResumeDocument::empty("Synthetic design review");
    doc.contact.full_name = "Alex Morgan".into();
    doc.contact.email = "[alex.morgan@example.org](mailto:alex.morgan@example.org)".into();
    doc.contact.phone = "+1 (202) 555-0142".into();
    doc.contact.location = "Portland, OR".into();
    doc.contact.links = vec![
        Link {
            id: None,
            order: None,
            label: "Portfolio".into(),
            url: "https://example.org".into(),
        },
        Link {
            id: None,
            order: None,
            label: "LinkedIn".into(),
            url: "https://www.linkedin.com/in/example".into(),
        },
    ];
    doc.sections = vec![
        section(
            0,
            "Experience",
            vec![
                entry(
                    "Senior Software Engineer",
                    "Example Studio",
                    "2022–Present",
                    &[
                        "Led the development of [accessible planning tools](https://example.org/planning) for cross-functional teams.",
                        "Reduced report preparation time by 35% through a reusable data validation workflow.",
                        "Partnered with design and customer teams to simplify onboarding and document editing.",
                    ],
                ),
                entry(
                    "Software Engineer",
                    "Northstar Example Cooperative",
                    "2019–2022",
                    &[
                        "Built reliable services in TypeScript and SQL with clear operational documentation.",
                        "Introduced automated checks for data quality and keyboard accessibility.",
                    ],
                ),
            ],
        ),
        section(
            1,
            "Projects",
            vec![entry(
                "Community Resource Directory",
                "Independent project",
                "2024",
                &[
                    "Designed a searchable directory that helps residents find local support services.",
                    "Implemented a responsive interface, clear content hierarchy, and accessible navigation.",
                ],
            )],
        ),
        section(
            2,
            "Education",
            vec![entry(
                "Example State University",
                "B.S. in Computer Science",
                "2015–2019",
                &["Coursework: software engineering, data structures, human-computer interaction."],
            )],
        ),
        section(
            3,
            "Skills",
            vec![entry(
                "Languages and tools",
                "TypeScript, Rust, Python, SQL, Git, Figma",
                "",
                &[
                    "Accessibility, user research, technical writing, and cross-functional collaboration.",
                ],
            )],
        ),
    ];
    doc.sections[0].entries[1].location.clear();
    doc
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Provide an output directory")?,
    );
    fs::create_dir_all(&root)?;
    let mut doc = representative();
    fs::write(
        root.join("representative.source.json"),
        serde_json::to_vec_pretty(&doc)?,
    )?;
    for (name, style) in [
        ("technical", DocumentStyle::Technical),
        ("professional", DocumentStyle::Professional),
        ("modern", DocumentStyle::Modern),
    ] {
        let artifact = ort_render::render_pdf_with_style(&doc, style)?;
        fs::write(root.join(format!("{name}.pdf")), artifact.bytes)?;
        fs::write(
            root.join(format!("{name}.docx")),
            ort_documents::render_docx_with_style(&doc, style)
                .map_err(|_| "DOCX rendering failed")?,
        )?;
        println!("{name}: {} PDF page(s)", artifact.receipt.page_count);
    }
    for index in 0..9 {
        doc.sections[0].entries.push(entry(&format!("Additional project {}",index+1),"Synthetic pagination review","2017–2019",&[
            "Coordinated an iterative delivery process with regular review of content and accessibility.",
            "Documented decisions, tested recovery paths, and maintained clear handoffs across the team.",
            "Preserved complete source information when preparing longer professional documents.",
        ]));
    }
    for (i, e) in doc.sections[0].entries.iter_mut().enumerate() {
        e.order = u16::try_from(i).expect("bounded synthetic fixture");
    }
    let artifact = ort_render::render_pdf_with_style(&doc, DocumentStyle::Technical)?;
    fs::write(root.join("technical-long.pdf"), artifact.bytes)?;
    fs::write(
        root.join("technical-long.docx"),
        ort_documents::render_docx_with_style(&doc, DocumentStyle::Technical)
            .map_err(|_| "DOCX rendering failed")?,
    )?;
    println!(
        "technical-long: {} PDF page(s)",
        artifact.receipt.page_count
    );
    Ok(())
}
