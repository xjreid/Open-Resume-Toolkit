use std::{
    io::Read,
    path::{Component, Path},
};

use cap_primitives::fs::FollowSymlinks;
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NativeInputError {
    #[error("choose a supported regular file")]
    InvalidSelection,
    #[error("the selected file is empty, changed, or exceeds its byte limit")]
    InvalidContent,
    #[error("the selected file could not be read")]
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDocumentFormat {
    Pdf,
    Docx,
}

/// Parent-owned snapshot of a native-dialog-selected document. Intentionally
/// does not implement `Debug` so document bytes cannot be logged accidentally.
pub struct NativeDocumentSource {
    pub format: NativeDocumentFormat,
    pub bytes: Vec<u8>,
}

/// Reads one native-dialog-selected backup through a held parent-directory
/// capability. The final component is opened without following symlinks, the
/// length is bounded before allocation, and growth/replacement never yields a
/// partial successful read.
///
/// # Errors
/// Rejects non-absolute paths, unexpected names/types, symlinks, empty or
/// oversized files, files that change during the read, and unavailable I/O.
pub fn read_native_backup(path: &Path) -> Result<Vec<u8>, NativeInputError> {
    let name = selected_name(path)?;
    if name.len() > 240
        || name.starts_with('.')
        || !name.to_ascii_lowercase().ends_with(".ort-backup")
        || name
            .chars()
            .any(|value| value.is_control() || "<>:\"/\\|?*".contains(value))
    {
        return Err(NativeInputError::InvalidSelection);
    }

    read_bounded_selection(path, name, ort_domain::MAX_BACKUP_BYTES)
}

/// Reads a PDF or DOCX selected by a trusted native dialog into one bounded,
/// parent-owned snapshot. The extension selects the expected format but is not
/// treated as proof of that format; callers must pass the returned bytes to the
/// document source-envelope inspector before staging or worker launch.
///
/// # Errors
/// Rejects non-absolute paths, unsupported or ambiguous names, symlinks,
/// non-regular files, empty/oversized files, changes observable while opening
/// or reading, and unavailable I/O.
pub fn read_native_document(path: &Path) -> Result<NativeDocumentSource, NativeInputError> {
    let name = selected_name(path)?;
    if name.len() > 240
        || name.starts_with('.')
        || name
            .chars()
            .any(|value| value.is_control() || "<>:\"/\\|?*".contains(value))
    {
        return Err(NativeInputError::InvalidSelection);
    }
    let extension = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .ok_or(NativeInputError::InvalidSelection)?;
    let format = if extension.eq_ignore_ascii_case("pdf") {
        NativeDocumentFormat::Pdf
    } else if extension.eq_ignore_ascii_case("docx") {
        NativeDocumentFormat::Docx
    } else {
        return Err(NativeInputError::InvalidSelection);
    };
    let bytes = read_bounded_selection(path, name, ort_domain::MAX_IMPORT_SOURCE_BYTES)?;
    Ok(NativeDocumentSource { format, bytes })
}

fn selected_name(path: &Path) -> Result<&str, NativeInputError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|value| matches!(value, Component::ParentDir))
    {
        return Err(NativeInputError::InvalidSelection);
    }
    path.file_name()
        .and_then(|value| value.to_str())
        .ok_or(NativeInputError::InvalidSelection)
}

fn read_bounded_selection(
    path: &Path,
    name: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, NativeInputError> {
    let parent = Dir::open_ambient_dir(
        path.parent().ok_or(NativeInputError::InvalidSelection)?,
        ambient_authority(),
    )
    .map_err(|_| NativeInputError::Unavailable)?;
    let before = parent
        .symlink_metadata(name)
        .map_err(|_| NativeInputError::Unavailable)?;
    if !before.file_type().is_file() {
        return Err(NativeInputError::InvalidSelection);
    }
    let expected_len = bounded_len(before.len(), max_bytes)?;

    let mut options = OpenOptions::new();
    options.read(true);
    // cap-std exposes this hook for cap-fs-ext. Keeping it here avoids a second
    // path lookup and makes the held directory plus final no-follow behavior
    // explicit on every supported platform.
    options._cap_fs_ext_follow(FollowSymlinks::No);
    let mut file = parent
        .open_with(name, &options)
        .map_err(|_| NativeInputError::InvalidSelection)?;
    let opened = file.metadata().map_err(|_| NativeInputError::Unavailable)?;
    if !opened.file_type().is_file()
        || bounded_len(opened.len(), max_bytes)? != expected_len
        || changed_timestamp(&before, &opened)
    {
        return Err(NativeInputError::InvalidContent);
    }

    let read_limit = u64::try_from(max_bytes)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(NativeInputError::InvalidContent)?;
    let mut bytes = Vec::with_capacity(expected_len);
    file.by_ref()
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|_| NativeInputError::Unavailable)?;
    if bytes.len() != expected_len {
        return Err(NativeInputError::InvalidContent);
    }
    let after = file.metadata().map_err(|_| NativeInputError::Unavailable)?;
    if bounded_len(after.len(), max_bytes)? != expected_len || changed_timestamp(&opened, &after) {
        return Err(NativeInputError::InvalidContent);
    }
    Ok(bytes)
}

fn bounded_len(value: u64, max_bytes: usize) -> Result<usize, NativeInputError> {
    let value = usize::try_from(value).map_err(|_| NativeInputError::InvalidContent)?;
    if value == 0 || value > max_bytes {
        return Err(NativeInputError::InvalidContent);
    }
    Ok(value)
}

fn changed_timestamp(before: &cap_std::fs::Metadata, after: &cap_std::fs::Metadata) -> bool {
    matches!((before.modified(), after.modified()), (Ok(left), Ok(right)) if left != right)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn reads_only_a_bounded_regular_backup_selected_by_absolute_path() {
        let temporary = TempDir::new().unwrap();
        let path = temporary.path().join("synthetic.ort-backup");
        fs::write(&path, b"ORTB synthetic ciphertext").unwrap();
        assert_eq!(
            read_native_backup(&path).unwrap(),
            b"ORTB synthetic ciphertext"
        );

        let wrong_extension = temporary.path().join("synthetic.bin");
        fs::write(&wrong_extension, b"ORTB synthetic ciphertext").unwrap();
        assert_eq!(
            read_native_backup(&wrong_extension),
            Err(NativeInputError::InvalidSelection)
        );
        assert_eq!(
            read_native_backup(temporary.path()),
            Err(NativeInputError::InvalidSelection)
        );
    }

    #[test]
    fn rejects_empty_and_oversized_backups_before_a_successful_read() {
        let temporary = TempDir::new().unwrap();
        let empty = temporary.path().join("empty.ort-backup");
        fs::write(&empty, []).unwrap();
        assert_eq!(
            read_native_backup(&empty),
            Err(NativeInputError::InvalidContent)
        );

        let oversized = temporary.path().join("oversized.ort-backup");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(u64::try_from(ort_domain::MAX_BACKUP_BYTES).unwrap() + 1)
            .unwrap();
        assert_eq!(
            read_native_backup(&oversized),
            Err(NativeInputError::InvalidContent)
        );
    }

    #[cfg(unix)]
    #[test]
    fn never_follows_a_selected_backup_symlink() {
        use std::os::unix::fs::symlink;

        let temporary = TempDir::new().unwrap();
        let target = temporary.path().join("target.ort-backup");
        let selected = temporary.path().join("selected.ort-backup");
        fs::write(&target, b"ORTB private target").unwrap();
        symlink(&target, &selected).unwrap();
        assert_eq!(
            read_native_backup(&selected),
            Err(NativeInputError::InvalidSelection)
        );
    }

    #[test]
    fn reads_only_supported_bounded_document_snapshots() {
        let temporary = TempDir::new().unwrap();
        for (name, format, marker) in [
            (
                "synthetic.PDF",
                NativeDocumentFormat::Pdf,
                b"%PDF synthetic".as_slice(),
            ),
            (
                "synthetic.docx",
                NativeDocumentFormat::Docx,
                b"PK synthetic".as_slice(),
            ),
        ] {
            let path = temporary.path().join(name);
            fs::write(&path, marker).unwrap();
            let source = read_native_document(&path).unwrap();
            assert_eq!(source.format, format);
            assert_eq!(source.bytes, marker);
        }

        for name in ["synthetic.txt", ".synthetic.pdf", "synthetic.pdf.exe"] {
            let path = temporary.path().join(name);
            fs::write(&path, b"synthetic").unwrap();
            assert!(matches!(
                read_native_document(&path),
                Err(NativeInputError::InvalidSelection)
            ));
        }
    }

    #[test]
    fn rejects_empty_and_oversized_document_sources() {
        let temporary = TempDir::new().unwrap();
        let empty = temporary.path().join("empty.pdf");
        fs::write(&empty, []).unwrap();
        assert!(matches!(
            read_native_document(&empty),
            Err(NativeInputError::InvalidContent)
        ));

        let oversized = temporary.path().join("oversized.docx");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(u64::try_from(ort_domain::MAX_IMPORT_SOURCE_BYTES).unwrap() + 1)
            .unwrap();
        assert!(matches!(
            read_native_document(&oversized),
            Err(NativeInputError::InvalidContent)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn never_follows_a_selected_document_symlink() {
        use std::os::unix::fs::symlink;

        let temporary = TempDir::new().unwrap();
        let target = temporary.path().join("target.pdf");
        let selected = temporary.path().join("selected.pdf");
        fs::write(&target, b"%PDF private target").unwrap();
        symlink(&target, &selected).unwrap();
        assert!(matches!(
            read_native_document(&selected),
            Err(NativeInputError::InvalidSelection)
        ));
    }
}
