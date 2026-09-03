//! Constrained, deterministic `WordprocessingML` output. Content is always data.
//! This does not read ZIP/XML inputs, invoke Word, fetch links, or enable import.
use ort_domain::{Link, ResumeDocument};

use crate::{TextExportError, normalized, opc, render_link, render_plain_text};

pub const DOCX_FORMAT_VERSION: u16 = 1;
pub const DOCX_TEMPLATE_ID: &str = "plain_docx_v1";
pub const MAX_DOCX_BYTES: usize = 2 * 1024 * 1024;
const MAX_XML_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocxExportError {
    InvalidDocument,
    UnsupportedCharacter,
    EmptyContent,
    OutputTooLarge,
}

impl From<TextExportError> for DocxExportError {
    fn from(error: TextExportError) -> Self {
        match error {
            TextExportError::InvalidDocument => Self::InvalidDocument,
            TextExportError::UnsupportedControlCharacter => Self::UnsupportedCharacter,
            TextExportError::EmptyContent => Self::EmptyContent,
            TextExportError::OutputTooLarge => Self::OutputTooLarge,
        }
    }
}

/// Generates six fixed OPC parts with semantic headings, real lists and explicit
/// hyperlinks. No internal title/IDs, timestamps, author, templates, fields,
/// macros, media, embedded files, or remote resources are packaged.
///
/// # Errors
/// Rejects invalid/empty canonical content, XML-invalid characters and bounded
/// output overflow. A failed render returns no partial document.
pub fn render_docx(document: &ResumeDocument) -> Result<Vec<u8>, DocxExportError> {
    // Shares text normalization/empty-content policy; validates before expansion.
    // This bounded temporary output is never persisted or returned to the UI.
    render_plain_text(document)?;
    let mut body = Xml::default();
    body.push(DOCUMENT_START)?;
    let mut relationships = Xml::default();
    relationships.push(RELS_START)?;
    let mut next_link = 0;
    paragraph(&mut body, "Title", &document.contact.full_name, false)?;
    for text in [
        &document.contact.email,
        &document.contact.phone,
        &document.contact.location,
    ] {
        paragraph(&mut body, "Normal", text, false)?;
    }
    for link in &document.contact.links {
        hyperlink(&mut body, &mut relationships, &mut next_link, link)?;
    }
    for section in &document.sections {
        let mut entries = Xml::default();
        for entry in &section.entries {
            paragraph(&mut entries, "Heading2", &entry.heading, false)?;
            for text in [&entry.subheading, &entry.date_range, &entry.location] {
                paragraph(&mut entries, "Normal", text, false)?;
            }
            for field in &entry.fields {
                let value = normalized(&field.value)?;
                if !value.is_empty() {
                    let label = normalized(&field.label)?;
                    let text = if label.is_empty() {
                        value
                    } else {
                        format!("{label}: {value}")
                    };
                    paragraph(&mut entries, "Normal", &text, false)?;
                }
            }
            for bullet in &entry.bullets {
                paragraph(&mut entries, "ListParagraph", &bullet.text, true)?;
            }
            for link in &entry.links {
                hyperlink(&mut entries, &mut relationships, &mut next_link, link)?;
            }
        }
        if !entries.0.is_empty() {
            paragraph(&mut body, "Heading1", &section.heading, false)?;
            body.push(&entries.0)?;
        }
    }
    body.push(DOCUMENT_END)?;
    relationships.push("</Relationships>")?;
    opc::package(&[
        (
            "[Content_Types].xml",
            include_str!("docx/content-types.xml"),
        ),
        ("_rels/.rels", include_str!("docx/root-rels.xml")),
        ("word/document.xml", &body.0),
        ("word/_rels/document.xml.rels", &relationships.0),
        ("word/styles.xml", include_str!("docx/styles.xml")),
        ("word/numbering.xml", include_str!("docx/numbering.xml")),
    ])
}

#[derive(Default)]
struct Xml(String);

impl Xml {
    fn push(&mut self, text: &str) -> Result<(), DocxExportError> {
        if text.len() > MAX_XML_BYTES - self.0.len() {
            return Err(DocxExportError::OutputTooLarge);
        }
        self.0.push_str(text);
        Ok(())
    }

    fn escaped(&mut self, text: &str) -> Result<(), DocxExportError> {
        for c in text.chars() {
            match c {
                '&' => self.push("&amp;")?,
                '<' => self.push("&lt;")?,
                '>' => self.push("&gt;")?,
                '"' => self.push("&quot;")?,
                '\'' => self.push("&apos;")?,
                '\u{fffe}' | '\u{ffff}' => return Err(DocxExportError::UnsupportedCharacter),
                c if c.is_control() => return Err(DocxExportError::UnsupportedCharacter),
                c => self.push(c.encode_utf8(&mut [0; 4]))?,
            }
        }
        Ok(())
    }

    fn text_run(&mut self, text: &str) -> Result<(), DocxExportError> {
        self.push("<w:r><w:t xml:space=\"preserve\">")?;
        // Preserve line breaks and tabs as OOXML semantics, not XML whitespace.
        for segment in text.split_inclusive(['\n', '\t']) {
            let content = segment.trim_end_matches(['\n', '\t']);
            self.escaped(content)?;
            if segment.ends_with('\n') {
                self.push("</w:t><w:br/><w:t xml:space=\"preserve\">")?;
            } else if segment.ends_with('\t') {
                self.push("</w:t><w:tab/><w:t xml:space=\"preserve\">")?;
            }
        }
        self.push("</w:t></w:r>")
    }
}

fn paragraph(body: &mut Xml, style: &str, text: &str, bullet: bool) -> Result<(), DocxExportError> {
    let text = normalized(text)?;
    if text.is_empty() {
        return Ok(());
    }
    body.push("<w:p><w:pPr><w:pStyle w:val=\"")?;
    body.push(style)?; // Only fixed internal style IDs, never user input.
    body.push("\"/>")?;
    if bullet {
        body.push("<w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr>")?;
    }
    body.push("</w:pPr>")?;
    body.text_run(&text)?;
    body.push("</w:p>")
}

fn hyperlink(
    body: &mut Xml,
    rels: &mut Xml,
    next: &mut usize,
    link: &Link,
) -> Result<(), DocxExportError> {
    // The domain has already allowed only http/https/mailto. Refuse controls
    // and whitespace instead of allowing URI parsers/XML to silently remove it.
    if link.url.chars().any(char::is_whitespace) {
        return Err(DocxExportError::InvalidDocument);
    }
    *next += 1;
    let id = format!("link{next}");
    rels.push("<Relationship Id=\"")?;
    rels.push(&id)?;
    rels.push("\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" TargetMode=\"External\" Target=\"")?;
    rels.escaped(&link.url)?;
    rels.push("\"/>")?;
    body.push("<w:p><w:hyperlink r:id=\"")?;
    body.push(&id)?;
    body.push("\" w:history=\"1\">")?;
    // Include the literal destination in visible text as well as the relation.
    body.text_run(&render_link(link)?)?;
    body.push("</w:hyperlink></w:p>")
}

const DOCUMENT_START: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>";
const DOCUMENT_END: &str = "<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>";
const RELS_START: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"styles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"numbering\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/>";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_growth_is_checked_before_copying() {
        let mut xml = Xml::default();
        xml.push(&"x".repeat(MAX_XML_BYTES)).unwrap();
        assert_eq!(xml.escaped("&"), Err(DocxExportError::OutputTooLarge));
        assert_eq!(xml.0.len(), MAX_XML_BYTES);
    }
}
