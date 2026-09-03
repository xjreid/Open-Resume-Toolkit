//! Output-only ZIP32/store encoder for fixed OPC parts. No archive input,
//! compression, encryption, extra fields, paths from content, or ZIP64 support.
//! Layout: PKWARE APPNOTE 6.3.10, sections 4.3.7, 4.3.12 and 4.3.16.
use crate::docx::{DocxExportError, MAX_DOCX_BYTES};

pub(crate) fn package(parts: &[(&str, &str)]) -> Result<Vec<u8>, DocxExportError> {
    let count = u16::try_from(parts.len()).map_err(|_| DocxExportError::OutputTooLarge)?;
    let size = parts.iter().try_fold(22_usize, |size, (name, body)| {
        size.checked_add(76 + name.len() * 2 + body.len())
            .filter(|size| *size <= MAX_DOCX_BYTES)
            .ok_or(DocxExportError::OutputTooLarge)
    })?;
    let mut out = Vec::with_capacity(size);
    let mut offsets = Vec::with_capacity(parts.len());
    for (name, body) in parts {
        let offset = u32::try_from(out.len()).map_err(|_| DocxExportError::OutputTooLarge)?;
        let length = u32::try_from(body.len()).map_err(|_| DocxExportError::OutputTooLarge)?;
        let name_length = u16::try_from(name.len()).map_err(|_| DocxExportError::OutputTooLarge)?;
        let crc = crc32fast::hash(body.as_bytes());
        offsets.push((offset, length, name_length, crc));
        u32le(&mut out, 0x0403_4b50);
        // Version 2.0, UTF-8 names, store, midnight 1980-01-01 (deterministic).
        for value in [20, 0x0800, 0, 0, 33] {
            u16le(&mut out, value);
        }
        for value in [crc, length, length] {
            u32le(&mut out, value);
        }
        u16le(&mut out, name_length);
        u16le(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(body.as_bytes());
    }
    let central = u32::try_from(out.len()).map_err(|_| DocxExportError::OutputTooLarge)?;
    for ((name, _), (offset, length, name_length, crc)) in parts.iter().zip(offsets) {
        u32le(&mut out, 0x0201_4b50);
        for value in [20, 20, 0x0800, 0, 0, 33] {
            u16le(&mut out, value);
        }
        for value in [crc, length, length] {
            u32le(&mut out, value);
        }
        for value in [name_length, 0, 0, 0, 0] {
            u16le(&mut out, value);
        }
        u32le(&mut out, 0);
        u32le(&mut out, offset);
        out.extend_from_slice(name.as_bytes());
    }
    let central_size =
        u32::try_from(out.len()).map_err(|_| DocxExportError::OutputTooLarge)? - central;
    u32le(&mut out, 0x0605_4b50);
    for value in [0, 0, count, count] {
        u16le(&mut out, value);
    }
    u32le(&mut out, central_size);
    u32le(&mut out, central);
    u16le(&mut out, 0);
    debug_assert_eq!(out.len(), size);
    Ok(out)
}

fn u16le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn u32le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_oversized_package_before_allocating_output() {
        assert_eq!(
            package(&[("fixed.xml", &"x".repeat(MAX_DOCX_BYTES))]),
            Err(DocxExportError::OutputTooLarge)
        );
    }
}
