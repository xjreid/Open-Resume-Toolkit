//! Process ownership lives outside every directory that restore or deletion renames.
use crate::{
    StorageError, create_private_directory, reject_symlink, validate_active_profile_root,
    validate_channel,
};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

/// Exclusive authority for one active profile and its recovery siblings.
/// Keep this owner alive across store replacement and unavailable-storage retries.
pub struct ProfileLifecycle {
    _ownership: File,
    root: PathBuf,
    channel: String,
}
impl ProfileLifecycle {
    /// Acquires an OS lock before opening any database or recovery state.
    /// The lock file must never be removed: unlinking it would split ownership.
    /// # Errors
    /// Refuses unsafe paths, unavailable locks, and an already owned profile.
    pub fn acquire(root: &Path, channel: &str) -> Result<Self, StorageError> {
        validate_active_profile_root(root)?;
        validate_channel(channel)?;
        let parent = root.parent().ok_or(StorageError::UnsafeLocation)?;
        reject_symlink(parent)?;
        create_private_directory(parent)?;
        let lock_path = parent.join(".ort-profile-owner.lock");
        reject_symlink(&lock_path)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let ownership = options
            .open(lock_path)
            .map_err(|_| StorageError::Unavailable)?;
        if !ownership
            .metadata()
            .map_err(|_| StorageError::Unavailable)?
            .is_file()
        {
            return Err(StorageError::UnsafeLocation);
        }
        ownership
            .try_lock()
            .map_err(|_| StorageError::ProfileInUse)?;
        Ok(Self {
            _ownership: ownership,
            root: root.to_path_buf(),
            channel: channel.into(),
        })
    }
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
    #[must_use]
    pub fn channel(&self) -> &str {
        &self.channel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    #[test]
    fn lock_child() {
        let Ok(root) = std::env::var("ORT_TEST_OWNER_ROOT") else {
            return;
        };
        let root = Path::new(&root);
        if std::env::var_os("ORT_TEST_OWNER_PROBE").is_some() {
            assert!(matches!(
                ProfileLifecycle::acquire(root, "test"),
                Err(StorageError::ProfileInUse)
            ));
            return;
        }
        let _owner = ProfileLifecycle::acquire(root, "test").unwrap();
        std::fs::write(root.parent().unwrap().join("ready"), b"ready").unwrap();
        // Parent forcibly kills this process, bypassing Rust destructors.
        loop {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    fn ownership_excludes_other_processes_and_survives_profile_removal() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("default");
        let owner = ProfileLifecycle::acquire(&root, "test").unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::remove_dir(&root).unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "lifecycle::tests::lock_child"])
            .env("ORT_TEST_OWNER_ROOT", &root)
            .env("ORT_TEST_OWNER_PROBE", "1")
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        drop(owner);
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "lifecycle::tests::lock_child"])
            .env("ORT_TEST_OWNER_ROOT", &root)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        while !temp.path().join("ready").exists() {
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(matches!(
            ProfileLifecycle::acquire(&root, "test"),
            Err(StorageError::ProfileInUse)
        ));
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(ProfileLifecycle::acquire(&root, "test").is_ok());
    }
}
