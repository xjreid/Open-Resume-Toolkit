//! Constrained DOCX text extraction for the disposable parser worker.

use std::collections::HashSet;
use std::io::{Cursor, Read};

use flate2::read::DeflateDecoder;
use ort_documents::import::{BlockKind, InputFormat, MAX_BLOCK_CHARACTERS};
use ort_documents::import_source::{
    MAX_DOCX_ENTRIES, MAX_IMPORT_SOURCE_BYTES, SourceError, inspect_source,
};
use ort_documents::worker_output::{WorkerExtractionBuilder, WorkerOutputError};
use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};

const ZIP_CENTRAL_HEADER: u32 = 0x0201_4b50;
const ZIP_END: u32 = 0x0605_4b50;
const ZIP_LOCAL_HEADER: u32 = 0x0403_4b50;
const ZIP_DATA_DESCRIPTOR: u32 = 0x0807_4b50;
const MAX_DOCUMENT_XML_BYTES: usize = 2 * 1024 * 1024;
const MAX_RELATIONSHIP_XML_BYTES: usize = 256 * 1024;
const MAX_PACKAGE_METADATA_XML_BYTES: usize = 256 * 1024;
const MAX_XML_DEPTH: usize = 64;
const MAX_XML_EVENTS: usize = 200_000;
const MAX_RELATIONSHIPS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DocxParseError {
    #[error("DOCX input exceeds its configured limit")]
    InputLimit,
    #[error("DOCX package is malformed or unsupported")]
    InvalidPackage,
    #[error("DOCX compressed content exceeds its configured limit")]
    ExpansionLimit,
    #[error("DOCX XML is malformed or exceeds its configured complexity")]
    InvalidXml,
    #[error("DOCX contains unsupported active or external content")]
    ActiveContent,
    #[error("DOCX contains no readable text; OCR is not available")]
    NoReadableText,
    #[error("DOCX extraction exceeds the worker protocol limit")]
    OutputLimit,
    #[error("DOCX input could not be read")]
    InputRead,
}

#[derive(Clone, Copy)]
struct ZipEntry<'a> {
    name: &'a str,
    flags: u16,
    method: u16,
    crc: u32,
    compressed: usize,
    expanded: usize,
    local_offset: usize,
    data_limit: usize,
}

/// Reads one already-open DOCX handle, validates the package independently,
/// extracts only visible `WordprocessingML` text and emits extraction wire v1.
/// It never resolves a path, relationship target, URI, field, or external part.
///
/// # Errors
/// Fails closed on I/O, package, compression, XML, active-content, or protocol
/// limits. No partial extraction is returned.
pub fn extract_docx(input: &mut impl Read) -> Result<Vec<u8>, DocxParseError> {
    let mut bytes = Vec::new();
    input
        .take(u64::try_from(MAX_IMPORT_SOURCE_BYTES).unwrap_or(u64::MAX) + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DocxParseError::InputRead)?;
    if bytes.is_empty() || bytes.len() > MAX_IMPORT_SOURCE_BYTES {
        return Err(DocxParseError::InputLimit);
    }
    inspect_source(&bytes, InputFormat::Docx).map_err(map_source_error)?;

    let entries = catalog(&bytes)?;
    let content_types = required_part(
        &bytes,
        &entries,
        "[Content_Types].xml",
        MAX_PACKAGE_METADATA_XML_BYTES,
    )?;
    validate_content_types(&content_types)?;
    let root_relationships = required_part(
        &bytes,
        &entries,
        "_rels/.rels",
        MAX_PACKAGE_METADATA_XML_BYTES,
    )?;
    validate_relationships(&root_relationships, RelationshipScope::Package)?;
    let document = required_part(
        &bytes,
        &entries,
        "word/document.xml",
        MAX_DOCUMENT_XML_BYTES,
    )?;
    if let Some(entry) = entries
        .iter()
        .find(|entry| entry.name == "word/_rels/document.xml.rels")
    {
        let relationships = inflate_part(&bytes, *entry, MAX_RELATIONSHIP_XML_BYTES)?;
        validate_relationships(&relationships, RelationshipScope::Document)?;
    }
    extract_document_xml(&document)
}

fn catalog(bytes: &[u8]) -> Result<Vec<ZipEntry<'_>>, DocxParseError> {
    let eocd = find_eocd(bytes)?;
    let count = usize::from(read_u16(bytes, eocd + 10)?);
    let central_size =
        usize::try_from(read_u32(bytes, eocd + 12)?).map_err(|_| DocxParseError::InvalidPackage)?;
    let mut cursor =
        usize::try_from(read_u32(bytes, eocd + 16)?).map_err(|_| DocxParseError::InvalidPackage)?;
    let central_end = cursor
        .checked_add(central_size)
        .filter(|end| *end == eocd)
        .ok_or(DocxParseError::InvalidPackage)?;
    if count == 0 || count > MAX_DOCX_ENTRIES {
        return Err(DocxParseError::InvalidPackage);
    }
    let mut entries = Vec::new();
    entries
        .try_reserve(count)
        .map_err(|_| DocxParseError::InputLimit)?;
    for _ in 0..count {
        if read_u32(bytes, cursor)? != ZIP_CENTRAL_HEADER {
            return Err(DocxParseError::InvalidPackage);
        }
        let flags = read_u16(bytes, cursor + 8)?;
        let method = read_u16(bytes, cursor + 10)?;
        let crc = read_u32(bytes, cursor + 16)?;
        let compressed = usize::try_from(read_u32(bytes, cursor + 20)?)
            .map_err(|_| DocxParseError::InvalidPackage)?;
        let expanded = usize::try_from(read_u32(bytes, cursor + 24)?)
            .map_err(|_| DocxParseError::InvalidPackage)?;
        let name_len = usize::from(read_u16(bytes, cursor + 28)?);
        let extra_len = usize::from(read_u16(bytes, cursor + 30)?);
        let comment_len = usize::from(read_u16(bytes, cursor + 32)?);
        let local_offset = usize::try_from(read_u32(bytes, cursor + 42)?)
            .map_err(|_| DocxParseError::InvalidPackage)?;
        let name_start = cursor
            .checked_add(46)
            .ok_or(DocxParseError::InvalidPackage)?;
        let name_end = name_start
            .checked_add(name_len)
            .ok_or(DocxParseError::InvalidPackage)?;
        let record_end = name_end
            .checked_add(extra_len)
            .and_then(|end| end.checked_add(comment_len))
            .filter(|end| *end <= central_end)
            .ok_or(DocxParseError::InvalidPackage)?;
        let name = std::str::from_utf8(
            bytes
                .get(name_start..name_end)
                .ok_or(DocxParseError::InvalidPackage)?,
        )
        .map_err(|_| DocxParseError::InvalidPackage)?;
        entries.push(ZipEntry {
            name,
            flags,
            method,
            crc,
            compressed,
            expanded,
            local_offset,
            data_limit: central_end - central_size,
        });
        cursor = record_end;
    }
    if cursor != central_end {
        return Err(DocxParseError::InvalidPackage);
    }
    Ok(entries)
}

fn required_part(
    bytes: &[u8],
    entries: &[ZipEntry<'_>],
    name: &str,
    limit: usize,
) -> Result<Vec<u8>, DocxParseError> {
    let entry = entries
        .iter()
        .find(|entry| entry.name == name)
        .copied()
        .ok_or(DocxParseError::InvalidPackage)?;
    inflate_part(bytes, entry, limit)
}

fn inflate_part(
    bytes: &[u8],
    entry: ZipEntry<'_>,
    limit: usize,
) -> Result<Vec<u8>, DocxParseError> {
    if entry.expanded > limit {
        return Err(DocxParseError::ExpansionLimit);
    }
    if read_u32(bytes, entry.local_offset)? != ZIP_LOCAL_HEADER
        || read_u16(bytes, entry.local_offset + 6)? != entry.flags
        || read_u16(bytes, entry.local_offset + 8)? != entry.method
    {
        return Err(DocxParseError::InvalidPackage);
    }
    let name_len = usize::from(read_u16(bytes, entry.local_offset + 26)?);
    let extra_len = usize::from(read_u16(bytes, entry.local_offset + 28)?);
    let name_start = entry
        .local_offset
        .checked_add(30)
        .ok_or(DocxParseError::InvalidPackage)?;
    let name_end = name_start
        .checked_add(name_len)
        .ok_or(DocxParseError::InvalidPackage)?;
    if bytes.get(name_start..name_end) != Some(entry.name.as_bytes()) {
        return Err(DocxParseError::InvalidPackage);
    }
    let data_start = entry
        .local_offset
        .checked_add(30 + name_len)
        .and_then(|offset| offset.checked_add(extra_len))
        .ok_or(DocxParseError::InvalidPackage)?;
    let data_end = data_start
        .checked_add(entry.compressed)
        .filter(|end| *end <= entry.data_limit)
        .ok_or(DocxParseError::InvalidPackage)?;
    validate_local_sizes(bytes, entry, data_end)?;
    let compressed = bytes
        .get(data_start..data_end)
        .ok_or(DocxParseError::InvalidPackage)?;
    let mut output = Vec::new();
    output
        .try_reserve(entry.expanded.min(limit))
        .map_err(|_| DocxParseError::ExpansionLimit)?;
    match entry.method {
        0 => {
            if entry.compressed != entry.expanded {
                return Err(DocxParseError::InvalidPackage);
            }
            output.extend_from_slice(compressed);
        }
        8 => {
            let mut decoder = DeflateDecoder::new(Cursor::new(compressed));
            decoder
                .by_ref()
                .take(u64::try_from(limit).unwrap_or(u64::MAX) + 1)
                .read_to_end(&mut output)
                .map_err(|_| DocxParseError::InvalidPackage)?;
            if decoder.total_in() != u64::try_from(compressed.len()).unwrap_or(u64::MAX) {
                return Err(DocxParseError::InvalidPackage);
            }
        }
        _ => return Err(DocxParseError::InvalidPackage),
    }
    if output.len() != entry.expanded || output.len() > limit {
        return Err(DocxParseError::ExpansionLimit);
    }
    if crc32fast::hash(&output) != entry.crc {
        return Err(DocxParseError::InvalidPackage);
    }
    Ok(output)
}

fn validate_local_sizes(
    bytes: &[u8],
    entry: ZipEntry<'_>,
    data_end: usize,
) -> Result<(), DocxParseError> {
    if entry.flags & 0x0008 == 0 {
        let local_compressed = usize::try_from(read_u32(bytes, entry.local_offset + 18)?)
            .map_err(|_| DocxParseError::InvalidPackage)?;
        let local_expanded = usize::try_from(read_u32(bytes, entry.local_offset + 22)?)
            .map_err(|_| DocxParseError::InvalidPackage)?;
        if read_u32(bytes, entry.local_offset + 14)? != entry.crc
            || local_compressed != entry.compressed
            || local_expanded != entry.expanded
        {
            return Err(DocxParseError::InvalidPackage);
        }
        return Ok(());
    }
    let signature = read_u32(bytes, data_end)?;
    let descriptor_start = if signature == ZIP_DATA_DESCRIPTOR {
        data_end + 4
    } else {
        data_end
    };
    let descriptor_end = descriptor_start
        .checked_add(12)
        .filter(|end| *end <= entry.data_limit)
        .ok_or(DocxParseError::InvalidPackage)?;
    let compressed = usize::try_from(read_u32(bytes, descriptor_start + 4)?)
        .map_err(|_| DocxParseError::InvalidPackage)?;
    let expanded = usize::try_from(read_u32(bytes, descriptor_start + 8)?)
        .map_err(|_| DocxParseError::InvalidPackage)?;
    if descriptor_end > bytes.len()
        || read_u32(bytes, descriptor_start)? != entry.crc
        || compressed != entry.compressed
        || expanded != entry.expanded
    {
        return Err(DocxParseError::InvalidPackage);
    }
    Ok(())
}

fn extract_document_xml(xml: &[u8]) -> Result<Vec<u8>, DocxParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut state = DocumentState::new()?;
    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|_| DocxParseError::InvalidXml)?;
        state.count_event()?;
        match event {
            Event::Start(start) => state.start(&start)?,
            Event::Empty(empty) => state.empty(&empty)?,
            Event::Text(text) if state.accepts_text() => {
                let decoded = text.decode().map_err(|_| DocxParseError::InvalidXml)?;
                let unescaped = quick_xml::escape::unescape(&decoded)
                    .map_err(|_| DocxParseError::InvalidXml)?;
                state.append(&unescaped)?;
            }
            Event::CData(_) | Event::DocType(_) | Event::PI(_) => {
                return Err(DocxParseError::InvalidXml);
            }
            Event::End(end) => state.end(end.name().as_ref())?,
            Event::Eof => break,
            Event::GeneralRef(reference) if state.accepts_text() => state.reference(&reference)?,
            Event::GeneralRef(_) => return Err(DocxParseError::InvalidXml),
            Event::Decl(_) | Event::Comment(_) | Event::Text(_) => {}
        }
        buffer.clear();
    }
    state.finish()
}

struct DocumentState {
    builder: WorkerExtractionBuilder,
    depth: usize,
    events: usize,
    saw_document: bool,
    paragraph: Option<Paragraph>,
    in_text: usize,
    ignored_depth: usize,
}

impl DocumentState {
    fn new() -> Result<Self, DocxParseError> {
        Ok(Self {
            builder: WorkerExtractionBuilder::new(InputFormat::Docx, 1)
                .map_err(map_output_error)?,
            depth: 0,
            events: 0,
            saw_document: false,
            paragraph: None,
            in_text: 0,
            ignored_depth: 0,
        })
    }

    fn count_event(&mut self) -> Result<(), DocxParseError> {
        self.events = self
            .events
            .checked_add(1)
            .filter(|events| *events <= MAX_XML_EVENTS)
            .ok_or(DocxParseError::InvalidXml)?;
        Ok(())
    }

    fn start(&mut self, start: &BytesStart<'_>) -> Result<(), DocxParseError> {
        self.depth = self
            .depth
            .checked_add(1)
            .filter(|depth| *depth <= MAX_XML_DEPTH)
            .ok_or(DocxParseError::InvalidXml)?;
        let name = start.name();
        if self.depth == 1 {
            if self.saw_document || name.as_ref() != b"w:document" {
                return Err(DocxParseError::InvalidXml);
            }
            self.saw_document = true;
        }
        if active_element(name.as_ref()) {
            return Err(DocxParseError::ActiveContent);
        }
        if self.ignored_depth > 0 || name.as_ref() == b"w:del" {
            self.ignored_depth += 1;
        } else if name.as_ref() == b"w:p" {
            if self.paragraph.is_some() {
                return Err(DocxParseError::InvalidXml);
            }
            self.paragraph = Some(Paragraph::default());
        } else if name.as_ref() == b"w:t" && self.paragraph.is_some() {
            self.in_text += 1;
        } else if name.as_ref() == b"w:pStyle" {
            apply_style(self.paragraph.as_mut(), start)?;
        } else if name.as_ref() == b"w:numPr"
            && let Some(paragraph) = self.paragraph.as_mut()
        {
            paragraph.kind = BlockKind::ListItem;
        }
        Ok(())
    }

    fn empty(&mut self, empty: &BytesStart<'_>) -> Result<(), DocxParseError> {
        let name = empty.name();
        if active_element(name.as_ref()) {
            return Err(DocxParseError::ActiveContent);
        }
        if self.ignored_depth > 0 {
            return Ok(());
        }
        if name.as_ref() == b"w:pStyle" {
            apply_style(self.paragraph.as_mut(), empty)?;
        } else if name.as_ref() == b"w:numPr" {
            if let Some(paragraph) = self.paragraph.as_mut() {
                paragraph.kind = BlockKind::ListItem;
            }
        } else if matches!(name.as_ref(), b"w:br" | b"w:cr") {
            self.append("\n")?;
        } else if name.as_ref() == b"w:tab" {
            self.append("\t")?;
        }
        Ok(())
    }

    fn accepts_text(&self) -> bool {
        self.ignored_depth == 0 && self.in_text > 0
    }

    fn append(&mut self, value: &str) -> Result<(), DocxParseError> {
        append_text(self.paragraph.as_mut(), value)
    }

    fn reference(
        &mut self,
        reference: &quick_xml::events::BytesRef<'_>,
    ) -> Result<(), DocxParseError> {
        let character = if let Some(character) = reference
            .resolve_char_ref()
            .map_err(|_| DocxParseError::InvalidXml)?
        {
            character
        } else {
            match reference.as_ref() {
                b"amp" => '&',
                b"lt" => '<',
                b"gt" => '>',
                b"quot" => '"',
                b"apos" => '\'',
                _ => return Err(DocxParseError::InvalidXml),
            }
        };
        self.append(character.encode_utf8(&mut [0; 4]))
    }

    fn end(&mut self, name: &[u8]) -> Result<(), DocxParseError> {
        if self.ignored_depth > 0 {
            self.ignored_depth -= 1;
        } else if name == b"w:t" && self.in_text > 0 {
            self.in_text -= 1;
        } else if name == b"w:p" {
            let paragraph = self.paragraph.take().ok_or(DocxParseError::InvalidXml)?;
            if !paragraph.text.is_empty() {
                self.builder
                    .push(1, paragraph.kind, paragraph.text)
                    .map_err(map_output_error)?;
            }
        }
        self.depth = self
            .depth
            .checked_sub(1)
            .ok_or(DocxParseError::InvalidXml)?;
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>, DocxParseError> {
        if !self.saw_document
            || self.depth != 0
            || self.paragraph.is_some()
            || self.in_text != 0
            || self.ignored_depth != 0
        {
            return Err(DocxParseError::InvalidXml);
        }
        self.builder.finish().map_err(map_output_error)
    }
}

struct Paragraph {
    kind: BlockKind,
    text: String,
    characters: usize,
}

impl Default for Paragraph {
    fn default() -> Self {
        Self {
            kind: BlockKind::Paragraph,
            text: String::new(),
            characters: 0,
        }
    }
}

fn append_text(paragraph: Option<&mut Paragraph>, value: &str) -> Result<(), DocxParseError> {
    let Some(paragraph) = paragraph else {
        return Ok(());
    };
    let added = value.chars().count();
    paragraph.characters = paragraph
        .characters
        .checked_add(added)
        .filter(|count| *count <= MAX_BLOCK_CHARACTERS)
        .ok_or(DocxParseError::OutputLimit)?;
    paragraph
        .text
        .try_reserve(value.len())
        .map_err(|_| DocxParseError::OutputLimit)?;
    paragraph.text.push_str(value);
    Ok(())
}

fn apply_style(
    paragraph: Option<&mut Paragraph>,
    element: &BytesStart<'_>,
) -> Result<(), DocxParseError> {
    let Some(paragraph) = paragraph else {
        return Ok(());
    };
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|_| DocxParseError::InvalidXml)?;
        if attribute.key.as_ref() == b"w:val" {
            let value = attribute
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, element.decoder())
                .map_err(|_| DocxParseError::InvalidXml)?;
            // Wire v1 has no heading-level field. Only the top-level built-in
            // style becomes a section hint; lower levels remain literal text
            // rather than incorrectly creating peer sections.
            if value == "Heading1" {
                paragraph.kind = BlockKind::Heading;
            } else if value == "ListParagraph" {
                paragraph.kind = BlockKind::ListItem;
            }
        }
    }
    Ok(())
}

fn active_element(name: &[u8]) -> bool {
    matches!(
        name,
        b"w:altChunk"
            | b"w:object"
            | b"w:oleObject"
            | b"w:control"
            | b"w:subDoc"
            | b"w:fldSimple"
            | b"w:instrText"
    )
}

#[derive(Clone, Copy)]
enum RelationshipScope {
    Package,
    Document,
}

fn validate_relationships(xml: &[u8], scope: RelationshipScope) -> Result<(), DocxParseError> {
    let mut reader = Reader::from_reader(xml);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut count = 0_usize;
    let mut ids = HashSet::new();
    let mut saw_root = false;
    let mut saw_office_document = false;
    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|_| DocxParseError::InvalidXml)?;
        match event {
            Event::Start(start) => {
                depth += 1;
                if depth == 1 && start.name().as_ref() == b"Relationships" {
                    saw_root = true;
                } else if start.name().as_ref() == b"Relationship" {
                    validate_relationship(&start, &mut ids, scope, &mut saw_office_document)?;
                    count += 1;
                } else if depth > 1 {
                    return Err(DocxParseError::InvalidXml);
                }
                if depth > 2 || count > MAX_RELATIONSHIPS {
                    return Err(DocxParseError::InvalidXml);
                }
            }
            Event::Empty(empty) if empty.name().as_ref() == b"Relationship" && depth == 1 => {
                validate_relationship(&empty, &mut ids, scope, &mut saw_office_document)?;
                count += 1;
                if count > MAX_RELATIONSHIPS {
                    return Err(DocxParseError::InvalidXml);
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(DocxParseError::InvalidXml)?;
            }
            Event::Text(text) if text.decode().is_ok_and(|text| text.trim().is_empty()) => {}
            Event::Decl(_) | Event::Comment(_) => {}
            Event::Eof => break,
            _ => return Err(DocxParseError::InvalidXml),
        }
        buffer.clear();
    }
    if !saw_root
        || depth != 0
        || matches!(scope, RelationshipScope::Package) && !saw_office_document
    {
        return Err(DocxParseError::InvalidXml);
    }
    Ok(())
}

fn validate_relationship(
    element: &BytesStart<'_>,
    ids: &mut HashSet<String>,
    scope: RelationshipScope,
    saw_office_document: &mut bool,
) -> Result<(), DocxParseError> {
    let mut id = None;
    let mut relation_type = None;
    let mut target = None;
    let mut target_mode = None;
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|_| DocxParseError::InvalidXml)?;
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, element.decoder())
            .map_err(|_| DocxParseError::InvalidXml)?
            .into_owned();
        match attribute.key.as_ref() {
            b"Id" => id = Some(value),
            b"Type" => relation_type = Some(value),
            b"Target" => target = Some(value),
            b"TargetMode" => target_mode = Some(value),
            _ => return Err(DocxParseError::InvalidXml),
        }
    }
    let id = id
        .filter(|id| !id.is_empty())
        .ok_or(DocxParseError::InvalidXml)?;
    if !ids.insert(id) {
        return Err(DocxParseError::InvalidXml);
    }
    let relation_type = relation_type.ok_or(DocxParseError::InvalidXml)?;
    let target = target.ok_or(DocxParseError::InvalidXml)?;
    if matches!(scope, RelationshipScope::Package) && relation_type.ends_with("/officeDocument") {
        if *saw_office_document || target_mode.is_some() || target != "word/document.xml" {
            return Err(DocxParseError::ActiveContent);
        }
        *saw_office_document = true;
    } else if target_mode.as_deref() == Some("External") {
        if matches!(scope, RelationshipScope::Package) {
            return Err(DocxParseError::ActiveContent);
        }
        if !relation_type.ends_with("/hyperlink") || !allowed_external_target(&target) {
            return Err(DocxParseError::ActiveContent);
        }
    } else if target_mode.is_some()
        || !safe_internal_target(&target)
        || active_relationship_type(&relation_type)
    {
        return Err(DocxParseError::ActiveContent);
    }
    Ok(())
}

fn active_relationship_type(relation_type: &str) -> bool {
    matches!(
        relation_type.rsplit('/').next(),
        Some(
            "altChunk"
                | "attachedTemplate"
                | "control"
                | "externalLink"
                | "oleObject"
                | "package"
                | "subDocument"
                | "vbaProject"
        )
    )
}

fn validate_content_types(xml: &[u8]) -> Result<(), DocxParseError> {
    let mut reader = Reader::from_reader(xml);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut entries = 0_usize;
    let mut saw_root = false;
    let mut saw_document = false;
    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|_| DocxParseError::InvalidXml)?;
        match event {
            Event::Start(start) => {
                depth += 1;
                if depth == 1 && start.name().as_ref() == b"Types" {
                    saw_root = true;
                } else if depth == 2 && matches!(start.name().as_ref(), b"Default" | b"Override") {
                    inspect_content_type(&start, &mut saw_document)?;
                    entries += 1;
                } else if depth > 1 {
                    return Err(DocxParseError::InvalidXml);
                }
                if depth > 2 || entries > MAX_DOCX_ENTRIES {
                    return Err(DocxParseError::InvalidXml);
                }
            }
            Event::Empty(empty)
                if depth == 1 && matches!(empty.name().as_ref(), b"Default" | b"Override") =>
            {
                inspect_content_type(&empty, &mut saw_document)?;
                entries += 1;
                if entries > MAX_DOCX_ENTRIES {
                    return Err(DocxParseError::InvalidXml);
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(DocxParseError::InvalidXml)?;
            }
            Event::Text(text) if text.decode().is_ok_and(|text| text.trim().is_empty()) => {}
            Event::Decl(_) | Event::Comment(_) => {}
            Event::Eof => break,
            _ => return Err(DocxParseError::InvalidXml),
        }
        buffer.clear();
    }
    if !saw_root || !saw_document || depth != 0 {
        return Err(DocxParseError::InvalidXml);
    }
    Ok(())
}

fn inspect_content_type(
    element: &BytesStart<'_>,
    saw_document: &mut bool,
) -> Result<(), DocxParseError> {
    let mut part_name = None;
    let mut content_type = None;
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|_| DocxParseError::InvalidXml)?;
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, element.decoder())
            .map_err(|_| DocxParseError::InvalidXml)?;
        match attribute.key.as_ref() {
            b"PartName" => part_name = Some(value.into_owned()),
            b"ContentType" => content_type = Some(value.into_owned()),
            b"Extension" => {}
            _ => return Err(DocxParseError::InvalidXml),
        }
    }
    let content_type = content_type.ok_or(DocxParseError::InvalidXml)?;
    let lower = content_type.to_ascii_lowercase();
    if lower.contains("macroenabled") || lower.contains("vba") || lower.contains("oleobject") {
        return Err(DocxParseError::ActiveContent);
    }
    if part_name.as_deref() == Some("/word/document.xml") {
        const MAIN: &str =
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
        if *saw_document || content_type != MAIN {
            return Err(DocxParseError::InvalidPackage);
        }
        *saw_document = true;
    }
    Ok(())
}

fn allowed_external_target(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    !target.chars().any(char::is_whitespace)
        && (lower.starts_with("https://")
            || lower.starts_with("http://")
            || lower.starts_with("mailto:"))
}

fn safe_internal_target(target: &str) -> bool {
    !target.is_empty()
        && !target.starts_with('/')
        && !target.contains(['\\', ':', '\0'])
        && !target
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
}

fn map_output_error(error: WorkerOutputError) -> DocxParseError {
    match error {
        WorkerOutputError::NoReadableText => DocxParseError::NoReadableText,
        WorkerOutputError::LimitExceeded
        | WorkerOutputError::InvalidBlock
        | WorkerOutputError::Encoding
        | WorkerOutputError::Allocation => DocxParseError::OutputLimit,
    }
}

fn map_source_error(error: SourceError) -> DocxParseError {
    match error {
        SourceError::LimitExceeded | SourceError::ExpansionLimit => DocxParseError::InputLimit,
        SourceError::ActiveContent => DocxParseError::ActiveContent,
        SourceError::FormatMismatch | SourceError::InvalidContainer | SourceError::UnsafePath => {
            DocxParseError::InvalidPackage
        }
    }
}

fn find_eocd(bytes: &[u8]) -> Result<usize, DocxParseError> {
    if bytes.len() < 22 {
        return Err(DocxParseError::InvalidPackage);
    }
    let start = bytes.len().saturating_sub(22 + 1_024);
    for index in (start..=bytes.len() - 22).rev() {
        if read_u32(bytes, index)? == ZIP_END
            && usize::from(read_u16(bytes, index + 20)?) + index + 22 == bytes.len()
        {
            return Ok(index);
        }
    }
    Err(DocxParseError::InvalidPackage)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DocxParseError> {
    Ok(u16::from_le_bytes(
        bytes
            .get(
                offset
                    ..offset
                        .checked_add(2)
                        .ok_or(DocxParseError::InvalidPackage)?,
            )
            .ok_or(DocxParseError::InvalidPackage)?
            .try_into()
            .map_err(|_| DocxParseError::InvalidPackage)?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DocxParseError> {
    Ok(u32::from_le_bytes(
        bytes
            .get(
                offset
                    ..offset
                        .checked_add(4)
                        .ok_or(DocxParseError::InvalidPackage)?,
            )
            .ok_or(DocxParseError::InvalidPackage)?
            .try_into()
            .map_err(|_| DocxParseError::InvalidPackage)?,
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{Compression, write::DeflateEncoder};
    use ort_documents::import::{BlockKind, InputFormat, ValidatedExtraction};

    use super::*;

    const CONTENT_TYPES: &[u8] = br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
    const ROOT_RELS: &[u8] = br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="document" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    const EMPTY_RELS: &[u8] = br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#;

    fn docx(document: &[u8], relationships: Option<&[u8]>, deflated: bool) -> Vec<u8> {
        let mut parts = vec![
            ("[Content_Types].xml", CONTENT_TYPES, deflated),
            ("_rels/.rels", ROOT_RELS, deflated),
            ("word/document.xml", document, deflated),
        ];
        if let Some(relationships) = relationships {
            parts.push(("word/_rels/document.xml.rels", relationships, deflated));
        }
        package(&parts)
    }

    fn package(parts: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let mut output = Vec::new();
        let mut records = Vec::new();
        for (name, body, deflated) in parts {
            let encoded = if *deflated {
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
                encoder.write_all(body).unwrap();
                encoder.finish().unwrap()
            } else {
                body.to_vec()
            };
            let offset = u32::try_from(output.len()).unwrap();
            let crc = crc32fast::hash(body);
            let compressed = u32::try_from(encoded.len()).unwrap();
            let expanded = u32::try_from(body.len()).unwrap();
            push_u32(&mut output, 0x0403_4b50);
            for value in [
                20,
                if *deflated { 0x0808 } else { 0x0800 },
                if *deflated { 8 } else { 0 },
                0,
                0,
            ] {
                push_u16(&mut output, value);
            }
            for value in if *deflated {
                [0, 0, 0]
            } else {
                [crc, compressed, expanded]
            } {
                push_u32(&mut output, value);
            }
            push_u16(&mut output, u16::try_from(name.len()).unwrap());
            push_u16(&mut output, 0);
            output.extend_from_slice(name.as_bytes());
            output.extend_from_slice(&encoded);
            if *deflated {
                for value in [ZIP_DATA_DESCRIPTOR, crc, compressed, expanded] {
                    push_u32(&mut output, value);
                }
            }
            records.push((name, *deflated, offset, crc, compressed, expanded));
        }
        let central_offset = u32::try_from(output.len()).unwrap();
        for (name, deflated, offset, crc, compressed, expanded) in &records {
            push_u32(&mut output, ZIP_CENTRAL_HEADER);
            for value in [
                20,
                20,
                if *deflated { 0x0808 } else { 0x0800 },
                if *deflated { 8 } else { 0 },
                0,
                0,
            ] {
                push_u16(&mut output, value);
            }
            for value in [*crc, *compressed, *expanded] {
                push_u32(&mut output, value);
            }
            for value in [u16::try_from(name.len()).unwrap(), 0, 0, 0, 0] {
                push_u16(&mut output, value);
            }
            push_u32(&mut output, 0);
            push_u32(&mut output, *offset);
            output.extend_from_slice(name.as_bytes());
        }
        let central_size = u32::try_from(output.len()).unwrap() - central_offset;
        push_u32(&mut output, ZIP_END);
        let part_count = u16::try_from(parts.len()).unwrap();
        for value in [0, 0, part_count, part_count] {
            push_u16(&mut output, value);
        }
        push_u32(&mut output, central_size);
        push_u32(&mut output, central_offset);
        push_u16(&mut output, 0);
        output
    }

    fn push_u16(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn extract(bytes: &[u8]) -> Result<ValidatedExtraction, DocxParseError> {
        let wire = extract_docx(&mut Cursor::new(bytes))?;
        ValidatedExtraction::decode(&wire, InputFormat::Docx)
            .map_err(|_| DocxParseError::OutputLimit)
    }

    #[test]
    fn extracts_bounded_semantic_blocks_from_stored_and_deflated_packages() {
        let document = br#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Exp&#233;rience &amp; Work</w:t></w:r></w:p><w:p><w:r><w:t xml:space="preserve">First line</w:t><w:br/><w:t>second</w:t><w:tab/><w:t>column</w:t></w:r><w:del><w:r><w:t>DELETED_PRIVATE_TEXT</w:t></w:r></w:del></w:p><w:p><w:pPr><w:numPr><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Built safely</w:t></w:r></w:p></w:body></w:document>"#;
        for deflated in [false, true] {
            let extraction = extract(&docx(document, Some(EMPTY_RELS), deflated)).unwrap();
            assert_eq!(extraction.page_count(), 1);
            assert_eq!(extraction.blocks().len(), 3);
            assert_eq!(extraction.blocks()[0].kind, BlockKind::Heading);
            assert_eq!(extraction.blocks()[0].text, "Expérience & Work");
            assert_eq!(extraction.blocks()[1].text, "First line\nsecond\tcolumn");
            assert_eq!(extraction.blocks()[2].kind, BlockKind::ListItem);
            assert_eq!(extraction.blocks()[2].text, "Built safely");
            assert!(
                extraction
                    .blocks()
                    .iter()
                    .all(|block| !block.text.contains("DELETED_PRIVATE_TEXT"))
            );
        }
    }

    #[test]
    fn permits_visible_http_hyperlinks_but_rejects_external_file_targets() {
        let document = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:hyperlink r:id="link1" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:r><w:t>Portfolio</w:t></w:r></w:hyperlink></w:p></w:body></w:document>"#;
        let safe = br#"<Relationships><Relationship Id="link1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" TargetMode="External" Target="https://example.org/profile"/></Relationships>"#;
        assert!(extract(&docx(document, Some(safe), false)).is_ok());

        let unsafe_relationships = br#"<Relationships><Relationship Id="link1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" TargetMode="External" Target="file:///private/secret"/></Relationships>"#;
        assert_eq!(
            extract(&docx(document, Some(unsafe_relationships), false)).unwrap_err(),
            DocxParseError::ActiveContent
        );

        let embedded = br#"<Relationships><Relationship Id="payload" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="payload.bin"/></Relationships>"#;
        assert_eq!(
            extract(&docx(document, Some(embedded), false)).unwrap_err(),
            DocxParseError::ActiveContent
        );
    }

    #[test]
    fn requires_fixed_docx_content_type_and_root_document_relationship() {
        let document = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Text</w:t></w:r></w:p></w:body></w:document>"#;
        let macro_types = CONTENT_TYPES
            .windows("application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml".len())
            .position(|window| {
                window
                    == b"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
            })
            .unwrap();
        let mut active = CONTENT_TYPES.to_vec();
        active.splice(
            macro_types..macro_types
                + "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
                    .len(),
            b"application/vnd.ms-word.document.macroEnabled.main+xml"
                .iter()
                .copied(),
        );
        assert_eq!(
            extract(&package(&[
                ("[Content_Types].xml", &active, false),
                ("_rels/.rels", ROOT_RELS, false),
                ("word/document.xml", document, false),
            ]))
            .unwrap_err(),
            DocxParseError::ActiveContent
        );

        let redirected = br#"<Relationships><Relationship Id="document" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="other/document.xml"/></Relationships>"#;
        assert_eq!(
            extract(&package(&[
                ("[Content_Types].xml", CONTENT_TYPES, false),
                ("_rels/.rels", redirected, false),
                ("word/document.xml", document, false),
            ]))
            .unwrap_err(),
            DocxParseError::ActiveContent
        );
    }

    #[test]
    fn rejects_active_xml_dtd_and_excessive_nesting() {
        for document in [
            br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:altChunk r:id="remote"/></w:body></w:document>"#.as_slice(),
            br#"<!DOCTYPE document [<!ENTITY x SYSTEM "file:///secret">]><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>&x;</w:t></w:r></w:p></w:body></w:document>"#.as_slice(),
        ] {
            assert!(extract(&docx(document, None, false)).is_err());
        }

        let mut deep = String::from(
            "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body><w:p>",
        );
        deep.push_str(&"<w:r>".repeat(MAX_XML_DEPTH));
        deep.push_str("<w:t>text</w:t>");
        deep.push_str(&"</w:r>".repeat(MAX_XML_DEPTH));
        deep.push_str("</w:p></w:body></w:document>");
        assert_eq!(
            extract(&docx(deep.as_bytes(), None, false)).unwrap_err(),
            DocxParseError::InvalidXml
        );
    }

    #[test]
    fn rejects_crc_corruption_oversized_input_and_empty_documents() {
        let document = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>SYNTHETIC_SENTINEL</w:t></w:r></w:p></w:body></w:document>"#;
        let mut corrupt = docx(document, None, false);
        let position = corrupt
            .windows("SYNTHETIC_SENTINEL".len())
            .position(|window| window == b"SYNTHETIC_SENTINEL")
            .unwrap();
        corrupt[position] ^= 1;
        assert_eq!(
            extract(&corrupt).unwrap_err(),
            DocxParseError::InvalidPackage
        );

        let oversized = vec![0_u8; MAX_IMPORT_SOURCE_BYTES + 1];
        assert_eq!(
            extract_docx(&mut Cursor::new(oversized)).unwrap_err(),
            DocxParseError::InputLimit
        );

        let empty = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>  </w:t></w:r></w:p></w:body></w:document>"#;
        assert_eq!(
            extract(&docx(empty, None, false)).unwrap_err(),
            DocxParseError::NoReadableText
        );
    }

    #[test]
    fn errors_and_debug_output_never_include_document_text() {
        let debug = format!("{:?}", DocxParseError::InvalidXml);
        assert!(!debug.contains("PRIVATE_DOCUMENT_TEXT"));
        assert!(!DocxParseError::InvalidXml.to_string().contains("PRIVATE"));
    }
}
