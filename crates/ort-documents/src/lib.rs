//! Deterministic document output. Hostile-file parsing remains disabled.

use ort_domain::{DocumentLimits, Link, ResumeDocument};
use url::Url;

mod docx;
pub mod import;
pub mod import_source;
pub mod import_transport;
mod opc;
pub mod worker_output;
pub mod worker_supervisor;

pub use docx::{
    DOCX_FORMAT_VERSION, DOCX_TEMPLATE_ID, DocxExportError, MAX_DOCX_BYTES, render_docx,
    render_docx_with_style,
};

pub const IMPORT_ENABLED: bool = false;
pub const TEXT_FORMAT_VERSION: u16 = 1;
pub const MAX_TEXT_BYTES: usize = 256 * 1024;
const PARAGRAPH_FIELD_LABEL: &str = "__ort_body_paragraph__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub url: Option<String>,
}

/// Parses the small, literal inline-formatting language used by the editor.
/// Unsupported or incomplete markers remain visible text, matching the preview.
#[must_use]
pub fn parse_inline_text(value: &str) -> Vec<InlineSpan> {
    let mut spans = Vec::new();
    let mut at = 0;
    let mut literal_at = 0;
    while at < value.len() {
        if let Some((consumed, span)) = inline_token(&value[at..]) {
            push_inline_span(
                &mut spans,
                InlineSpan {
                    text: value[literal_at..at].to_owned(),
                    bold: false,
                    italic: false,
                    url: None,
                },
            );
            push_inline_span(&mut spans, span);
            at += consumed;
            literal_at = at;
        } else {
            at += value[at..].chars().next().map_or(1, char::len_utf8);
        }
    }
    push_inline_span(
        &mut spans,
        InlineSpan {
            text: value[literal_at..].to_owned(),
            bold: false,
            italic: false,
            url: None,
        },
    );
    spans
}

fn inline_token(value: &str) -> Option<(usize, InlineSpan)> {
    if let Some(rest) = value.strip_prefix("**") {
        if let Some(end) = rest.find("**") {
            let text = &rest[..end];
            if !text.is_empty() && !text.contains('*') {
                return Some((end + 4, formatted_span(text, true, false, None)));
            }
        }
    }
    if let Some(rest) = value.strip_prefix('*') {
        if !rest.starts_with('*') {
            if let Some(end) = rest.find('*') {
                let text = &rest[..end];
                if !text.is_empty() && !text.contains('*') {
                    return Some((end + 2, formatted_span(text, false, true, None)));
                }
            }
        }
    }
    if let Some(rest) = value.strip_prefix('[') {
        if let Some(label_end) = rest.find("](") {
            let label = &rest[..label_end];
            let url_and_end = &rest[label_end + 2..];
            if let Some(url_end) = url_and_end.find(')') {
                let address = &url_and_end[..url_end];
                if !label.is_empty()
                    && !address.is_empty()
                    && !address.chars().any(char::is_whitespace)
                    && safe_inline_url(address)
                {
                    return Some((
                        1 + label_end + 2 + url_end + 1,
                        formatted_span(label, false, false, Some(address.to_owned())),
                    ));
                }
            }
        }
    }
    None
}

fn formatted_span(text: &str, bold: bool, italic: bool, url: Option<String>) -> InlineSpan {
    InlineSpan {
        text: text.to_owned(),
        bold,
        italic,
        url,
    }
}

fn push_inline_span(spans: &mut Vec<InlineSpan>, span: InlineSpan) {
    if span.text.is_empty() {
        return;
    }
    if let Some(previous) = spans.last_mut() {
        if previous.bold == span.bold && previous.italic == span.italic && previous.url == span.url
        {
            previous.text.push_str(&span.text);
            return;
        }
    }
    spans.push(span);
}

fn safe_inline_url(value: &str) -> bool {
    Url::parse(value).is_ok_and(|parsed| matches!(parsed.scheme(), "http" | "https" | "mailto"))
}

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
            for date in entry.dates.iter().flatten() {
                push_nonempty(&mut lines, &date.display_text())?;
            }
            for field in &entry.fields {
                let value = normalized(&field.value)?;
                if value.is_empty() {
                    continue;
                }
                if field.label == PARAGRAPH_FIELD_LABEL {
                    lines.push(value);
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
            id: None,
            order: None,
            label: "Portfolio".to_owned(),
            url: "https://example.org".to_owned(),
        });
        document.sections.push(ResumeSection {
            id: EntityId::new(),
            order: 0,
            heading: "Experience".to_owned(),
            entries: vec![ResumeEntry {
                dates: None,
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
