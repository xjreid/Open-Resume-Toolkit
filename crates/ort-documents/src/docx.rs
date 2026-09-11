//! Constrained, deterministic `WordprocessingML` output. Content is always data.
//! This does not read ZIP/XML inputs, invoke Word, fetch links, or enable import.
use ort_domain::{DocumentStyle, Link, ResumeDocument};

use crate::{InlineSpan, TextExportError, normalized, opc, parse_inline_text, render_plain_text};

pub const DOCX_FORMAT_VERSION: u16 = 1;
pub const DOCX_TEMPLATE_ID: &str = "plain_docx_v1";
pub const MAX_DOCX_BYTES: usize = 2 * 1024 * 1024;
const MAX_XML_BYTES: usize = 1024 * 1024;
const PARAGRAPH_FIELD_LABEL: &str = "__ort_body_paragraph__";

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
    render_docx_with_style(document, DocumentStyle::Plain)
}

/// Generates the same constrained six-part package with a bundled presentation.
/// Content order, semantic headings, lists and hyperlinks remain unchanged.
///
/// # Errors
/// Applies the same validation and expansion bounds as plain DOCX output.
pub fn render_docx_with_style(
    document: &ResumeDocument,
    style: DocumentStyle,
) -> Result<Vec<u8>, DocxExportError> {
    // Shares text normalization/empty-content policy; validates before expansion.
    // This bounded temporary output is never persisted or returned to the UI.
    render_plain_text(document)?;
    let mut body = Xml::default();
    body.push(DOCUMENT_START)?;
    let mut relationships = Xml::default();
    relationships.push(RELS_START)?;
    let mut next_link = 0;
    rich_paragraph(
        &mut body,
        &mut relationships,
        &mut next_link,
        "Title",
        &document.contact.full_name,
        false,
        true,
        false,
    )?;
    contact_paragraphs(
        &mut body,
        &mut relationships,
        &mut next_link,
        document,
        style,
    )?;
    for section in &document.sections {
        let mut entries = Xml::default();
        for entry in &section.entries {
            entry_rows(
                &mut entries,
                &mut relationships,
                &mut next_link,
                entry,
                if style == DocumentStyle::Plain {
                    9360
                } else {
                    9792
                },
            )?;
            if let Some(body_text) = entry
                .fields
                .iter()
                .find(|field| field.label == PARAGRAPH_FIELD_LABEL)
                .map(|field| field.value.as_str())
                .filter(|value| !value.trim().is_empty())
            {
                rich_paragraph(
                    &mut entries,
                    &mut relationships,
                    &mut next_link,
                    "Normal",
                    body_text,
                    false,
                    false,
                    false,
                )?;
            } else {
                for bullet in &entry.bullets {
                    rich_paragraph(
                        &mut entries,
                        &mut relationships,
                        &mut next_link,
                        "ListParagraph",
                        &bullet.text,
                        true,
                        false,
                        false,
                    )?;
                }
            }
            for link in &entry.links {
                hyperlink(&mut entries, &mut relationships, &mut next_link, link)?;
            }
        }
        if !entries.0.is_empty() {
            rich_paragraph(
                &mut body,
                &mut relationships,
                &mut next_link,
                "Heading1",
                &section.heading,
                false,
                true,
                false,
            )?;
            body.push(&entries.0)?;
        }
    }
    if style == DocumentStyle::Plain {
        body.push(DOCUMENT_END)?;
    } else {
        body.push(&DOCUMENT_END.replace("1440", "1224"))?;
    }
    relationships.push("</Relationships>")?;
    opc::package(&[
        (
            "[Content_Types].xml",
            include_str!("docx/content-types.xml"),
        ),
        ("_rels/.rels", include_str!("docx/root-rels.xml")),
        ("word/document.xml", &body.0),
        ("word/_rels/document.xml.rels", &relationships.0),
        (
            "word/styles.xml",
            match style {
                DocumentStyle::Plain => include_str!("docx/styles.xml"),
                DocumentStyle::Technical => include_str!("docx/technical_v1.xml"),
                DocumentStyle::Professional => include_str!("docx/professional_v1.xml"),
                DocumentStyle::Modern => include_str!("docx/modern_v1.xml"),
            },
        ),
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

    fn text_run(
        &mut self,
        text: &str,
        bold: bool,
        italic: bool,
        hyperlink: bool,
    ) -> Result<(), DocxExportError> {
        self.push("<w:r><w:rPr>")?;
        self.push(if bold { "<w:b/>" } else { "<w:b w:val=\"0\"/>" })?;
        self.push(if italic {
            "<w:i/>"
        } else {
            "<w:i w:val=\"0\"/>"
        })?;
        if hyperlink {
            self.push("<w:color w:val=\"0563C1\"/><w:u w:val=\"single\"/>")?;
        }
        self.push("</w:rPr><w:t xml:space=\"preserve\">")?;
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

fn contact_paragraphs(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    document: &ResumeDocument,
    style: DocumentStyle,
) -> Result<(), DocxExportError> {
    if style == DocumentStyle::Plain {
        for text in [
            &document.contact.email,
            &document.contact.phone,
            &document.contact.location,
        ] {
            rich_paragraph(
                body,
                relationships,
                next_link,
                "Normal",
                text,
                false,
                false,
                false,
            )?;
        }
        for link in &document.contact.links {
            hyperlink(body, relationships, next_link, link)?;
        }
    } else {
        let mut content = Vec::new();
        for value in [
            &document.contact.email,
            &document.contact.phone,
            &document.contact.location,
        ] {
            let value = normalized(value)?;
            if value.is_empty() {
                continue;
            }
            append_separator(&mut content, "  •  ");
            content.extend(spans(&value, false, false)?);
        }
        for link in &document.contact.links {
            append_separator(&mut content, "  •  ");
            content.push(link_span(link)?);
        }
        paragraph_with_spans(body, relationships, next_link, "Contact", false, &content)?;
    }
    Ok(())
}

fn entry_rows(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    entry: &ort_domain::ResumeEntry,
    tab_position: u16,
) -> Result<(), DocxExportError> {
    let mut title = spans(&entry.heading, true, false)?;
    if let Some(details) = entry.fields.iter().find(|field| {
        field.label != PARAGRAPH_FIELD_LABEL
            && !field.label.trim().eq_ignore_ascii_case("extra")
            && !field.value.trim().is_empty()
    }) {
        append_separator(&mut title, " | ");
        title.extend(spans(&details.value, false, false)?);
    }
    paired_paragraph(
        body,
        relationships,
        next_link,
        "Heading2",
        &title,
        &spans(&entry.location, true, false)?,
        tab_position,
        false,
    )?;

    let dates = if entry.date_range.trim().is_empty() {
        entry
            .dates
            .iter()
            .flatten()
            .map(ort_domain::ResumeDate::display_text)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        entry.date_range.clone()
    };
    paired_paragraph(
        body,
        relationships,
        next_link,
        "Normal",
        &spans(&entry.subheading, false, false)?,
        &spans(&dates, false, false)?,
        tab_position,
        true,
    )?;

    let extras = entry
        .fields
        .iter()
        .filter(|field| {
            field.label.trim().eq_ignore_ascii_case("extra") && !field.value.trim().is_empty()
        })
        .map(|field| field.value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    paired_paragraph(
        body,
        relationships,
        next_link,
        "Normal",
        &[],
        &spans(&extras, false, false)?,
        tab_position,
        true,
    )
}

fn rich_paragraph(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    style: &str,
    text: &str,
    bullet: bool,
    force_bold: bool,
    force_italic: bool,
) -> Result<(), DocxExportError> {
    let text = normalized(text)?;
    if text.is_empty() {
        return Ok(());
    }
    let content = spans(&text, force_bold, force_italic)?;
    paragraph_with_spans(body, relationships, next_link, style, bullet, &content)
}

fn paragraph_with_spans(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    style: &str,
    bullet: bool,
    content: &[InlineSpan],
) -> Result<(), DocxExportError> {
    if content.is_empty() {
        return Ok(());
    }
    body.push("<w:p><w:pPr><w:pStyle w:val=\"")?;
    body.push(style)?; // Only fixed internal style IDs, never user input.
    body.push("\"/>")?;
    if bullet {
        body.push("<w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr>")?;
    }
    body.push("</w:pPr>")?;
    write_spans(body, relationships, next_link, content)?;
    body.push("</w:p>")
}

fn paired_paragraph(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    style: &str,
    left: &[InlineSpan],
    right: &[InlineSpan],
    tab_position: u16,
    compact: bool,
) -> Result<(), DocxExportError> {
    if left.is_empty() && right.is_empty() {
        return Ok(());
    }
    body.push("<w:p><w:pPr><w:pStyle w:val=\"")?;
    body.push(style)?;
    body.push("\"/>")?;
    if compact {
        body.push("<w:keepNext/><w:spacing w:after=\"20\"/>")?;
    }
    body.push("<w:tabs><w:tab w:val=\"right\" w:pos=\"")?;
    body.push(&tab_position.to_string())?;
    body.push("\"/></w:tabs></w:pPr>")?;
    write_spans(body, relationships, next_link, left)?;
    if !right.is_empty() {
        body.push("<w:r><w:tab/></w:r>")?;
        write_spans(body, relationships, next_link, right)?;
    }
    body.push("</w:p>")
}

fn spans(
    text: &str,
    force_bold: bool,
    force_italic: bool,
) -> Result<Vec<InlineSpan>, DocxExportError> {
    let text = normalized(text)?;
    Ok(parse_inline_text(&text)
        .into_iter()
        .map(|span| InlineSpan {
            bold: force_bold || span.bold,
            italic: force_italic || span.italic,
            ..span
        })
        .collect())
}

fn append_separator(content: &mut Vec<InlineSpan>, separator: &str) {
    if !content.is_empty() {
        content.push(InlineSpan {
            text: separator.to_owned(),
            bold: false,
            italic: false,
            url: None,
        });
    }
}

fn link_span(link: &Link) -> Result<InlineSpan, DocxExportError> {
    if link.url.chars().any(char::is_whitespace) {
        return Err(DocxExportError::InvalidDocument);
    }
    let url = normalized(&link.url)?;
    let label = normalized(&link.label)?;
    Ok(InlineSpan {
        text: if label.is_empty() { url.clone() } else { label },
        bold: false,
        italic: false,
        url: Some(url),
    })
}

fn write_spans(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    content: &[InlineSpan],
) -> Result<(), DocxExportError> {
    for span in content {
        if let Some(url) = &span.url {
            let id = relationship(relationships, next_link, url)?;
            body.push("<w:hyperlink r:id=\"")?;
            body.push(&id)?;
            body.push("\" w:history=\"1\">")?;
            body.text_run(&span.text, span.bold, span.italic, true)?;
            body.push("</w:hyperlink>")?;
        } else {
            body.text_run(&span.text, span.bold, span.italic, false)?;
        }
    }
    Ok(())
}

fn relationship(
    relationships: &mut Xml,
    next_link: &mut usize,
    url: &str,
) -> Result<String, DocxExportError> {
    if url.chars().any(char::is_whitespace) {
        return Err(DocxExportError::InvalidDocument);
    }
    *next_link += 1;
    let id = format!("link{next_link}");
    relationships.push("<Relationship Id=\"")?;
    relationships.push(&id)?;
    relationships.push("\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" TargetMode=\"External\" Target=\"")?;
    relationships.escaped(url)?;
    relationships.push("\"/>")?;
    Ok(id)
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
    let content = [link_span(link)?];
    paragraph_with_spans(body, rels, next, "Normal", false, &content)
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
