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
    Backup,
    Text,
    Docx,
    Pdf,
}

impl ExportFileType {
    const fn extension(self) -> &'static str {
        match self {
            Self::Backup => ".ort-backup",
            Self::Text => ".txt",
            Self::Docx => ".docx",
            Self::Pdf => ".pdf",
        }
    }
    const fn max_bytes(self) -> usize {
        match self {
            Self::Backup => ort_domain::MAX_BACKUP_BYTES,
            Self::Text => MAX_BYTES,
            Self::Docx => 2 * 1024 * 1024,
            Self::Pdf => ort_domain::MAX_PDF_BYTES,
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

    /// Publishes complete bytes without replacing an existing target. macOS
    /// unlinks staging before writing and uses atomic file cloning; other
    /// platforms retain sibling staging and no-clobber hard-link publication.
    /// Unsupported filesystems fail closed without a copy/overwrite fallback.
    ///
    /// # Errors
    /// Returns a bounded error without replacing any existing target.
    pub fn write(self, bytes: &[u8]) -> Result<ExportWriteReceipt, ExportWriteError> {
        if bytes.is_empty() || bytes.len() > self.file_type.max_bytes() {
            return Err(ExportWriteError::InvalidContent);
        }
        ensure_absent(&self.parent, Path::new(&self.name))?;
        #[cfg(target_os = "macos")]
        {
            self.write_unlinked(bytes, |_| {})
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.write_named(bytes)
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn write_named(self, bytes: &[u8]) -> Result<ExportWriteReceipt, ExportWriteError> {
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

    #[cfg(not(target_os = "macos"))]
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

    #[cfg(target_os = "macos")]
    fn write_unlinked(
        self,
        bytes: &[u8],
        mut checkpoint: impl FnMut(ExportPhase),
    ) -> Result<ExportWriteReceipt, ExportWriteError> {
        let stage_name = format!(".ort-export-{}", Uuid::now_v7());
        self.parent
            .create_dir_with(&stage_name, &private_directory_builder())
            .map_err(|_| ExportWriteError::Unavailable)?;
        let Ok(stage) = self.parent.open_dir(&stage_name) else {
            let _ = self.parent.remove_dir(&stage_name);
            return Err(ExportWriteError::Unavailable);
        };
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true).mode(0o600);
        let file = stage.open_with("payload", &options);
        let Ok(mut file) = file else {
            drop(stage);
            let _ = self.parent.remove_dir(&stage_name);
            return Err(ExportWriteError::Unavailable);
        };
        checkpoint(ExportPhase::EmptyNamedStage);
        // No document bytes may be written until every staging name is removed
        // and those namespace changes have been flushed. Never scan old exports.
        stage
            .remove_file("payload")
            .map_err(|_| ExportWriteError::Unavailable)?;
        if rustix::fs::fstat(&file)
            .map_err(|_| ExportWriteError::Unavailable)?
            .st_nlink
            != 0
        {
            return Err(ExportWriteError::Unavailable);
        }
        stage
            .into_std_file()
            .sync_all()
            .map_err(|_| ExportWriteError::Unavailable)?;
        self.parent
            .remove_dir(&stage_name)
            .map_err(|_| ExportWriteError::Unavailable)?;
        self.parent
            .try_clone()
            .and_then(|dir| dir.into_std_file().sync_all())
            .map_err(|_| ExportWriteError::Unavailable)?;
        checkpoint(ExportPhase::Unlinked);
        let (first, rest) = bytes.split_at(bytes.len() / 2);
        file.write_all(first)
            .map_err(|_| ExportWriteError::Unavailable)?;
        checkpoint(ExportPhase::PayloadPartial);
        file.write_all(rest)
            .and_then(|()| file.sync_all())
            .map_err(|_| ExportWriteError::Unavailable)?;
        checkpoint(ExportPhase::PayloadFlushed);
        // The held directory remains the native-dialog capability even if its
        // ambient path changes. clonefile requires a new final name atomically.
        rustix::fs::fclonefileat(
            &file,
            &self.parent,
            Path::new(&self.name),
            rustix::fs::CloneFlags::NOOWNERCOPY,
        )
        .map_err(|error| {
            if error == rustix::io::Errno::EXIST {
                ExportWriteError::AlreadyExists
            } else {
                ExportWriteError::Unavailable
            }
        })?;
        checkpoint(ExportPhase::Published);
        drop(file); // Kernel reclaims the unnamed staging inode, including on process death.
        let synced = self.parent.into_std_file().sync_all().is_ok();
        Ok(ExportWriteReceipt {
            cleanup_pending: false,
            durability_unconfirmed: !synced,
        })
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportPhase {
    EmptyNamedStage,
    Unlinked,
    PayloadPartial,
    PayloadFlushed,
    Published,
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
    fn backup_uses_private_no_clobber_publication_and_its_fixed_extension() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("synthetic.ort-backup");
        let bytes = b"ORTB-synthetic-encrypted-container";
        let receipt = ExportDestination::for_native_dialog(&path, ExportFileType::Backup)
            .unwrap()
            .write(bytes)
            .unwrap();
        assert!(!receipt.cleanup_pending);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            ExportDestination::for_native_dialog(&path, ExportFileType::Backup).err(),
            Some(ExportWriteError::AlreadyExists)
        );
        assert!(
            ExportDestination::for_native_dialog(
                &dir.path().join("synthetic.backup"),
                ExportFileType::Backup
            )
            .is_err()
        );
        let large = dir.path().join("large.ort-backup");
        assert!(matches!(
            ExportDestination::for_native_dialog(&large, ExportFileType::Backup)
                .unwrap()
                .write(&vec![0; ort_domain::MAX_BACKUP_BYTES + 1]),
            Err(ExportWriteError::InvalidContent)
        ));
        assert!(!large.exists());
    }

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

    #[cfg(not(target_os = "macos"))]
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

    #[cfg(target_os = "macos")]
    #[test]
    fn final_publication_cannot_clobber_a_last_moment_target() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("race.txt");
        let token = ExportDestination::from_native_dialog(&path).unwrap();
        let result = token.write_unlinked(b"replacement", |phase| {
            if phase == ExportPhase::PayloadFlushed {
                fs::write(&path, "original").unwrap();
            }
        });
        assert_eq!(result.err(), Some(ExportWriteError::AlreadyExists));
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn export_crash_child() {
        let Some(root) = std::env::var_os("ORT_EXPORT_CRASH_TEST_ROOT") else {
            return;
        };
        let phase = std::env::var("ORT_EXPORT_CRASH_TEST_PHASE").unwrap();
        let token =
            ExportDestination::from_native_dialog(&Path::new(&root).join("resume.txt")).unwrap();
        token
            .write_unlinked(b"synthetic export crash fixture", |checkpoint| {
                let name = match checkpoint {
                    ExportPhase::EmptyNamedStage => "empty",
                    ExportPhase::Unlinked => "unlinked",
                    ExportPhase::PayloadPartial => "partial",
                    ExportPhase::PayloadFlushed => "flushed",
                    ExportPhase::Published => "published",
                };
                if name == phase {
                    // SIGKILL skips Rust destructors and application cleanup entirely.
                    std::process::Command::new("/bin/kill")
                        .args(["-KILL", &std::process::id().to_string()])
                        .status()
                        .unwrap();
                    panic!("test child survived SIGKILL");
                }
            })
            .unwrap();
        panic!("test checkpoint was not reached");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn failed_clone_does_not_fall_back_to_a_visible_partial_file() {
        let root = TempDir::new().unwrap();
        let parent = root.path().join("selected");
        fs::create_dir(&parent).unwrap();
        let token = ExportDestination::from_native_dialog(&parent.join("resume.txt")).unwrap();
        let result = token.write_unlinked(b"synthetic fixture", |phase| {
            if phase == ExportPhase::PayloadFlushed {
                // Remove the now-empty chosen directory while its capability
                // remains open: publication must fail without another path.
                fs::remove_dir(&parent).unwrap();
            }
        });
        assert_eq!(result.err(), Some(ExportWriteError::Unavailable));
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn killed_export_leaves_only_empty_staging_or_complete_publication() {
        use std::os::unix::process::ExitStatusExt;
        for phase in ["empty", "unlinked", "partial", "flushed", "published"] {
            let root = TempDir::new().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "export::tests::export_crash_child",
                    "--nocapture",
                ])
                .env("ORT_EXPORT_CRASH_TEST_ROOT", root.path())
                .env("ORT_EXPORT_CRASH_TEST_PHASE", phase)
                .output()
                .unwrap();
            assert_eq!(output.status.signal(), Some(9), "{phase}: {output:?}");
            let entries: Vec<_> = fs::read_dir(root.path())
                .unwrap()
                .map(Result::unwrap)
                .collect();
            match phase {
                "empty" => {
                    assert_eq!(entries.len(), 1);
                    assert!(
                        entries[0]
                            .file_name()
                            .to_str()
                            .unwrap()
                            .starts_with(".ort-export-")
                    );
                    let files: Vec<_> = fs::read_dir(entries[0].path())
                        .unwrap()
                        .map(Result::unwrap)
                        .collect();
                    assert_eq!(files.len(), 1);
                    assert_eq!(files[0].file_name(), "payload");
                    assert_eq!(files[0].metadata().unwrap().len(), 0);
                }
                "published" => {
                    assert_eq!(entries.len(), 1);
                    assert_eq!(
                        fs::read(root.path().join("resume.txt")).unwrap(),
                        b"synthetic export crash fixture"
                    );
                }
                _ => assert!(entries.is_empty(), "{phase} left a named file"),
            }
        }
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
#[test]
fn pdf_uses_exact_bytes_a_fixed_extension_and_no_overwrite() {
    use std::fs;
    use tempfile::TempDir;
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("resume.pdf");
    let target = ExportDestination::for_native_dialog(&path, ExportFileType::Pdf).unwrap();
    let bytes = b"%PDF-synthetic-fixture-only";
    target.write(bytes).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(matches!(
        ExportDestination::for_native_dialog(&path, ExportFileType::Pdf),
        Err(ExportWriteError::AlreadyExists)
    ));
    assert!(
        ExportDestination::for_native_dialog(
            &directory.path().join("wrong.txt"),
            ExportFileType::Pdf
        )
        .is_err()
    );
    let large = directory.path().join("large.pdf");
    assert!(matches!(
        ExportDestination::for_native_dialog(&large, ExportFileType::Pdf)
            .unwrap()
            .write(&vec![0; ort_domain::MAX_PDF_BYTES + 1]),
        Err(ExportWriteError::InvalidContent)
    ));
    assert!(!large.exists());
}
