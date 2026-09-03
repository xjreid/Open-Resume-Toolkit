use std::{
    ffi::OsString,
    io::{self, Write},
    path::{Component, Path},
};

#[cfg(unix)]
use cap_std::fs::{DirBuilderExt, OpenOptionsExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, DirBuilder, OpenOptions},
};
use uuid::Uuid;

const MAX_BYTES: usize = 256 * 1024;

/// Chosen by a fixed native command, never inferred from a renderer path.
#[derive(Clone, Copy)]
pub enum ExportFileType {
    Text,
    Docx,
}

impl ExportFileType {
    const fn extension(self) -> &'static str {
        match self {
            Self::Text => ".txt",
            Self::Docx => ".docx",
        }
    }
    const fn max_bytes(self) -> usize {
        match self {
            Self::Text => MAX_BYTES,
            Self::Docx => 2 * 1024 * 1024,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExportWriteError {
    #[error("choose a new regular filename with the required extension")]
    InvalidDestination,
    #[error("the destination already exists; choose a new filename")]
    AlreadyExists,
    #[error("export content is empty or exceeds its byte limit")]
    InvalidContent,
    #[error("the selected filesystem could not complete the export")]
    Unavailable,
}

pub struct ExportWriteReceipt {
    pub cleanup_pending: bool,
    pub durability_unconfirmed: bool,
}

/// A native-dialog destination capability, consumed exactly once. Never accept
/// its path from renderer IPC. Directory-relative operations remain attached to
/// the approved directory even if its ambient pathname changes afterwards.
pub struct ExportDestination {
    parent: Dir,
    name: OsString,
    file_type: ExportFileType,
}

impl ExportDestination {
    /// Converts a native save-dialog selection into a one-use directory token.
    ///
    /// # Errors
    /// Rejects relative paths, special filenames, existing entries, and I/O errors.
    pub fn from_native_dialog(path: &Path) -> Result<Self, ExportWriteError> {
        Self::for_native_dialog(path, ExportFileType::Text)
    }

    /// Creates a one-use destination for the native command's fixed format.
    ///
    /// # Errors
    /// Rejects unsafe names, wrong extensions, existing entries and I/O errors.
    pub fn for_native_dialog(
        path: &Path,
        file_type: ExportFileType,
    ) -> Result<Self, ExportWriteError> {
        if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(ExportWriteError::InvalidDestination);
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(ExportWriteError::InvalidDestination)?;
        if !safe_filename(name, file_type) {
            return Err(ExportWriteError::InvalidDestination);
        }
        let parent = Dir::open_ambient_dir(
            path.parent().ok_or(ExportWriteError::InvalidDestination)?,
            ambient_authority(),
        )
        .map_err(|_| ExportWriteError::Unavailable)?;
        ensure_absent(&parent, name.as_ref())?;
        Ok(Self {
            parent,
            name: name.into(),
            file_type,
        })
    }

    /// Flushes a sibling staging file and publishes it with a no-clobber link.
    /// Filesystems without hard-link support fail closed; there is no unsafe
    /// copy/overwrite fallback. Only the exact owned staging entries are removed.
    ///
    /// # Errors
    /// Returns a bounded error without replacing any existing target.
    pub fn write(self, bytes: &[u8]) -> Result<ExportWriteReceipt, ExportWriteError> {
        if bytes.is_empty() || bytes.len() > self.file_type.max_bytes() {
            return Err(ExportWriteError::InvalidContent);
        }
        ensure_absent(&self.parent, Path::new(&self.name))?;
        let stage_name = format!(".ort-export-{}", Uuid::now_v7());
        let builder = private_directory_builder();
        self.parent
            .create_dir_with(&stage_name, &builder)
            .map_err(|_| ExportWriteError::Unavailable)?;
        let Ok(stage) = self.parent.open_dir(&stage_name) else {
            let _ = self.parent.remove_dir(&stage_name);
            return Err(ExportWriteError::Unavailable);
        };
        let result = self.write_staged(&stage, bytes);
        let cleaned_file = match stage.remove_file("payload") {
            Ok(()) => true,
            Err(error) => error.kind() == io::ErrorKind::NotFound,
        };
        drop(stage);
        let cleaned_dir = self.parent.remove_dir(&stage_name).is_ok();
        result?;
        // Some filesystems cannot fsync directories. Report committed bytes
        // separately from crash-durability/cleanup warnings; do not retry writes.
        let synced = self.parent.into_std_file().sync_all().is_ok();
        Ok(ExportWriteReceipt {
            cleanup_pending: !cleaned_file || !cleaned_dir,
            durability_unconfirmed: !synced,
        })
    }

    fn write_staged(&self, stage: &Dir, bytes: &[u8]) -> Result<(), ExportWriteError> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = stage
            .open_with("payload", &options)
            .map_err(|_| ExportWriteError::Unavailable)?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| ExportWriteError::Unavailable)?;
        drop(file);
        stage
            .hard_link("payload", &self.parent, Path::new(&self.name))
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    ExportWriteError::AlreadyExists
                } else {
                    ExportWriteError::Unavailable
                }
            })
    }
}

fn private_directory_builder() -> DirBuilder {
    #[cfg(unix)]
    {
        let mut builder = DirBuilder::new();
        builder.mode(0o700);
        builder
    }
    #[cfg(not(unix))]
    DirBuilder::new()
}

fn ensure_absent(parent: &Dir, name: &Path) -> Result<(), ExportWriteError> {
    match parent.symlink_metadata(name) {
        Ok(_) => Err(ExportWriteError::AlreadyExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(ExportWriteError::Unavailable),
    }
}

fn safe_filename(name: &str, file_type: ExportFileType) -> bool {
    if name.len() > 240
        || !name.to_ascii_lowercase().ends_with(file_type.extension())
        || name.starts_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return false;
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return false;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(number) = stem.strip_prefix(prefix)
            && number.chars().count() == 1
            && number.chars().all(|c| "123456789¹²³".contains(c))
        {
            return false;
        }
    }
    !stem.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn docx_uses_same_no_clobber_capability_with_its_own_bound_and_extension() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("synthetic.DOCX");
        let bytes = vec![0x42; ExportFileType::Docx.max_bytes()];
        assert!(ExportDestination::from_native_dialog(&path).is_err());
        assert!(
            ExportDestination::for_native_dialog(
                &dir.path().join("wrong.txt"),
                ExportFileType::Docx
            )
            .is_err()
        );
        let receipt = ExportDestination::for_native_dialog(&path, ExportFileType::Docx)
            .unwrap()
            .write(&bytes)
            .unwrap();
        assert!(!receipt.cleanup_pending);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        assert_eq!(
            ExportDestination::for_native_dialog(&path, ExportFileType::Docx).err(),
            Some(ExportWriteError::AlreadyExists)
        );
        let raced = dir.path().join("race.docx");
        let token = ExportDestination::for_native_dialog(&raced, ExportFileType::Docx).unwrap();
        fs::write(&raced, b"keep").unwrap();
        assert_eq!(
            token.write(b"replace").err(),
            Some(ExportWriteError::AlreadyExists)
        );
        assert_eq!(fs::read(&raced).unwrap(), b"keep");
        for data in [vec![], vec![0; ExportFileType::Docx.max_bytes() + 1]] {
            let token = ExportDestination::for_native_dialog(
                &dir.path().join("invalid.docx"),
                ExportFileType::Docx,
            )
            .unwrap();
            assert_eq!(
                token.write(&data).err(),
                Some(ExportWriteError::InvalidContent)
            );
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
        for name in [
            "CON.docx",
            "LPT1.docx",
            "name:stream.docx",
            ".docx",
            "../x.docx",
            "macro.docm",
        ] {
            assert!(!safe_filename(name, ExportFileType::Docx));
        }
    }

    #[test]
    fn writes_complete_bytes_and_removes_staging() {
        let dir = TempDir::new().expect("dir");
        let path = dir.path().join("synthetic.txt");
        let receipt = ExportDestination::from_native_dialog(&path)
            .expect("select")
            .write("Zoë 示例\n".as_bytes())
            .expect("write");
        assert_eq!(fs::read_to_string(&path).expect("read"), "Zoë 示例\n");
        assert!(!receipt.cleanup_pending);
        assert_eq!(fs::read_dir(dir.path()).expect("list").count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).expect("metadata").permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn never_overwrites_even_if_a_target_appears_after_selection() {
        let dir = TempDir::new().expect("dir");
        let path = dir.path().join("existing.txt");
        let token = ExportDestination::from_native_dialog(&path).expect("select");
        fs::write(&path, "keep this").expect("seed");
        assert_eq!(
            token.write(b"replacement").err(),
            Some(ExportWriteError::AlreadyExists)
        );
        assert_eq!(
            ExportDestination::from_native_dialog(&path).err(),
            Some(ExportWriteError::AlreadyExists)
        );
        assert_eq!(fs::read_to_string(path).expect("read"), "keep this");
    }

    #[test]
    fn rejects_special_names_and_oversized_output_without_creating_files() {
        for name in [
            "CON.txt",
            "LPT1.txt",
            "COM¹.txt",
            "name:stream.txt",
            ".txt",
            "resume.exe",
            "../resume.txt",
            "a\n.txt",
        ] {
            assert!(!safe_filename(name, ExportFileType::Text), "{name}");
        }
        assert!(safe_filename("履歴書.txt", ExportFileType::Text));
        let dir = TempDir::new().expect("dir");
        let token =
            ExportDestination::from_native_dialog(&dir.path().join("large.txt")).expect("select");
        assert_eq!(
            token.write(&vec![0; MAX_BYTES + 1]).err(),
            Some(ExportWriteError::InvalidContent)
        );
        assert_eq!(fs::read_dir(dir.path()).expect("list").count(), 0);
    }

    #[test]
    fn final_publication_cannot_clobber_a_last_moment_target() {
        let dir = TempDir::new().expect("dir");
        let path = dir.path().join("race.txt");
        let token = ExportDestination::from_native_dialog(&path).expect("select");
        token.parent.create_dir("staging").expect("staging");
        let stage = token.parent.open_dir("staging").expect("open");
        // Simulate another writer after the last target-absence check.
        fs::write(&path, "original").expect("racer");
        assert_eq!(
            token.write_staged(&stage, b"replacement"),
            Err(ExportWriteError::AlreadyExists)
        );
        assert_eq!(fs::read_to_string(&path).expect("read"), "original");
        assert_eq!(stage.read("payload").expect("staged"), b"replacement");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_target_and_cannot_be_redirected_by_parent_replacement() {
        use std::os::unix::fs::symlink;
        let dir = TempDir::new().expect("dir");
        let original = dir.path().join("approved");
        let moved = dir.path().join("moved");
        let other = dir.path().join("other");
        fs::create_dir(&original).expect("parent");
        fs::create_dir(&other).expect("other");
        symlink(other.join("missing"), original.join("link.txt")).expect("symlink");
        assert_eq!(
            ExportDestination::from_native_dialog(&original.join("link.txt")).err(),
            Some(ExportWriteError::AlreadyExists)
        );
        let token =
            ExportDestination::from_native_dialog(&original.join("new.txt")).expect("select");
        fs::rename(&original, &moved).expect("move");
        symlink(&other, &original).expect("redirect");
        token.write(b"synthetic").expect("write to held directory");
        assert_eq!(fs::read(moved.join("new.txt")).expect("read"), b"synthetic");
        assert!(!other.join("new.txt").exists());
    }
}
