//! Private, operation-owned staging for hostile document input.
//!
//! This module does not parse documents or launch workers. It creates only a
//! fixed marker and source file beneath a held application-data capability.
//! Platform privacy must be verified before construction; unsupported targets
//! fail closed.

use std::{
    ffi::OsString,
    io::{self, Read, Write},
    path::{Component, Path},
    time::{Duration, SystemTime},
};

#[cfg(unix)]
use cap_std::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, DirBuilder, File, OpenOptions},
};
use uuid::Uuid;

use crate::{NativeDocumentFormat, NativeDocumentSource};

pub const IMPORTS_DIRECTORY: &str = "imports";
pub const IMPORT_STAGE_MAX_AGE: Duration = Duration::from_hours(24);
pub const MAX_IMPORT_STAGE_ENTRIES: usize = 128;
const STAGE_PREFIX: &str = "operation-";
const MARKER_NAME: &str = "owner";
const SOURCE_NAME: &str = "source.bin";
const MARKER_PREFIX: &str = "ORT-IMPORT-STAGE-V1\n";
const MAX_MARKER_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ImportStageError {
    #[error("private import staging is unavailable on this platform")]
    PlatformSecurityUnavailable,
    #[error("the private import staging root is invalid")]
    InvalidRoot,
    #[error("the import source is invalid")]
    InvalidSource,
    #[error("private import staging could not be created")]
    Unavailable,
    #[error("private import staging cleanup is incomplete")]
    CleanupIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportCleanupReport {
    pub scanned: usize,
    pub removed: usize,
    pub preserved: usize,
    pub scan_limit_reached: bool,
}

/// Held capability for the fixed private `imports` directory.
pub struct ImportStagingRoot {
    directory: Dir,
}

impl ImportStagingRoot {
    /// Opens or creates the fixed import-staging child of a trusted application
    /// data directory. The path must originate in native application setup,
    /// never renderer IPC.
    ///
    /// # Errors
    /// Rejects relative/traversing/symlink roots, broad Unix permissions, and
    /// platforms whose current implementation cannot verify private staging.
    pub fn for_app_data(app_data: &Path) -> Result<Self, ImportStageError> {
        supported_staging_root(app_data)
    }

    /// Writes one already-bounded native source into a random private stage and
    /// returns an opaque held read-only file. The original selected path is not
    /// retained or reopened.
    ///
    /// The trusted application layer must validate the source envelope before
    /// calling this method. This layer independently rechecks the shared byte
    /// limit and writes no caller-controlled filename.
    ///
    /// # Errors
    /// Returns a content-free failure if creation, permission verification,
    /// writing, syncing, or read-only reopening cannot be completed.
    pub fn stage(&self, source: NativeDocumentSource) -> Result<StagedImport, ImportStageError> {
        if source.bytes.is_empty() || source.bytes.len() > ort_domain::MAX_IMPORT_SOURCE_BYTES {
            return Err(ImportStageError::InvalidSource);
        }
        let NativeDocumentSource { format, bytes } = source;
        let operation_id = Uuid::now_v7();
        let directory_name = format!("{STAGE_PREFIX}{operation_id}");
        self.directory
            .create_dir_with(&directory_name, &private_directory_builder())
            .map_err(|_| ImportStageError::Unavailable)?;
        let result = self.stage_created(&directory_name, operation_id, format, &bytes);
        if result.is_err()
            && cleanup_partial_stage(&self.directory, Path::new(&directory_name)).is_err()
        {
            return Err(ImportStageError::CleanupIncomplete);
        }
        result
    }

    fn stage_created(
        &self,
        directory_name: &str,
        operation_id: Uuid,
        format: NativeDocumentFormat,
        bytes: &[u8],
    ) -> Result<StagedImport, ImportStageError> {
        let stage = self
            .directory
            .open_dir(directory_name)
            .map_err(|_| ImportStageError::Unavailable)?;
        verify_private_directory(&stage)?;
        let marker = marker(format, bytes.len());
        write_private_file(&stage, MARKER_NAME, marker.as_bytes())?;
        write_private_file(&stage, SOURCE_NAME, bytes)?;
        let input = open_verified_source(&stage, bytes.len())?;
        stage
            .into_std_file()
            .sync_all()
            .map_err(|_| ImportStageError::Unavailable)?;
        Ok(StagedImport {
            parent: self
                .directory
                .try_clone()
                .map_err(|_| ImportStageError::Unavailable)?,
            directory_name: directory_name.into(),
            operation_id,
            format,
            byte_count: bytes.len(),
            input: Some(input),
            cleaned: false,
        })
    }

    /// Removes expired, structurally exact ORT stages. Unknown names, symlinks,
    /// extra entries, malformed markers, fresh stages, and future timestamps are
    /// preserved for explicit repair instead of recursively deleted.
    ///
    /// # Errors
    /// Returns a content-free error if the bounded root scan cannot complete.
    pub fn cleanup_expired(&self) -> Result<ImportCleanupReport, ImportStageError> {
        self.cleanup_expired_at(SystemTime::now(), IMPORT_STAGE_MAX_AGE)
    }

    fn cleanup_expired_at(
        &self,
        now: SystemTime,
        minimum_age: Duration,
    ) -> Result<ImportCleanupReport, ImportStageError> {
        let mut names = Vec::new();
        let entries = self
            .directory
            .entries()
            .map_err(|_| ImportStageError::Unavailable)?;
        for entry in entries.take(MAX_IMPORT_STAGE_ENTRIES + 1) {
            let entry = entry.map_err(|_| ImportStageError::Unavailable)?;
            names.push(entry.file_name());
        }
        let scan_limit_reached = names.len() > MAX_IMPORT_STAGE_ENTRIES;
        names.truncate(MAX_IMPORT_STAGE_ENTRIES);
        let mut report = ImportCleanupReport {
            scanned: names.len(),
            removed: 0,
            preserved: 0,
            scan_limit_reached,
        };
        for name in names {
            let path = Path::new(&name);
            if !owned_stage_name(path) || !expired(&self.directory, path, now, minimum_age) {
                report.preserved += 1;
                continue;
            }
            if cleanup_known_stage(&self.directory, path).is_ok() {
                report.removed += 1;
            } else {
                report.preserved += 1;
            }
        }
        Ok(report)
    }
}

/// Operation-owned stage. It exposes no filesystem path; native containment
/// code receives only the transferred read-only file handle and expected format.
pub struct StagedImport {
    parent: Dir,
    directory_name: OsString,
    operation_id: Uuid,
    format: NativeDocumentFormat,
    byte_count: usize,
    input: Option<File>,
    cleaned: bool,
}

impl StagedImport {
    #[must_use]
    pub const fn operation_id(&self) -> Uuid {
        self.operation_id
    }

    #[must_use]
    pub const fn format(&self) -> NativeDocumentFormat {
        self.format
    }

    #[must_use]
    pub const fn byte_count(&self) -> usize {
        self.byte_count
    }

    /// Transfers the held read-only input exactly once to a reviewed native
    /// containment adapter. No pathname is disclosed and no duplicated handle
    /// can share or alter a later parser's seek position.
    ///
    /// # Errors
    /// Returns a content-free failure if the handle was already transferred.
    pub fn take_input(&mut self) -> Result<std::fs::File, ImportStageError> {
        let input = self
            .input
            .take()
            .ok_or(ImportStageError::CleanupIncomplete)?;
        Ok(input.into_std())
    }

    /// Closes the stage-owned handle and removes only the two fixed files and
    /// exact random operation directory. The transferred handle must be closed
    /// before this is treated as verified cleanup.
    ///
    /// # Errors
    /// Returns a content-free error and leaves unrecognized data intact when
    /// exact cleanup cannot be established.
    pub fn cleanup(mut self) -> Result<(), ImportStageError> {
        self.input.take();
        cleanup_known_stage(&self.parent, Path::new(&self.directory_name))?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for StagedImport {
    fn drop(&mut self) {
        self.input.take();
        if !self.cleaned
            && cleanup_known_stage(&self.parent, Path::new(&self.directory_name)).is_ok()
        {
            self.cleaned = true;
        }
    }
}

impl std::fmt::Debug for StagedImport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StagedImport")
            .field("operation_id", &self.operation_id)
            .field("format", &self.format)
            .field("byte_count", &self.byte_count)
            .finish_non_exhaustive()
    }
}

#[cfg(unix)]
fn supported_staging_root(app_data: &Path) -> Result<ImportStagingRoot, ImportStageError> {
    if !app_data.is_absolute()
        || app_data
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(ImportStageError::InvalidRoot);
    }
    let metadata =
        std::fs::symlink_metadata(app_data).map_err(|_| ImportStageError::InvalidRoot)?;
    if !metadata.file_type().is_dir() {
        return Err(ImportStageError::InvalidRoot);
    }
    let app = Dir::open_ambient_dir(app_data, ambient_authority())
        .map_err(|_| ImportStageError::InvalidRoot)?;
    match app.symlink_metadata(IMPORTS_DIRECTORY) {
        Ok(found) if found.file_type().is_dir() => {}
        Ok(_) => return Err(ImportStageError::InvalidRoot),
        Err(error) if error.kind() == io::ErrorKind::NotFound => app
            .create_dir_with(IMPORTS_DIRECTORY, &private_directory_builder())
            .map_err(|_| ImportStageError::Unavailable)?,
        Err(_) => return Err(ImportStageError::Unavailable),
    }
    let directory = app
        .open_dir(IMPORTS_DIRECTORY)
        .map_err(|_| ImportStageError::InvalidRoot)?;
    verify_private_directory(&directory)?;
    Ok(ImportStagingRoot { directory })
}

#[cfg(not(unix))]
fn supported_staging_root(_app_data: &Path) -> Result<ImportStagingRoot, ImportStageError> {
    Err(ImportStageError::PlatformSecurityUnavailable)
}

#[cfg(unix)]
fn private_directory_builder() -> DirBuilder {
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    builder
}

#[cfg(not(unix))]
fn private_directory_builder() -> DirBuilder {
    DirBuilder::new()
}

#[cfg(unix)]
fn verify_private_directory(directory: &Dir) -> Result<(), ImportStageError> {
    let metadata = directory
        .dir_metadata()
        .map_err(|_| ImportStageError::Unavailable)?;
    if !metadata.file_type().is_dir() || metadata.permissions().mode() & 0o777 != 0o700 {
        return Err(ImportStageError::InvalidRoot);
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_private_directory(_directory: &Dir) -> Result<(), ImportStageError> {
    Err(ImportStageError::PlatformSecurityUnavailable)
}

fn write_private_file(directory: &Dir, name: &str, bytes: &[u8]) -> Result<(), ImportStageError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = directory
        .open_with(name, &options)
        .map_err(|_| ImportStageError::Unavailable)?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| ImportStageError::Unavailable)
}

fn open_verified_source(directory: &Dir, expected: usize) -> Result<File, ImportStageError> {
    let mut options = OpenOptions::new();
    options.read(true);
    options._cap_fs_ext_follow(cap_primitives::fs::FollowSymlinks::No);
    let file = directory
        .open_with(SOURCE_NAME, &options)
        .map_err(|_| ImportStageError::Unavailable)?;
    let metadata = file.metadata().map_err(|_| ImportStageError::Unavailable)?;
    if !metadata.file_type().is_file() || usize::try_from(metadata.len()).ok() != Some(expected) {
        return Err(ImportStageError::InvalidSource);
    }
    #[cfg(unix)]
    {
        if metadata.permissions().mode() & 0o777 != 0o600 {
            return Err(ImportStageError::InvalidRoot);
        }
    }
    Ok(file)
}

fn marker(format: NativeDocumentFormat, byte_count: usize) -> String {
    let format = match format {
        NativeDocumentFormat::Pdf => "pdf",
        NativeDocumentFormat::Docx => "docx",
    };
    format!("{MARKER_PREFIX}format={format}\nbytes={byte_count}\n")
}

fn valid_marker(bytes: &[u8], source_bytes: u64) -> bool {
    let Ok(value) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut lines = value.lines();
    if lines.next() != Some(MARKER_PREFIX.trim_end()) {
        return false;
    }
    if !matches!(lines.next(), Some("format=pdf" | "format=docx")) {
        return false;
    }
    let declared = lines
        .next()
        .and_then(|line| line.strip_prefix("bytes="))
        .and_then(|value| value.parse::<u64>().ok());
    declared == Some(source_bytes)
        && source_bytes > 0
        && source_bytes <= u64::try_from(ort_domain::MAX_IMPORT_SOURCE_BYTES).unwrap_or(u64::MAX)
        && lines.next().is_none()
        && value.ends_with('\n')
}

fn owned_stage_name(path: &Path) -> bool {
    let Some(name) = path.to_str() else {
        return false;
    };
    let Some(identifier) = name.strip_prefix(STAGE_PREFIX) else {
        return false;
    };
    Uuid::parse_str(identifier).is_ok_and(|uuid| uuid.get_version_num() == 7)
}

fn expired(directory: &Dir, path: &Path, now: SystemTime, minimum_age: Duration) -> bool {
    let Ok(metadata) = directory.symlink_metadata(path) else {
        return false;
    };
    metadata.file_type().is_dir()
        && metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified.into_std()).ok())
            .is_some_and(|age| age >= minimum_age)
}

fn cleanup_known_stage(parent: &Dir, name: &Path) -> Result<(), ImportStageError> {
    if !owned_stage_name(name) {
        return Err(ImportStageError::CleanupIncomplete);
    }
    let metadata = parent
        .symlink_metadata(name)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    if !metadata.file_type().is_dir() {
        return Err(ImportStageError::CleanupIncomplete);
    }
    let stage = parent
        .open_dir(name)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    verify_private_directory(&stage).map_err(|_| ImportStageError::CleanupIncomplete)?;
    let mut names = Vec::new();
    for entry in stage
        .entries()
        .map_err(|_| ImportStageError::CleanupIncomplete)?
        .take(3)
    {
        names.push(
            entry
                .map_err(|_| ImportStageError::CleanupIncomplete)?
                .file_name(),
        );
    }
    names.sort_unstable();
    if names != [OsString::from(MARKER_NAME), OsString::from(SOURCE_NAME)] {
        return Err(ImportStageError::CleanupIncomplete);
    }
    let source = stage
        .symlink_metadata(SOURCE_NAME)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    let marker_metadata = stage
        .symlink_metadata(MARKER_NAME)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    if !source.file_type().is_file()
        || !marker_metadata.file_type().is_file()
        || marker_metadata.len() > u64::try_from(MAX_MARKER_BYTES).unwrap_or(u64::MAX)
    {
        return Err(ImportStageError::CleanupIncomplete);
    }
    let marker_bytes = read_marker(&stage)?;
    if !valid_marker(&marker_bytes, source.len()) {
        return Err(ImportStageError::CleanupIncomplete);
    }
    stage
        .remove_file(SOURCE_NAME)
        .and_then(|()| stage.remove_file(MARKER_NAME))
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    drop(stage);
    parent
        .remove_dir(name)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    parent
        .try_clone()
        .and_then(|directory| directory.into_std_file().sync_all())
        .map_err(|_| ImportStageError::CleanupIncomplete)
}

fn cleanup_partial_stage(parent: &Dir, name: &Path) -> Result<(), ImportStageError> {
    if !owned_stage_name(name) {
        return Err(ImportStageError::CleanupIncomplete);
    }
    let metadata = parent
        .symlink_metadata(name)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    if !metadata.file_type().is_dir() {
        return Err(ImportStageError::CleanupIncomplete);
    }
    let stage = parent
        .open_dir(name)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    verify_private_directory(&stage).map_err(|_| ImportStageError::CleanupIncomplete)?;
    let mut names = Vec::new();
    for entry in stage
        .entries()
        .map_err(|_| ImportStageError::CleanupIncomplete)?
        .take(3)
    {
        names.push(
            entry
                .map_err(|_| ImportStageError::CleanupIncomplete)?
                .file_name(),
        );
    }
    if names.len() > 2
        || names
            .iter()
            .any(|entry| entry != MARKER_NAME && entry != SOURCE_NAME)
    {
        return Err(ImportStageError::CleanupIncomplete);
    }
    for entry in [SOURCE_NAME, MARKER_NAME] {
        match stage.symlink_metadata(entry) {
            Ok(found) if found.file_type().is_file() => stage
                .remove_file(entry)
                .map_err(|_| ImportStageError::CleanupIncomplete)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Ok(_) | Err(_) => return Err(ImportStageError::CleanupIncomplete),
        }
    }
    drop(stage);
    parent
        .remove_dir(name)
        .map_err(|_| ImportStageError::CleanupIncomplete)
}

fn read_marker(directory: &Dir) -> Result<Vec<u8>, ImportStageError> {
    let mut options = OpenOptions::new();
    options.read(true);
    options._cap_fs_ext_follow(cap_primitives::fs::FollowSymlinks::No);
    let mut file = directory
        .open_with(MARKER_NAME, &options)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(u64::try_from(MAX_MARKER_BYTES + 1).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|_| ImportStageError::CleanupIncomplete)?;
    if bytes.len() > MAX_MARKER_BYTES {
        return Err(ImportStageError::CleanupIncomplete);
    }
    Ok(bytes)
}

#[cfg(all(test, unix))]
mod tests {
    use std::{fs, io::Read, os::unix::fs::PermissionsExt};

    use tempfile::TempDir;

    use super::*;

    fn source(format: NativeDocumentFormat) -> NativeDocumentSource {
        NativeDocumentSource {
            format,
            bytes: b"SYNTHETIC_HOSTILE_DOCUMENT".to_vec(),
        }
    }

    #[test]
    fn stages_private_fixed_files_and_transfers_one_read_only_handle() {
        let temporary = TempDir::new().unwrap();
        let root = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        let staged = root.stage(source(NativeDocumentFormat::Pdf)).unwrap();
        assert_eq!(staged.format(), NativeDocumentFormat::Pdf);
        assert_eq!(staged.byte_count(), 26);
        assert_eq!(staged.operation_id().get_version_num(), 7);
        let mut staged = staged;
        let mut input = staged.take_input().unwrap();
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"SYNTHETIC_HOSTILE_DOCUMENT");
        assert!(input.write_all(b"no").is_err());
        assert!(matches!(
            staged.take_input(),
            Err(ImportStageError::CleanupIncomplete)
        ));

        let imports = temporary.path().join(IMPORTS_DIRECTORY);
        assert_eq!(
            fs::metadata(&imports).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let operation = fs::read_dir(&imports)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(
            fs::metadata(&operation).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(operation.join(SOURCE_NAME))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        drop(input);
        staged.cleanup().unwrap();
        assert_eq!(fs::read_dir(imports).unwrap().count(), 0);
    }

    #[test]
    fn drop_removes_only_the_exact_owned_stage() {
        let temporary = TempDir::new().unwrap();
        let root = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        let unrelated = temporary.path().join(IMPORTS_DIRECTORY).join("keep");
        fs::create_dir(&unrelated).unwrap();
        {
            let _staged = root.stage(source(NativeDocumentFormat::Docx)).unwrap();
            assert_eq!(
                fs::read_dir(temporary.path().join(IMPORTS_DIRECTORY))
                    .unwrap()
                    .count(),
                2
            );
        }
        assert!(unrelated.is_dir());
        assert_eq!(
            fs::read_dir(temporary.path().join(IMPORTS_DIRECTORY))
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn cleanup_preserves_fresh_unknown_and_tampered_entries() {
        let temporary = TempDir::new().unwrap();
        let root = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        let staged = root.stage(source(NativeDocumentFormat::Pdf)).unwrap();
        let operation = staged.directory_name.clone();
        std::mem::forget(staged);
        let imports = temporary.path().join(IMPORTS_DIRECTORY);
        fs::create_dir(imports.join("unknown")).unwrap();
        let report = root
            .cleanup_expired_at(SystemTime::now(), IMPORT_STAGE_MAX_AGE)
            .unwrap();
        assert_eq!(report.removed, 0);
        assert_eq!(report.preserved, 2);

        fs::write(imports.join(&operation).join("extra"), b"keep").unwrap();
        let report = root
            .cleanup_expired_at(SystemTime::now(), Duration::ZERO)
            .unwrap();
        assert_eq!(report.removed, 0);
        assert_eq!(report.preserved, 2);
        assert!(imports.join(operation).join("extra").is_file());
    }

    #[test]
    fn cleanup_removes_an_expired_exact_stage_and_rejects_broad_roots() {
        let temporary = TempDir::new().unwrap();
        let root = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        let staged = root.stage(source(NativeDocumentFormat::Pdf)).unwrap();
        std::mem::forget(staged);
        let report = root
            .cleanup_expired_at(SystemTime::now(), Duration::ZERO)
            .unwrap();
        assert_eq!(report.removed, 1);
        assert_eq!(report.preserved, 0);

        let imports = temporary.path().join(IMPORTS_DIRECTORY);
        fs::set_permissions(&imports, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            ImportStagingRoot::for_app_data(temporary.path()),
            Err(ImportStageError::InvalidRoot)
        ));
    }

    #[test]
    fn invalid_source_and_stage_symlink_fail_closed() {
        use std::os::unix::fs::symlink;

        let temporary = TempDir::new().unwrap();
        let root = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        assert!(matches!(
            root.stage(NativeDocumentSource {
                format: NativeDocumentFormat::Pdf,
                bytes: Vec::new(),
            }),
            Err(ImportStageError::InvalidSource)
        ));

        let other = TempDir::new().unwrap();
        let linked = TempDir::new().unwrap();
        symlink(other.path(), linked.path().join(IMPORTS_DIRECTORY)).unwrap();
        assert!(matches!(
            ImportStagingRoot::for_app_data(linked.path()),
            Err(ImportStageError::InvalidRoot)
        ));
    }
}
