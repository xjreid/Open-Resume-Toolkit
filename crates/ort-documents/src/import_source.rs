//! Bounded parent-side source-envelope validation before private staging.
//!
//! This is deliberately not a PDF or DOCX content parser. It checks enough of
//! the outer format to reject extension-only claims, malformed ZIP metadata,
//! unsafe package names, encryption, active DOCX payloads, and obvious expansion
//! bombs without decompressing any attacker-controlled entry. Page/content
//! parsing remains inside the disabled, disposable native worker.

use std::collections::HashSet;

use crate::import::InputFormat;

pub use ort_domain::MAX_IMPORT_SOURCE_BYTES;
pub const MAX_DOCX_ENTRIES: usize = 4_096;
pub const MAX_DOCX_ENTRY_NAME_BYTES: usize = 512;
pub const MAX_DOCX_EXPANSION_RATIO: u64 = 100;
const MAX_ZIP_COMMENT_BYTES: usize = 1_024;
const PDF_EOF_SEARCH_BYTES: usize = 1_024;

const ZIP_LOCAL_HEADER: u32 = 0x0403_4b50;
const ZIP_CENTRAL_HEADER: u32 = 0x0201_4b50;
const ZIP_END: u32 = 0x0605_4b50;
const ZIP64_SENTINEL_U16: u16 = u16::MAX;
const ZIP64_SENTINEL_U32: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceInspection {
    pub format: InputFormat,
    pub byte_count: usize,
    pub package_entries: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    #[error("document source exceeds its configured byte limit")]
    LimitExceeded,
    #[error("document source signature does not match the selected format")]
    FormatMismatch,
    #[error("document source container is malformed or unsupported")]
    InvalidContainer,
    #[error("document source contains an unsafe package path")]
    UnsafePath,
    #[error("encrypted or active document content is unsupported")]
    ActiveContent,
    #[error("document source exceeds its compressed expansion limit")]
    ExpansionLimit,
}

/// Validates a complete, already bounded parent-owned snapshot of a selected
/// source. The bytes must not be read again from the user path after this check.
///
/// # Errors
/// Rejects empty/oversized input, format mismatch, malformed/ZIP64/multidisk
/// packages, unsafe or duplicate names, encrypted/active content, unsupported
/// compression, and metadata-declared expansion over the fixed ratio.
pub fn inspect_source(
    bytes: &[u8],
    expected: InputFormat,
) -> Result<SourceInspection, SourceError> {
    if bytes.is_empty() || bytes.len() > MAX_IMPORT_SOURCE_BYTES {
        return Err(SourceError::LimitExceeded);
    }
    match expected {
        InputFormat::Pdf => inspect_pdf(bytes),
        InputFormat::Docx => inspect_docx(bytes),
    }
}

fn inspect_pdf(bytes: &[u8]) -> Result<SourceInspection, SourceError> {
    let version = bytes.get(..8).ok_or(SourceError::FormatMismatch)?;
    let supported = version == b"%PDF-2.0"
        || version.get(..7) == Some(b"%PDF-1.") && matches!(version.get(7), Some(b'0'..=b'7'));
    if !supported {
        return Err(SourceError::FormatMismatch);
    }
    let tail_start = bytes.len().saturating_sub(PDF_EOF_SEARCH_BYTES);
    let tail = &bytes[tail_start..];
    let eof = tail
        .windows(5)
        .rposition(|window| window == b"%%EOF")
        .ok_or(SourceError::InvalidContainer)?;
    if tail[eof + 5..]
        .iter()
        .any(|byte| !byte.is_ascii_whitespace())
    {
        return Err(SourceError::InvalidContainer);
    }
    Ok(SourceInspection {
        format: InputFormat::Pdf,
        byte_count: bytes.len(),
        package_entries: None,
    })
}

fn inspect_docx(bytes: &[u8]) -> Result<SourceInspection, SourceError> {
    if read_u32(bytes, 0) != Some(ZIP_LOCAL_HEADER) {
        return Err(SourceError::FormatMismatch);
    }
    let (central_start, central_end, total_entries) = docx_directory(bytes)?;
    inspect_docx_entries(bytes, central_start, central_end, total_entries)?;
    Ok(SourceInspection {
        format: InputFormat::Docx,
        byte_count: bytes.len(),
        package_entries: Some(total_entries),
    })
}

fn docx_directory(bytes: &[u8]) -> Result<(usize, usize, u16), SourceError> {
    let eocd = find_end_record(bytes)?;
    let disk = read_u16(bytes, eocd + 4).ok_or(SourceError::InvalidContainer)?;
    let central_disk = read_u16(bytes, eocd + 6).ok_or(SourceError::InvalidContainer)?;
    let disk_entries = read_u16(bytes, eocd + 8).ok_or(SourceError::InvalidContainer)?;
    let total_entries = read_u16(bytes, eocd + 10).ok_or(SourceError::InvalidContainer)?;
    let central_size = read_u32(bytes, eocd + 12).ok_or(SourceError::InvalidContainer)?;
    let central_offset = read_u32(bytes, eocd + 16).ok_or(SourceError::InvalidContainer)?;
    if disk != 0
        || central_disk != 0
        || disk_entries != total_entries
        || total_entries == 0
        || total_entries == ZIP64_SENTINEL_U16
        || usize::from(total_entries) > MAX_DOCX_ENTRIES
        || central_size == ZIP64_SENTINEL_U32
        || central_offset == ZIP64_SENTINEL_U32
    {
        return Err(SourceError::InvalidContainer);
    }
    let central_start =
        usize::try_from(central_offset).map_err(|_| SourceError::InvalidContainer)?;
    let central_len = usize::try_from(central_size).map_err(|_| SourceError::InvalidContainer)?;
    let central_end = central_start
        .checked_add(central_len)
        .ok_or(SourceError::InvalidContainer)?;
    if central_end != eocd || central_end > bytes.len() {
        return Err(SourceError::InvalidContainer);
    }
    Ok((central_start, central_end, total_entries))
}

fn inspect_docx_entries(
    bytes: &[u8],
    central_start: usize,
    central_end: usize,
    total_entries: u16,
) -> Result<(), SourceError> {
    let mut cursor = central_start;
    let mut names = HashSet::with_capacity(usize::from(total_entries));
    let mut compressed_total = 0_u64;
    let mut expanded_total = 0_u64;
    for _ in 0..total_entries {
        if read_u32(bytes, cursor) != Some(ZIP_CENTRAL_HEADER) {
            return Err(SourceError::InvalidContainer);
        }
        let flags = read_u16(bytes, cursor + 8).ok_or(SourceError::InvalidContainer)?;
        let method = read_u16(bytes, cursor + 10).ok_or(SourceError::InvalidContainer)?;
        let compressed = read_u32(bytes, cursor + 20).ok_or(SourceError::InvalidContainer)?;
        let expanded = read_u32(bytes, cursor + 24).ok_or(SourceError::InvalidContainer)?;
        let name_len =
            usize::from(read_u16(bytes, cursor + 28).ok_or(SourceError::InvalidContainer)?);
        let extra_len =
            usize::from(read_u16(bytes, cursor + 30).ok_or(SourceError::InvalidContainer)?);
        let comment_len =
            usize::from(read_u16(bytes, cursor + 32).ok_or(SourceError::InvalidContainer)?);
        let start_disk = read_u16(bytes, cursor + 34).ok_or(SourceError::InvalidContainer)?;
        let local_offset = read_u32(bytes, cursor + 42).ok_or(SourceError::InvalidContainer)?;
        if flags & 0x2041 != 0 {
            return Err(SourceError::ActiveContent);
        }
        if flags & !0x0808 != 0 || method != 0 && method != 8 {
            return Err(SourceError::InvalidContainer);
        }
        if compressed == ZIP64_SENTINEL_U32
            || expanded == ZIP64_SENTINEL_U32
            || local_offset == ZIP64_SENTINEL_U32
            || start_disk != 0
            || name_len == 0
            || name_len > MAX_DOCX_ENTRY_NAME_BYTES
        {
            return Err(SourceError::InvalidContainer);
        }
        let name_start = cursor
            .checked_add(46)
            .ok_or(SourceError::InvalidContainer)?;
        let name_end = name_start
            .checked_add(name_len)
            .ok_or(SourceError::InvalidContainer)?;
        let record_end = name_end
            .checked_add(extra_len)
            .and_then(|value| value.checked_add(comment_len))
            .ok_or(SourceError::InvalidContainer)?;
        if record_end > central_end {
            return Err(SourceError::InvalidContainer);
        }
        let name = package_name(&bytes[name_start..name_end], flags)?;
        if !names.insert(name) {
            return Err(SourceError::InvalidContainer);
        }
        if active_docx_part(name) {
            return Err(SourceError::ActiveContent);
        }
        validate_local_header(
            bytes,
            central_start,
            local_offset,
            flags,
            method,
            compressed,
            name.as_bytes(),
        )?;
        compressed_total = compressed_total
            .checked_add(u64::from(compressed))
            .ok_or(SourceError::ExpansionLimit)?;
        expanded_total = expanded_total
            .checked_add(u64::from(expanded))
            .ok_or(SourceError::ExpansionLimit)?;
        cursor = record_end;
    }
    if cursor != central_end
        || !names.contains("[Content_Types].xml")
        || !names.contains("_rels/.rels")
        || !names.contains("word/document.xml")
    {
        return Err(SourceError::InvalidContainer);
    }
    if expanded_total
        > compressed_total
            .max(1)
            .saturating_mul(MAX_DOCX_EXPANSION_RATIO)
    {
        return Err(SourceError::ExpansionLimit);
    }
    Ok(())
}

fn find_end_record(bytes: &[u8]) -> Result<usize, SourceError> {
    let search = bytes.len().min(22 + MAX_ZIP_COMMENT_BYTES);
    let start = bytes.len() - search;
    for index in (start..=bytes.len().saturating_sub(22)).rev() {
        if read_u32(bytes, index) != Some(ZIP_END) {
            continue;
        }
        let comment_len =
            usize::from(read_u16(bytes, index + 20).ok_or(SourceError::InvalidContainer)?);
        if comment_len <= MAX_ZIP_COMMENT_BYTES && index + 22 + comment_len == bytes.len() {
            return Ok(index);
        }
    }
    Err(SourceError::InvalidContainer)
}

fn package_name(bytes: &[u8], flags: u16) -> Result<&str, SourceError> {
    if bytes.iter().any(|byte| *byte == 0 || *byte == b'\\') {
        return Err(SourceError::UnsafePath);
    }
    let name = std::str::from_utf8(bytes).map_err(|_| SourceError::UnsafePath)?;
    if (!name.is_ascii() && flags & 0x0800 == 0)
        || name.starts_with('/')
        || name.ends_with('/')
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || name.contains(':')
    {
        return Err(SourceError::UnsafePath);
    }
    Ok(name)
}

fn active_docx_part(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "word/vbaproject.bin"
        || lower.starts_with("word/activex/")
        || lower.starts_with("word/embeddings/")
        || lower.starts_with("word/ink/")
}

fn validate_local_header(
    bytes: &[u8],
    central_start: usize,
    offset: u32,
    expected_flags: u16,
    expected_method: u16,
    compressed_size: u32,
    expected_name: &[u8],
) -> Result<(), SourceError> {
    let offset = usize::try_from(offset).map_err(|_| SourceError::InvalidContainer)?;
    if offset >= central_start || read_u32(bytes, offset) != Some(ZIP_LOCAL_HEADER) {
        return Err(SourceError::InvalidContainer);
    }
    let flags = read_u16(bytes, offset + 6).ok_or(SourceError::InvalidContainer)?;
    let method = read_u16(bytes, offset + 8).ok_or(SourceError::InvalidContainer)?;
    let name_len = usize::from(read_u16(bytes, offset + 26).ok_or(SourceError::InvalidContainer)?);
    let extra_len = usize::from(read_u16(bytes, offset + 28).ok_or(SourceError::InvalidContainer)?);
    let name_start = offset
        .checked_add(30)
        .ok_or(SourceError::InvalidContainer)?;
    let name_end = name_start
        .checked_add(name_len)
        .ok_or(SourceError::InvalidContainer)?;
    let data_start = name_end
        .checked_add(extra_len)
        .ok_or(SourceError::InvalidContainer)?;
    let data_end = data_start
        .checked_add(usize::try_from(compressed_size).map_err(|_| SourceError::InvalidContainer)?)
        .ok_or(SourceError::InvalidContainer)?;
    if flags != expected_flags
        || method != expected_method
        || expected_name
            != bytes
                .get(name_start..name_end)
                .ok_or(SourceError::InvalidContainer)?
        || data_end > central_start
    {
        return Err(SourceError::InvalidContainer);
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Entry<'a> {
        name: &'a str,
        flags: u16,
        method: u16,
        compressed: u32,
        expanded: u32,
    }

    fn package(entries: &[Entry<'_>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut offsets = Vec::new();
        for entry in entries {
            offsets.push(u32::try_from(bytes.len()).unwrap());
            bytes.extend_from_slice(&ZIP_LOCAL_HEADER.to_le_bytes());
            bytes.extend_from_slice(&20_u16.to_le_bytes());
            bytes.extend_from_slice(&entry.flags.to_le_bytes());
            bytes.extend_from_slice(&entry.method.to_le_bytes());
            bytes.extend_from_slice(&[0; 8]);
            bytes.extend_from_slice(&entry.compressed.to_le_bytes());
            bytes.extend_from_slice(&entry.expanded.to_le_bytes());
            bytes.extend_from_slice(&u16::try_from(entry.name.len()).unwrap().to_le_bytes());
            bytes.extend_from_slice(&0_u16.to_le_bytes());
            bytes.extend_from_slice(entry.name.as_bytes());
            bytes.resize(bytes.len() + usize::try_from(entry.compressed).unwrap(), 0);
        }
        let central_offset = u32::try_from(bytes.len()).unwrap();
        for (entry, offset) in entries.iter().zip(offsets) {
            bytes.extend_from_slice(&ZIP_CENTRAL_HEADER.to_le_bytes());
            bytes.extend_from_slice(&20_u16.to_le_bytes());
            bytes.extend_from_slice(&20_u16.to_le_bytes());
            bytes.extend_from_slice(&entry.flags.to_le_bytes());
            bytes.extend_from_slice(&entry.method.to_le_bytes());
            bytes.extend_from_slice(&[0; 8]);
            bytes.extend_from_slice(&entry.compressed.to_le_bytes());
            bytes.extend_from_slice(&entry.expanded.to_le_bytes());
            bytes.extend_from_slice(&u16::try_from(entry.name.len()).unwrap().to_le_bytes());
            bytes.extend_from_slice(&0_u16.to_le_bytes());
            bytes.extend_from_slice(&0_u16.to_le_bytes());
            bytes.extend_from_slice(&0_u16.to_le_bytes());
            bytes.extend_from_slice(&0_u16.to_le_bytes());
            bytes.extend_from_slice(&0_u32.to_le_bytes());
            bytes.extend_from_slice(&offset.to_le_bytes());
            bytes.extend_from_slice(entry.name.as_bytes());
        }
        let central_size = u32::try_from(bytes.len()).unwrap() - central_offset;
        bytes.extend_from_slice(&ZIP_END.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        let count = u16::try_from(entries.len()).unwrap();
        bytes.extend_from_slice(&count.to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
        bytes.extend_from_slice(&central_size.to_le_bytes());
        bytes.extend_from_slice(&central_offset.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes
    }

    fn required() -> [Entry<'static>; 3] {
        [
            Entry {
                name: "[Content_Types].xml",
                flags: 0,
                method: 0,
                compressed: 10,
                expanded: 10,
            },
            Entry {
                name: "_rels/.rels",
                flags: 0,
                method: 8,
                compressed: 10,
                expanded: 50,
            },
            Entry {
                name: "word/document.xml",
                flags: 0,
                method: 8,
                compressed: 20,
                expanded: 100,
            },
        ]
    }

    #[test]
    fn accepts_bounded_pdf_and_minimal_docx_envelopes() {
        for version in ["1.0", "1.7", "2.0"] {
            let pdf = format!("%PDF-{version}\n1 0 obj\n<<>>\nendobj\n%%EOF\n");
            let result = inspect_source(pdf.as_bytes(), InputFormat::Pdf).unwrap();
            assert_eq!(result.format, InputFormat::Pdf);
            assert_eq!(result.package_entries, None);
        }
        let docx = package(&required());
        let result = inspect_source(&docx, InputFormat::Docx).unwrap();
        assert_eq!(result.byte_count, docx.len());
        assert_eq!(result.package_entries, Some(3));
    }

    #[test]
    fn format_selection_is_not_an_extension_only_claim() {
        let pdf = b"%PDF-1.7\n%%EOF\n";
        let docx = package(&required());
        assert_eq!(
            inspect_source(pdf, InputFormat::Docx),
            Err(SourceError::FormatMismatch)
        );
        assert_eq!(
            inspect_source(&docx, InputFormat::Pdf),
            Err(SourceError::FormatMismatch)
        );
        assert_eq!(
            inspect_source(b"PK fake", InputFormat::Docx),
            Err(SourceError::FormatMismatch)
        );
    }

    #[test]
    fn pdf_requires_supported_header_and_terminal_eof() {
        for bytes in [
            b"%PDF-1.8\n%%EOF\n".as_slice(),
            b"junk%PDF-1.7\n%%EOF\n",
            b"%PDF-1.7\n",
            b"%PDF-1.7\n%%EOF\ntrailing",
        ] {
            assert!(inspect_source(bytes, InputFormat::Pdf).is_err());
        }
    }

    #[test]
    fn docx_rejects_traversal_duplicates_active_parts_and_encryption() {
        let mut cases = Vec::new();
        for name in [
            "../escape",
            "/absolute",
            "word\\document.xml",
            "a//b",
            "a/./b",
        ] {
            let mut entries = required().to_vec();
            entries.push(Entry { name, ..entries[0] });
            cases.push((package(&entries), SourceError::UnsafePath));
        }
        for name in [
            "word/vbaProject.bin",
            "word/activeX/control.bin",
            "word/embeddings/object.bin",
        ] {
            let mut entries = required().to_vec();
            entries.push(Entry { name, ..entries[0] });
            cases.push((package(&entries), SourceError::ActiveContent));
        }
        let mut duplicate = required().to_vec();
        duplicate.push(duplicate[2]);
        cases.push((package(&duplicate), SourceError::InvalidContainer));
        let mut encrypted = required();
        encrypted[1].flags = 1;
        cases.push((package(&encrypted), SourceError::ActiveContent));
        for (bytes, expected) in cases {
            assert_eq!(inspect_source(&bytes, InputFormat::Docx), Err(expected));
        }
    }

    #[test]
    fn docx_rejects_bombs_missing_parts_and_corrupt_metadata() {
        let mut bomb = required();
        bomb[2].compressed = 1;
        bomb[2].expanded = 10_000;
        assert_eq!(
            inspect_source(&package(&bomb), InputFormat::Docx),
            Err(SourceError::ExpansionLimit)
        );
        assert_eq!(
            inspect_source(&package(&required()[..2]), InputFormat::Docx),
            Err(SourceError::InvalidContainer)
        );
        let valid = package(&required());
        for length in [1, 21, valid.len() - 1] {
            assert!(inspect_source(&valid[..length], InputFormat::Docx).is_err());
        }
        let mut multidisk = valid.clone();
        let eocd = multidisk.len() - 22;
        multidisk[eocd + 4] = 1;
        assert_eq!(
            inspect_source(&multidisk, InputFormat::Docx),
            Err(SourceError::InvalidContainer)
        );
        let mut zip64 = valid;
        zip64[eocd + 10..eocd + 12].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(
            inspect_source(&zip64, InputFormat::Docx),
            Err(SourceError::InvalidContainer)
        );

        let mut too_many = package(&required());
        let eocd = too_many.len() - 22;
        let count = u16::try_from(MAX_DOCX_ENTRIES + 1).unwrap().to_le_bytes();
        too_many[eocd + 8..eocd + 10].copy_from_slice(&count);
        too_many[eocd + 10..eocd + 12].copy_from_slice(&count);
        assert_eq!(
            inspect_source(&too_many, InputFormat::Docx),
            Err(SourceError::InvalidContainer)
        );

        let mut unsupported = required();
        unsupported[0].method = 12;
        assert_eq!(
            inspect_source(&package(&unsupported), InputFormat::Docx),
            Err(SourceError::InvalidContainer)
        );

        let mut unsupported = required();
        unsupported[0].flags = 2;
        assert_eq!(
            inspect_source(&package(&unsupported), InputFormat::Docx),
            Err(SourceError::InvalidContainer)
        );

        let mut local_mismatch = package(&required());
        local_mismatch[8..10].copy_from_slice(&8_u16.to_le_bytes());
        assert_eq!(
            inspect_source(&local_mismatch, InputFormat::Docx),
            Err(SourceError::InvalidContainer)
        );

        let mut overlap = package(&required());
        let eocd = overlap.len() - 22;
        let central = read_u32(&overlap, eocd + 16).unwrap();
        let central_index = usize::try_from(central).unwrap();
        overlap[central_index + 20..central_index + 24].copy_from_slice(&central.to_le_bytes());
        assert_eq!(
            inspect_source(&overlap, InputFormat::Docx),
            Err(SourceError::InvalidContainer)
        );
    }

    #[test]
    fn empty_and_oversized_sources_fail_before_format_work() {
        assert_eq!(
            inspect_source(&[], InputFormat::Pdf),
            Err(SourceError::LimitExceeded)
        );
        let oversized = vec![0; MAX_IMPORT_SOURCE_BYTES + 1];
        assert_eq!(
            inspect_source(&oversized, InputFormat::Docx),
            Err(SourceError::LimitExceeded)
        );
    }
}
