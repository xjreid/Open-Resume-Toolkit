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
    #[error("choose a regular Open Resume Toolkit backup file")]
    InvalidSelection,
    #[error("the selected backup is empty, changed, or exceeds its byte limit")]
    InvalidContent,
    #[error("the selected backup could not be read")]
    Unavailable,
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
    if !path.is_absolute()
        || path
            .components()
            .any(|value| matches!(value, Component::ParentDir))
    {
        return Err(NativeInputError::InvalidSelection);
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(NativeInputError::InvalidSelection)?;
    if name.len() > 240
        || name.starts_with('.')
        || !name.to_ascii_lowercase().ends_with(".ort-backup")
        || name
            .chars()
            .any(|value| value.is_control() || "<>:\"/\\|?*".contains(value))
    {
        return Err(NativeInputError::InvalidSelection);
    }

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
    let expected_len = bounded_len(before.len())?;

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
    if !opened.file_type().is_file() || bounded_len(opened.len())? != expected_len {
        return Err(NativeInputError::InvalidContent);
    }

    let read_limit = u64::try_from(ort_domain::MAX_BACKUP_BYTES)
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
    Ok(bytes)
}

fn bounded_len(value: u64) -> Result<usize, NativeInputError> {
    let value = usize::try_from(value).map_err(|_| NativeInputError::InvalidContent)?;
    if value == 0 || value > ort_domain::MAX_BACKUP_BYTES {
        return Err(NativeInputError::InvalidContent);
    }
    Ok(value)
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
}
