//! Independent, top-aligned header columns keep wrapped roles away from metadata.
use super::{CONTENT_INDENT_TWIPS, DocumentStyle, DocxExportError, InlineSpan, Xml, write_spans};

#[allow(clippy::too_many_arguments)]
pub(super) fn write(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    style: DocumentStyle,
    title: &[InlineSpan],
    role: &[InlineSpan],
    metadata: &[Vec<InlineSpan>],
) -> Result<(), DocxExportError> {
    let has_left = !title.is_empty() || !role.is_empty();
    let columns = has_left && !metadata.is_empty();
    let width = match style {
        DocumentStyle::Plain => 9360,
        DocumentStyle::Technical => 10_710,
        DocumentStyle::Professional => 10_470,
        DocumentStyle::Modern => 10_530,
    } - CONTENT_INDENT_TWIPS;
    if columns {
        let right_width = width / 4;
        let left_width = width - right_width;
        body.push(&format!(
            "<w:tbl><w:tblPr><w:tblW w:w=\"{width}\" w:type=\"dxa\"/>\
             <w:tblInd w:w=\"{CONTENT_INDENT_TWIPS}\" w:type=\"dxa\"/>\
             <w:tblLayout w:type=\"fixed\"/>\
             <w:tblCellMar><w:top w:w=\"0\" w:type=\"dxa\"/>\
             <w:left w:w=\"0\" w:type=\"dxa\"/><w:bottom w:w=\"0\" w:type=\"dxa\"/>\
             <w:right w:w=\"0\" w:type=\"dxa\"/></w:tblCellMar></w:tblPr>\
             <w:tblGrid><w:gridCol w:w=\"{left_width}\"/><w:gridCol w:w=\"{right_width}\"/>\
             </w:tblGrid><w:tr>"
        ))?;
        cell_start(body, left_width, true)?;
    }
    for (content, heading) in [(title, true), (role, false)] {
        if !content.is_empty() {
            let line = if heading {
                Line::Title
            } else if title.is_empty() {
                Line::First
            } else {
                Line::Continuation
            };
            paragraph(
                body,
                relationships,
                next_link,
                content,
                line,
                false,
                !columns,
            )?;
        }
    }
    if columns {
        body.push("</w:tc>")?;
        cell_start(body, width / 4, false)?;
    }
    for (index, content) in metadata.iter().enumerate() {
        paragraph(
            body,
            relationships,
            next_link,
            content,
            if index == 0 {
                Line::First
            } else {
                Line::Continuation
            },
            true,
            !columns,
        )?;
    }
    if columns {
        body.push("</w:tc></w:tr></w:tbl>")?;
    }
    Ok(())
}

fn cell_start(body: &mut Xml, width: u16, left: bool) -> Result<(), DocxExportError> {
    body.push(&format!(
        "<w:tc><w:tcPr><w:tcW w:w=\"{width}\" w:type=\"dxa\"/><w:vAlign w:val=\"top\"/>"
    ))?;
    if left {
        body.push("<w:tcMar><w:right w:w=\"120\" w:type=\"dxa\"/></w:tcMar>")?;
    }
    body.push("</w:tcPr>")
}

#[derive(Clone, Copy)]
enum Line {
    Title,
    First,
    Continuation,
}

#[allow(clippy::too_many_arguments)]
fn paragraph(
    body: &mut Xml,
    relationships: &mut Xml,
    next_link: &mut usize,
    content: &[InlineSpan],
    line: Line,
    right: bool,
    indent: bool,
) -> Result<(), DocxExportError> {
    let style = if matches!(line, Line::Title) {
        "Heading2"
    } else {
        "Normal"
    };
    let before = if matches!(line, Line::Continuation) {
        0
    } else {
        40
    };
    body.push(&format!(
        "<w:p><w:pPr><w:pStyle w:val=\"{style}\"/><w:keepNext/>\
         <w:spacing w:before=\"{before}\" w:after=\"0\"/>"
    ))?;
    if right {
        body.push("<w:jc w:val=\"right\"/>")?;
    }
    if indent {
        body.push(&format!("<w:ind w:left=\"{CONTENT_INDENT_TWIPS}\"/>"))?;
    }
    body.push("</w:pPr>")?;
    write_spans(body, relationships, next_link, content)?;
    body.push("</w:p>")
}
