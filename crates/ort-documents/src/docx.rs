//! Constrained, deterministic `WordprocessingML` output. Content is always data.
//! This does not read ZIP/XML inputs, invoke Word, fetch links, or enable import.
use ort_domain::{DocumentStyle, Link, ResumeDocument};

use crate::{InlineSpan, TextExportError, normalized, opc, parse_inline_text, render_plain_text};

pub const DOCX_FORMAT_VERSION: u16 = 1;
pub const DOCX_TEMPLATE_ID: &str = "plain_docx_v1";
pub const MAX_DOCX_BYTES: usize = 2 * 1024 * 1024;
const MAX_XML_BYTES: usize = 1024 * 1024;
const CONTENT_INDENT_TWIPS: u16 = 210;
mod header;

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
#[allow(clippy::too_many_lines)]
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
        style != DocumentStyle::Professional,
        false,
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
        let mut starts_with_table = false;
        for entry in &section.entries {
            let first_content = entries.0.is_empty() && !section.heading.trim().is_empty();
            let table = entry_rows(
                &mut entries,
                &mut relationships,
                &mut next_link,
                entry,
                style,
                first_content,
            )?;
            starts_with_table |= first_content && table;
            for field in entry
                .fields_for_role(ort_domain::FieldRole::Paragraph)
                .filter(|field| !field.value.trim().is_empty())
            {
                let body_text = &field.value;
                rich_paragraph(
                    &mut entries,
                    &mut relationships,
                    &mut next_link,
                    "Normal",
                    body_text,
                    false,
                    false,
                    false,
                    true,
                )?;
            }
            {
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
                        true,
                    )?;
                }
            }
            for link in &entry.links {
                hyperlink(&mut entries, &mut relationships, &mut next_link, link, true)?;
            }
        }
        if !entries.0.is_empty() {
            if style == DocumentStyle::Technical {
                let mut heading = spans(&section.heading.to_uppercase(), false, false)?;
                for span in &mut heading {
                    span.bold = false;
                }
                paragraph_with_spans(
                    &mut body,
                    &mut relationships,
                    &mut next_link,
                    "Heading1",
                    false,
                    &heading,
                    false,
                    starts_with_table.then_some(0),
                )?;
            } else {
                paragraph_with_spans(
                    &mut body,
                    &mut relationships,
                    &mut next_link,
                    "Heading1",
                    false,
                    &spans(&section.heading, true, false)?,
                    false,
                    starts_with_table.then_some(0),
                )?;
            }
            body.push(&entries.0)?;
        }
    }
    body.push(document_end(style))?;
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
        (
            "word/numbering.xml",
            if style == DocumentStyle::Plain {
                include_str!("docx/numbering.xml")
            } else {
                include_str!("docx/numbering_styled.xml")
            },
        ),
    ])
}

const fn document_end(style: DocumentStyle) -> &'static str {
    match style {
        DocumentStyle::Plain => DOCUMENT_END,
        DocumentStyle::Technical => TECHNICAL_DOCUMENT_END,
        DocumentStyle::Professional => PROFESSIONAL_DOCUMENT_END,
        DocumentStyle::Modern => MODERN_DOCUMENT_END,
    }
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
                false,
            )?;
        }
        for link in &document.contact.links {
            hyperlink(body, relationships, next_link, link, false)?;
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
            append_separator(&mut content, document.contact.divider.separator());
            content.extend(spans(&value, false, false)?);
        }
        for link in &document.contact.links {
            append_separator(&mut content, document.contact.divider.separator());
            content.push(link_span(link)?);
        }
        paragraph_with_spans(
            body,
            relationships,
            next_link,
            "Contact",
            false,
            &content,
            false,
            None,
        )?;
    }
    Ok(())
}

fn entry_rows(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    entry: &ort_domain::ResumeEntry,
    style: DocumentStyle,
    first_content: bool,
) -> Result<bool, DocxExportError> {
    let mut title = spans(&entry.heading, true, false)?;
    let details = entry.field_text(ort_domain::FieldRole::Details, " | ");
    if !details.is_empty() {
        append_separator(&mut title, " | ");
        title.extend(spans(&details, false, false)?);
    }
    let mut date_lines = Vec::new();
    if !entry.date_range.trim().is_empty() {
        date_lines.push(entry.date_range.clone());
    }
    date_lines.extend(
        entry
            .dates
            .iter()
            .flatten()
            .map(ort_domain::ResumeDate::display_text),
    );
    let dates = date_lines.join("\n");
    let extras = entry.field_text(ort_domain::FieldRole::Extra, "\n");
    let right_rows = [dates.as_str(), entry.location.as_str(), extras.as_str()]
        .into_iter()
        .map(|value| spans(value, false, false))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|runs| !runs.is_empty())
        .collect::<Vec<_>>();
    header::write(
        body,
        relationships,
        next_link,
        style,
        &title,
        &spans(&entry.subheading, false, false)?,
        &right_rows,
        first_content,
    )
}

#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn rich_paragraph(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    style: &str,
    text: &str,
    bullet: bool,
    force_bold: bool,
    force_italic: bool,
    content_indent: bool,
) -> Result<(), DocxExportError> {
    let text = normalized(text)?;
    if text.is_empty() {
        return Ok(());
    }
    let content = spans(&text, force_bold, force_italic)?;
    paragraph_with_spans(
        body,
        relationships,
        next_link,
        style,
        bullet,
        &content,
        content_indent,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn paragraph_with_spans(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    style: &str,
    bullet: bool,
    content: &[InlineSpan],
    content_indent: bool,
    space_after: Option<u16>,
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
    if let Some(after) = space_after {
        body.push(&format!("<w:spacing w:after=\"{after}\"/>"))?;
    } else if content_indent {
        body.push("<w:spacing w:after=\"20\"/>")?;
    }
    if content_indent && !bullet {
        body.push("<w:ind w:left=\"")?;
        body.push(&CONTENT_INDENT_TWIPS.to_string())?;
        body.push("\"/>")?;
    }
    body.push("</w:pPr>")?;
    write_spans(body, relationships, next_link, content)?;
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
    content_indent: bool,
) -> Result<(), DocxExportError> {
    // The domain has already allowed only http/https/mailto. Refuse controls
    // and whitespace instead of allowing URI parsers/XML to silently remove it.
    if link.url.chars().any(char::is_whitespace) {
        return Err(DocxExportError::InvalidDocument);
    }
    let content = [link_span(link)?];
    paragraph_with_spans(
        body,
        rels,
        next,
        "Normal",
        false,
        &content,
        content_indent,
        None,
    )
}

const DOCUMENT_START: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>";
const DOCUMENT_END: &str = "<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>";
const TECHNICAL_DOCUMENT_END: &str = "<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"675\" w:right=\"765\" w:bottom=\"675\" w:left=\"765\" w:header=\"432\" w:footer=\"432\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>";
const PROFESSIONAL_DOCUMENT_END: &str = "<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"675\" w:right=\"885\" w:bottom=\"675\" w:left=\"885\" w:header=\"432\" w:footer=\"432\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>";
const MODERN_DOCUMENT_END: &str = "<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"675\" w:right=\"855\" w:bottom=\"675\" w:left=\"855\" w:header=\"432\" w:footer=\"432\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>";
const RELS_START: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"styles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"numbering\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/>";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_contact_dividers_reach_styled_docx_content() {
        use ort_domain::ContactDivider;
        let mut document = ResumeDocument::empty("Synthetic");
        document.contact.email = "person@example.com".into();
        document.contact.location = "Boston".into();
        document.contact.links.push(Link {
            id: None,
            order: None,
            label: "Portfolio".into(),
            url: "https://example.com".into(),
        });
        for (divider, separator) in [
            (ContactDivider::Dot, "  •  "),
            (ContactDivider::Bar, "  |  "),
            (ContactDivider::Dash, "  -  "),
        ] {
            document.contact.divider = divider;
            for style in [
                DocumentStyle::Technical,
                DocumentStyle::Professional,
                DocumentStyle::Modern,
                DocumentStyle::Plain,
            ] {
                let mut body = Xml::default();
                let mut relationships = Xml::default();
                contact_paragraphs(&mut body, &mut relationships, &mut 0, &document, style)
                    .unwrap();
                assert_eq!(
                    body.0.matches(separator).count(),
                    if style == DocumentStyle::Plain { 0 } else { 2 }
                );
                assert!(body.0.contains("Portfolio"));
                assert!(relationships.0.contains("https://example.com"));
                render_docx_with_style(&document, style).unwrap();
            }
        }
    }

    #[test]
    fn xml_growth_is_checked_before_copying() {
        let mut xml = Xml::default();
        xml.push(&"x".repeat(MAX_XML_BYTES)).unwrap();
        assert_eq!(xml.escaped("&"), Err(DocxExportError::OutputTooLarge));
        assert_eq!(xml.0.len(), MAX_XML_BYTES);
    }
}
