//! Private Windows drag exports. Cleanup never traverses a reparse point or
//! recursively removes a directory: only the exact owned layout is retired.
use ort_windows::{PrivateDirectory, reject_reparse_path, secure_file};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use uuid::Uuid;
const OWNER: &str = ".ort-drag-owner";
const MARKER: &[u8] = b"ORT-WINDOWS-DRAG-V1\n";
const RETENTION: Duration = Duration::from_hours(24);
const MAX_ENTRIES: usize = 512;
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Private drag artifact unavailable",
    )
}
fn root() -> io::Result<PathBuf> {
    Ok(
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or_else(denied)?)
            .join("com.openresumetoolkit.dev-drag"),
    )
}
fn valid_id(name: &str) -> bool {
    Uuid::parse_str(name).is_ok_and(|id| id.get_version_num() == 7 && id.to_string() == name)
}
fn valid_name(name: &str) -> bool {
    matches!(
        name,
        "tailored-resume.pdf" | "tailored-resume.docx" | "cover-letter.pdf" | "cover-letter.docx"
    )
}
fn children(path: &Path) -> io::Result<Vec<PathBuf>> {
    let entries: Vec<_> = fs::read_dir(path)?
        .take(MAX_ENTRIES + 1)
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<_>>()?;
    if entries.len() > MAX_ENTRIES {
        return Err(denied());
    }
    Ok(entries)
}

/// The lifetime is extended by native drag leases until the OS callback returns.
pub struct WindowsDragArtifact {
    path: PathBuf,
    directory: Option<PrivateDirectory>,
    ownership: Option<File>,
    _root: PrivateDirectory,
    #[cfg(test)]
    preserve: bool,
}
impl WindowsDragArtifact {
    /// # Errors
    /// Requires a private local NTFS location and exclusive ownership marker.
    pub fn new() -> io::Result<Self> {
        Self::create_in(&root()?)
    }
    fn create_in(root: &Path) -> io::Result<Self> {
        let root_lease = PrivateDirectory::create(root)?;
        let path = root.join(format!("ort-drag-{}", Uuid::now_v7()));
        let directory = PrivateDirectory::create(&path)?;
        let marker = path.join(OWNER);
        let mut ownership = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(1)
            .open(&marker)?;
        secure_file(&marker)?;
        ownership.write_all(MARKER)?;
        ownership.sync_all()?;
        Ok(Self {
            path,
            directory: Some(directory),
            ownership: Some(ownership),
            _root: root_lease,
            #[cfg(test)]
            preserve: false,
        })
    }
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// # Errors
    /// Accepts only fixed export filenames and bounded nonempty bytes. The
    /// current-user ACL is present before any unencrypted export bytes are written.
    pub fn write(&self, bytes: &[u8], name: &str) -> io::Result<PathBuf> {
        if !valid_name(name)
            || bytes.is_empty()
            || bytes.len() > 64 * 1024 * 1024
            || children(&self.path)?.len() >= MAX_ENTRIES
        {
            return Err(denied());
        }
        let folder = self.path.join(Uuid::now_v7().to_string());
        let _folder = PrivateDirectory::create(&folder)?;
        let path = folder.join(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        secure_file(&path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(path)
    }
}
impl Drop for WindowsDragArtifact {
    fn drop(&mut self) {
        #[cfg(test)]
        if self.preserve {
            return;
        }
        let _ = cleanup(&self.path, &mut self.directory, &mut self.ownership);
    }
}
fn cleanup(
    path: &Path,
    directory: &mut Option<PrivateDirectory>,
    ownership: &mut Option<File>,
) -> io::Result<()> {
    if directory.is_none() || ownership.is_none() {
        return Err(denied());
    }
    let folders = children(path)?;
    // Validate the complete shallow schema before removing any content.
    for folder in &folders {
        if folder.file_name().and_then(|name| name.to_str()) == Some(OWNER) {
            continue;
        }
        if !folder
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(valid_id)
        {
            return Err(denied());
        }
        let _folder = PrivateDirectory::open(folder)?;
        let files = children(folder)?;
        if files.len() > 4 {
            return Err(denied());
        }
        for file in files {
            if !file
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(valid_name)
            {
                return Err(denied());
            }
            secure_file(&file)?;
        }
    }
    for folder in folders {
        if folder.file_name().and_then(|name| name.to_str()) == Some(OWNER) {
            continue;
        }
        let lease = PrivateDirectory::open(&folder)?;
        for file in children(&folder)? {
            secure_file(&file)?;
            fs::remove_file(file)?;
        }
        drop(lease);
        fs::remove_dir(folder)?;
    }
    ownership.take();
    fs::remove_file(path.join(OWNER))?;
    directory.take();
    fs::remove_dir(path)
}

/// Best-effort startup cleanup; live leases, unsafe/unrecognized entries and
/// artifacts younger than 24 hours are preserved. Inventory is streamed on a
/// background thread so a large backlog neither blocks startup nor disables cleanup.
pub fn sweep_windows_drag_artifacts() {
    if let Ok(root) = root() {
        let _ = std::thread::Builder::new()
            .name("ort-drag-cleanup".into())
            .spawn(move || sweep_in(&root, SystemTime::now()));
    }
}
fn sweep_in(root: &Path, now: SystemTime) {
    let Ok(_root) = PrivateDirectory::open(root) else {
        return;
    };
    // The root is an inventory, not an individual artifact's strict layout.
    // Stream it with constant memory and preserve the iterator across batches;
    // truncating or rejecting this inventory would starve later stale artifacts.
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for (index, entry) in entries.enumerate() {
        if index % MAX_ENTRIES == 0 {
            std::thread::yield_now();
        }
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let valid = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix("ort-drag-"))
            .is_some_and(valid_id);
        if !valid {
            continue;
        }
        let Ok(directory) = PrivateDirectory::open(&path) else {
            continue;
        };
        if path
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_none_or(|age| age < RETENTION)
        {
            continue;
        }
        let marker = path.join(OWNER);
        let prepared = (|| {
            let _guards = reject_reparse_path(&marker)?;
            secure_file(&marker)
        })();
        if prepared.is_err() {
            continue;
        }
        let Ok(mut ownership) = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(1)
            .open(&marker)
        else {
            continue;
        };
        let mut bytes = Vec::new();
        if Read::by_ref(&mut ownership)
            .take(64)
            .read_to_end(&mut bytes)
            .is_err()
            || bytes != MARKER
        {
            continue;
        }
        let _ = cleanup(&path, &mut Some(directory), &mut Some(ownership));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_oversized_inventory_does_not_starve_stale_exports_or_remove_live_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("private");
        let live = WindowsDragArtifact::create_in(&root).unwrap();
        let live_path = live.write(b"LIVE", "tailored-resume.pdf").unwrap();
        for index in 0..=MAX_ENTRIES {
            fs::create_dir(root.join(format!("unrelated-{index:04}"))).unwrap();
        }
        let mut stale_paths = Vec::new();
        for _ in 0..3 {
            let mut artifact = WindowsDragArtifact::create_in(&root).unwrap();
            stale_paths.push(artifact.write(b"STALE", "cover-letter.docx").unwrap());
            artifact.preserve = true;
        }
        sweep_in(
            &root,
            SystemTime::now() + RETENTION + Duration::from_secs(1),
        );
        assert!(stale_paths.iter().all(|path| !path.exists()));
        assert_eq!(fs::read(live_path).unwrap(), b"LIVE");
        assert!(root.join("unrelated-0000").is_dir());
        assert!(root.join(format!("unrelated-{MAX_ENTRIES:04}")).is_dir());
    }
    #[test]
    fn active_exports_are_private_and_removed_only_after_lease_release() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("private");
        let artifact = WindowsDragArtifact::create_in(&root).unwrap();
        let path = artifact.write(b"SYNTHETIC", "tailored-resume.pdf").unwrap();
        let original = artifact.path().to_owned();
        assert_eq!(fs::read(&path).unwrap(), b"SYNTHETIC");
        assert!(artifact.write(b"X", "../escape.pdf").is_err());
        sweep_in(
            &root,
            SystemTime::now() + RETENTION + Duration::from_secs(1),
        );
        assert!(path.exists());
        drop(artifact);
        assert!(!original.exists());
    }
    #[test]
    fn stale_owned_exports_are_retired_without_recursive_or_unrelated_deletion() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("private");
        let mut artifact = WindowsDragArtifact::create_in(&root).unwrap();
        let original = artifact.path().to_owned();
        artifact.write(b"SYNTHETIC", "cover-letter.docx").unwrap();
        let unrelated = root.join("unrelated");
        fs::create_dir(&unrelated).unwrap();
        artifact.preserve = true;
        drop(artifact); // Simulate a dead owner with retained files.
        sweep_in(
            &root,
            SystemTime::now() + RETENTION + Duration::from_secs(1),
        );
        assert!(!original.exists());
        assert!(unrelated.exists());
    }
    #[test]
    fn cleanup_preserves_hard_linked_files_and_unknown_contents() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("private");
        let artifact = WindowsDragArtifact::create_in(&root).unwrap();
        let path = artifact.write(b"SYNTHETIC", "tailored-resume.pdf").unwrap();
        let outside = temp.path().join("outside.pdf");
        fs::hard_link(&path, &outside).unwrap();
        drop(artifact);
        assert_eq!(fs::read(&outside).unwrap(), b"SYNTHETIC");
        assert!(path.exists());
        let artifact = WindowsDragArtifact::create_in(&root).unwrap();
        let unexpected = artifact.path().join("unrelated.txt");
        fs::write(&unexpected, b"PRESERVE").unwrap();
        drop(artifact);
        assert_eq!(fs::read(unexpected).unwrap(), b"PRESERVE");
    }
}
