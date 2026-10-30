//! Disposable private fixtures for consumers testing the Windows storage boundary.
//! The parent remains owned by tempfile; no fixture touches an application profile.
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct TempDir {
    temporary: tempfile::TempDir,
    path: PathBuf,
}

impl TempDir {
    /// Creates a private fixture without changing the ACL of a pre-existing root.
    /// # Errors
    /// Returns filesystem/security failures; never relaxes production privacy checks.
    pub fn new() -> io::Result<Self> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("private");
        crate::PrivateDirectory::create(&path)?;
        Ok(Self { temporary, path })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// # Errors
    /// Returns an error if fixture cleanup cannot complete.
    pub fn close(self) -> io::Result<()> {
        self.temporary.close()
    }
}

/// # Errors
/// Returns an error when a private temporary fixture cannot be created.
pub fn tempdir() -> io::Result<TempDir> {
    TempDir::new()
}
