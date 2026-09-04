use ort_domain::{Bullet, EntityId, Link, NamedField, ResumeDocument, ResumeEntry, ResumeSection};

pub const OUTPUT_FIXTURE_KINDS: [&str; 8] = [
    "standard",
    "sparse",
    "unicode",
    "hostile",
    "dense",
    "optional",
    "structured",
    "paginated",
];

pub fn fixture(kind: &str) -> ResumeDocument {
    let mut doc = ResumeDocument::empty("INTERNAL_SYNTHETIC_TITLE_DO_NOT_EXPORT");
    doc.contact.full_name = "Zoë Example".into();
    doc.contact.email = "synthetic@example.org".into();
    doc.contact.phone = "+1 202 555 0100".into();
    doc.contact.location = "Example City".into();
    doc.contact.links.push(Link {
        label: "Portfolio".into(),
        url: "https://example.org/work?a=1&b=2".into(),
    });
    doc.sections.push(ResumeSection {
        id: EntityId::new(), order: 0, heading: "Experience".into(),
        entries: vec![ResumeEntry {
            id: EntityId::new(), order: 0, heading: "Software Engineer".into(),
            subheading: "Synthetic Research Cooperative".into(), date_range: "2023–2026".into(),
            location: "Remote".into(),
            fields: vec![NamedField { id: EntityId::new(), order: 0, label: "Tools".into(), value: "Rust, TypeScript & SQL".into(), is_skill: true }],
            bullets: vec![
                Bullet { id: EntityId::new(), order: 0, text: "Built an offline document workflow with explicit review and recovery.".into() },
                Bullet { id: EntityId::new(), order: 1, text: "Tested Unicode, links, and multi-line content.\r\nRetained a second line\twith a tab.".into() },
            ],
            links: vec![Link { label: "Project details".into(), url: "https://example.org/project".into() }],
        }],
    });
    match kind {
        "sparse" => {
            doc.sections.clear();
            doc.contact.links.clear();
        }
        "unicode" => {
            doc.contact.full_name = "Zoë García — Élise".into();
            doc.sections[0].heading = "Expérience / Ελληνικά".into();
            doc.sections[0].entries[0].fields[0].value =
                "Français, Español, Ελληνικά, Русский".into();
        }
        "hostile" => {
            doc.contact.full_name = "<script> & \"quoted\" 'literal'".into();
            doc.sections[0].entries[0].bullets[0].text =
                r#"#read("/secret") #include "secret" #panic("fail") </w:t><w:object>INCLUDETEXT file:///secret</w:object> $(command)"#.into();
        }
        "dense" => {
            for index in 2..42 {
                doc.sections[0].entries[0].bullets.push(Bullet {
                    id: EntityId::new(), order: index,
                    text: format!("Synthetic contribution {index}: verified ordering, safe links, and careful recovery across a deliberately multi-page document. Wrapped lines must remain readable and no content may disappear at a page boundary."),
                });
            }
        }
        "optional" => {
            doc.contact.email.clear();
            doc.contact.phone.clear();
            doc.contact.location.clear();
            doc.contact.links.clear();
            let entry = &mut doc.sections[0].entries[0];
            entry.subheading.clear();
            entry.date_range.clear();
            entry.location.clear();
            entry.fields.clear();
            entry.bullets.clear();
            entry.links.clear();
            doc.sections.push(ResumeSection {
                id: EntityId::new(),
                order: 1,
                heading: "Empty synthetic section".into(),
                entries: vec![],
            });
        }
        "structured" => {
            add_structured_content(&mut doc);
        }
        "paginated" => {
            for index in 2..20 {
                doc.sections[0].entries[0].bullets.push(Bullet {
                    id: EntityId::new(),
                    order: index,
                    text: format!(
                        "Pagination boundary item {index}: deterministic wrapped content remains in reading order without clipping, overlap, or loss."
                    ),
                });
            }
        }
        "standard" => {}
        _ => panic!("unknown synthetic fixture"),
    }
    doc
}

fn add_structured_content(doc: &mut ResumeDocument) {
    doc.contact.links.extend([
        Link {
            label: "Email".into(),
            url: "mailto:synthetic@example.org".into(),
        },
        Link {
            label: "Plain profile".into(),
            url: "https://example.org/plain".into(),
        },
    ]);
    doc.sections[0].entries.push(ResumeEntry {
        id: EntityId::new(),
        order: 1,
        heading: "Product Analyst".into(),
        subheading: String::new(),
        date_range: "2021–2023".into(),
        location: String::new(),
        fields: vec![NamedField {
            id: EntityId::new(),
            order: 0,
            label: "Certification".into(),
            value: "Synthetic credential".into(),
            is_skill: false,
        }],
        bullets: vec![Bullet {
            id: EntityId::new(),
            order: 0,
            text: "Compared ordered results without relying on color alone.".into(),
        }],
        links: vec![],
    });
    doc.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 1,
        heading: "Projects & Community".into(),
        entries: vec![ResumeEntry {
            id: EntityId::new(),
            order: 0,
            heading: "Open tooling".into(),
            subheading: "Synthetic maintainers group".into(),
            date_range: String::new(),
            location: "Hybrid".into(),
            fields: vec![NamedField {
                id: EntityId::new(),
                order: 0,
                label: String::new(),
                value: "Community-maintained".into(),
                is_skill: false,
            }],
            bullets: vec![Bullet {
                id: EntityId::new(),
                order: 0,
                text: "Preserved résumé ordering, optional values, and links.".into(),
            }],
            links: vec![Link {
                label: "Project mail".into(),
                url: "mailto:project@example.org".into(),
            }],
        }],
    });
}
