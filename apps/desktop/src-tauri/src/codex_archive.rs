//! No general-purpose archive extraction: accept exactly the pinned single-file
//! tar layout and copy only its bounded payload to a private non-executable file.
use flate2::read::GzDecoder;
use ort_codex_install::{Result, check_bytes, manifest, open_payload};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

fn octal(bytes: &[u8]) -> Result<u64> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| "PLAN_INSTALL_ARCHIVE_INVALID")?
        .trim_matches(['\0', ' ']);
    if value.is_empty() || !value.bytes().all(|b| matches!(b, b'0'..=b'7')) {
        return Err("PLAN_INSTALL_ARCHIVE_INVALID");
    }
    u64::from_str_radix(value, 8).map_err(|_| "PLAN_INSTALL_ARCHIVE_INVALID")
}
fn header(header: &[u8; 512], expected_name: &str, expected_length: u64) -> Result<()> {
    let name = &header[..100];
    let end = name.iter().position(|b| *b == 0).unwrap_or(100);
    let checksum: u64 = header
        .iter()
        .enumerate()
        .map(|(i, b)| {
            if (148..156).contains(&i) {
                32
            } else {
                u64::from(*b)
            }
        })
        .sum();
    if &name[..end] != expected_name.as_bytes()
        || name[end..].iter().any(|b| *b != 0)
        || !matches!(header[156], 0 | b'0')
        || header[157..257].iter().any(|b| *b != 0)
        || header[345..500].iter().any(|b| *b != 0)
        || &header[257..262] != b"ustar"
        || octal(&header[124..136])? != expected_length
        || octal(&header[148..156])? != checksum
    {
        return Err("PLAN_INSTALL_ARCHIVE_INVALID");
    }
    Ok(())
}
pub(crate) fn extract(archive: &Path, payload: &mut File) -> Result<()> {
    let spec = manifest();
    let mut compressed = open_payload(archive, spec.archive_bytes)?;
    // Verify the complete compressed archive before feeding an archive parser.
    check_bytes(
        &mut compressed,
        &mut std::io::sink(),
        spec.archive_bytes,
        &spec.archive_sha256,
        false,
    )?;
    compressed
        .seek(SeekFrom::Start(0))
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    let mut gzip = GzDecoder::new(compressed);
    let mut block = [0_u8; 512];
    gzip.read_exact(&mut block)
        .map_err(|_| "PLAN_INSTALL_ARCHIVE_INVALID")?;
    header(&block, &spec.archive_entry, spec.executable_bytes)?;
    check_bytes(
        &mut gzip.by_ref().take(spec.executable_bytes),
        payload,
        spec.executable_bytes,
        &spec.executable_sha256,
        true,
    )?;
    let padding = (512 - spec.executable_bytes % 512) % 512;
    let mut tail = Vec::new();
    gzip.take(padding + 10_241)
        .read_to_end(&mut tail)
        .map_err(|_| "PLAN_INSTALL_ARCHIVE_INVALID")?;
    if tail.len() < usize::try_from(padding + 1024).unwrap_or(usize::MAX)
        || tail.len() > usize::try_from(padding + 10_240).unwrap_or(0)
        || tail.len() % 512 != usize::try_from(padding).unwrap_or(1)
        || tail.iter().any(|b| *b != 0)
    {
        return Err("PLAN_INSTALL_ARCHIVE_INVALID");
    }
    payload.sync_all().map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid_header() -> [u8; 512] {
        let mut block = [0; 512];
        block[..4].copy_from_slice(b"file");
        block[124..136].copy_from_slice(b"00000000020\0");
        block[156] = b'0';
        block[257..263].copy_from_slice(b"ustar\0");
        seal(&mut block);
        block
    }
    fn seal(block: &mut [u8; 512]) {
        block[148..156].fill(b' ');
        let sum: u64 = block.iter().map(|b| u64::from(*b)).sum();
        block[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    }
    #[test]
    fn archive_headers_reject_links_devices_prefixes_extensions_duplicates_and_traversal() {
        let good = valid_header();
        assert!(header(&good, "file", 16).is_ok());
        for kind in *b"123456xgL" {
            let mut block = good;
            block[156] = kind;
            seal(&mut block);
            assert!(header(&block, "file", 16).is_err());
        }
        for name in ["../file", "/file", "dir/file", "other", "file/", "file\n"] {
            let mut block = good;
            block[..100].fill(0);
            block[..name.len()].copy_from_slice(name.as_bytes());
            seal(&mut block);
            assert!(header(&block, "file", 16).is_err());
        }
        for index in [157, 345, 124, 0, 148] {
            let mut block = good;
            block[index] = b'1';
            assert!(header(&block, "file", 16).is_err());
        }
        assert!(header(&good, "file", 17).is_err());
    }
    #[test]
    fn truncated_or_wrong_archives_never_create_executables() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("wrong.tar.gz");
        std::fs::write(&archive, b"not an archive").unwrap();
        let mut payload = tempfile::tempfile().unwrap();
        assert!(extract(&archive, &mut payload).is_err());
        assert_eq!(payload.metadata().unwrap().len(), 0);
    }
    #[test]
    #[ignore = "explicit official archive verification; no installation or account access"]
    fn official_archive_matches_both_pins_and_single_file_layout() {
        let archive = std::env::var_os("ORT_CODEX_ARCHIVE").unwrap();
        let mut payload = tempfile::NamedTempFile::new().unwrap();
        extract(Path::new(&archive), payload.as_file_mut()).unwrap();
        ort_codex_install::verify_payload(payload.path()).unwrap();
    }
}
