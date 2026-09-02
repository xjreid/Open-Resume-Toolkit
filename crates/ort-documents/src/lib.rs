//! Deterministic document output. Hostile-file parsing remains disabled.

use ort_domain::{DocumentLimits, Link, ResumeDocument};

pub const IMPORT_ENABLED: bool = false;
pub const TEXT_FORMAT_VERSION: u16 = 1;
pub const MAX_TEXT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextExportError {
    InvalidDocument,
    UnsupportedControlCharacter,
    EmptyContent,
    OutputTooLarge,
}

/// Produces UTF-8 text, LF line endings, and one final newline. Internal title,
/// IDs, revisions, branding, and the skill flag are not professional content.
///
/// # Errors
/// Rejects invalid documents, empty output, controls, or oversized output.
pub fn render_plain_text(document: &ResumeDocument) -> Result<String, TextExportError> {
    document
        .validate(DocumentLimits::default())
        .map_err(|_| TextExportError::InvalidDocument)?;
    let mut blocks = Vec::new();
    let mut contact = Vec::new();
    for value in [
        &document.contact.full_name,
        &document.contact.email,
        &document.contact.phone,
        &document.contact.location,
    ] {
        push_nonempty(&mut contact, value)?;
    }
    for link in &document.contact.links {
        push_nonempty(&mut contact, &render_link(link)?)?;
    }
    if !contact.is_empty() {
        blocks.push(contact.join("\n"));
    }
    for section in &document.sections {
        let mut entries = Vec::new();
        for entry in &section.entries {
            let mut lines = Vec::new();
            for value in [
                &entry.heading,
                &entry.subheading,
                &entry.date_range,
                &entry.location,
            ] {
                push_nonempty(&mut lines, value)?;
            }
            for field in &entry.fields {
                let value = normalized(&field.value)?;
                if value.is_empty() {
                    continue;
                }
                let label = normalized(&field.label)?;
                lines.push(if label.is_empty() {
                    value
                } else {
                    format!("{label}: {value}")
                });
            }
            for bullet in &entry.bullets {
                let text = normalized(&bullet.text)?;
                if !text.is_empty() {
                    lines.push(format!("- {}", text.replace('\n', "\n  ")));
                }
            }
            for link in &entry.links {
                push_nonempty(&mut lines, &render_link(link)?)?;
            }
            if !lines.is_empty() {
                entries.push(lines.join("\n"));
            }
        }
        if !entries.is_empty() {
            blocks.push(format!(
                "{}\n{}",
                normalized(&section.heading)?,
                entries.join("\n\n")
            ));
        }
    }
    if blocks.is_empty() {
        return Err(TextExportError::EmptyContent);
    }
    let text = format!("{}\n", blocks.join("\n\n"));
    if text.len() > MAX_TEXT_BYTES {
        return Err(TextExportError::OutputTooLarge);
    }
    Ok(text)
}

fn normalized(value: &str) -> Result<String, TextExportError> {
    if value
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(TextExportError::UnsupportedControlCharacter);
    }
    Ok(value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_owned())
}

fn push_nonempty(lines: &mut Vec<String>, value: &str) -> Result<(), TextExportError> {
    let text = normalized(value)?;
    if !text.is_empty() {
        lines.push(text);
    }
    Ok(())
}

fn render_link(link: &Link) -> Result<String, TextExportError> {
    let label = normalized(&link.label)?;
    let url = normalized(&link.url)?;
    Ok(if label.is_empty() || label == url {
        url
    } else {
        format!("{label}: {url}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_domain::{Bullet, EntityId, NamedField, ResumeEntry, ResumeSection};

    fn sample() -> ResumeDocument {
        let mut document = ResumeDocument::empty("Internal synthetic title");
        document.contact.full_name = "Zoë Example 示例".to_owned();
        document.contact.email = "example@example.org".to_owned();
        document.contact.links.push(Link {
            label: "Portfolio".to_owned(),
            url: "https://example.org".to_owned(),
        });
        document.sections.push(ResumeSection {
            id: EntityId::new(),
            order: 0,
            heading: "Experience".to_owned(),
            entries: vec![ResumeEntry {
                id: EntityId::new(),
                order: 0,
                heading: "Engineer".to_owned(),
                subheading: "Example Org".to_owned(),
                date_range: String::new(),
                location: String::new(),
                fields: vec![NamedField {
                    id: EntityId::new(),
                    order: 0,
                    label: "Language".to_owned(),
                    value: "Rust".to_owned(),
                    is_skill: true,
                }],
                bullets: vec![Bullet {
                    id: EntityId::new(),
                    order: 0,
                    text: "First line\r\nSecond line".to_owned(),
                }],
                links: vec![],
            }],
        });
        document
    }

    #[test]
    fn golden_output_is_deterministic_unicode_and_canonical() {
        let document = sample();
        let expected = "Zoë Example 示例\nexample@example.org\nPortfolio: https://example.org\n\nExperience\nEngineer\nExample Org\nLanguage: Rust\n- First line\n  Second line\n";
        assert_eq!(render_plain_text(&document).expect("render"), expected);
        assert_eq!(render_plain_text(&document).expect("repeat"), expected);
    }

    #[test]
    fn omits_empty_fields_and_internal_metadata() {
        let mut document = sample();
        document.sections[0].entries[0].fields[0].value.clear();
        let output = render_plain_text(&document).expect("render");
        assert!(!output.contains("Language"));
        assert!(!output.contains(&document.title));
        assert!(!output.contains(&document.document_id.to_string()));
        assert_eq!(
            render_plain_text(&ResumeDocument::empty("Only title")),
            Err(TextExportError::EmptyContent)
        );
    }

    #[test]
    fn rejects_invalid_input_and_keeps_code_like_content_literal() {
        let mut document = sample();
        document.sections[0].entries[0].heading =
            "<script>alert(1)</script> $(command) #include".to_owned();
        assert!(
            render_plain_text(&document)
                .expect("literal text")
                .contains("<script>alert(1)</script> $(command) #include")
        );
        document.contact.full_name.push('\u{1b}');
        assert_eq!(
            render_plain_text(&document),
            Err(TextExportError::UnsupportedControlCharacter)
        );
        document.contact.full_name.clear();
        document.sections[0].order = 1;
        assert_eq!(
            render_plain_text(&document),
            Err(TextExportError::InvalidDocument)
        );
    }
}
