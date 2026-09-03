use ort_domain::{Bullet, EntityId, Link, NamedField, ResumeDocument, ResumeEntry, ResumeSection};

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
            doc.contact.full_name = "Zoë Example 示例".into();
            doc.sections[0].heading = "Expérience / 経験".into();
            doc.sections[0].entries[0].fields[0].value =
                "Français, Español, 中文, 日本語, Ελληνικά".into();
        }
        "hostile" => {
            doc.contact.full_name = "<script> & \"quoted\" 'literal'".into();
            doc.sections[0].entries[0].bullets[0].text =
                "</w:t><w:object>INCLUDETEXT file:///secret</w:object> #include $(command)".into();
        }
        "dense" => {
            for index in 2..42 {
                doc.sections[0].entries[0].bullets.push(Bullet {
                    id: EntityId::new(), order: index,
                    text: format!("Synthetic contribution {index}: verified ordering, safe links, and careful recovery across a deliberately multi-page document. Wrapped lines must remain readable and no content may disappear at a page boundary."),
                });
            }
        }
        "standard" => {}
        _ => panic!("unknown synthetic fixture"),
    }
    doc
}
