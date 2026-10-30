//! Only the per-user Chrome development host name may be registered here.
use crate::{CredentialLock, PrivateDirectory, reject_reparse_path, secure_file};
use std::{
    io,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, KEY_WOW64_64KEY,
        REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteKeyExW,
        RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    },
};
const KEY: &str = r"Software\Google\Chrome\NativeMessagingHosts\com.openresumetoolkit.dev";
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Development Chrome registration refused",
    )
}
fn key_name() -> Vec<u16> {
    KEY.encode_utf16().chain(Some(0)).collect()
}
fn valid_wide(value: &[u16]) -> bool {
    (2..=32_768).contains(&value.len())
        && value.last() == Some(&0)
        && !value[..value.len() - 1].contains(&0)
}
fn value(key: &Key) -> io::Result<Option<Vec<u16>>> {
    let mut kind = 0;
    let mut length = 0;
    // SAFETY: borrowed registry handle; bounded two-pass default-value query.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            null(),
            null(),
            &raw mut kind,
            null_mut(),
            &raw mut length,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status != ERROR_SUCCESS
        || kind != REG_SZ
        || !(2..=65_536).contains(&length)
        || length % 2 != 0
    {
        return Err(denied());
    }
    let mut bytes = vec![0_u16; length as usize / 2];
    if unsafe {
        RegQueryValueExW(
            key.0,
            null(),
            null(),
            &raw mut kind,
            bytes.as_mut_ptr().cast(),
            &raw mut length,
        )
    } != ERROR_SUCCESS
        || kind != REG_SZ
        || length as usize != bytes.len() * 2
        || bytes.last() != Some(&0)
        || bytes[..bytes.len() - 1].contains(&0)
    {
        return Err(denied());
    }
    Ok(Some(bytes))
}
fn manifest_name(manifest: &Path) -> io::Result<Vec<u16>> {
    if !manifest.is_absolute()
        || manifest.file_name().and_then(|v| v.to_str()) != Some("com.openresumetoolkit.dev.json")
    {
        return Err(denied());
    }
    let mut name: Vec<_> = manifest.as_os_str().encode_wide().collect();
    if name.contains(&0) || name.len() > 32_767 {
        return Err(denied());
    }
    name.push(0);
    Ok(name)
}

/// One owner for the entire manifest/image/registry operation, including readers.
/// The thread-affine guard prevents cooperating setup/removal processes from
/// observing or publishing partial filesystem updates around registry calls.
pub struct DevelopmentChromeRegistration {
    _lock: CredentialLock,
    manifest: PathBuf,
    key: Vec<u16>,
}
impl DevelopmentChromeRegistration {
    /// # Errors
    /// Refuses invalid names, another installation's registration or a lock timeout.
    pub fn acquire(manifest: &Path) -> io::Result<Self> {
        Self::acquire_at(manifest, key_name())
    }
    fn acquire_at(manifest: &Path, key: Vec<u16>) -> io::Result<Self> {
        manifest_name(manifest)?;
        let owner = Self {
            _lock: CredentialLock::chrome_registration()?,
            manifest: manifest.into(),
            key,
        };
        owner.is_registered()?; // Check ownership before the caller mutates any files.
        Ok(owner)
    }
    /// # Errors
    /// A different existing installation is never adopted or replaced.
    pub fn is_registered(&self) -> io::Result<bool> {
        owns_at(&self.key, &manifest_name(&self.manifest)?)
    }
    /// Called only after the complete manifest and its image have been published.
    /// # Errors
    /// Rejects unsafe manifests and registry failures without changing another owner.
    pub fn register(&self) -> io::Result<()> {
        let manifest = &self.manifest;
        let _directory = PrivateDirectory::open(manifest.parent().ok_or_else(denied)?)?;
        let _guards = reject_reparse_path(manifest)?;
        secure_file(manifest)?;
        register_at(&self.key, &manifest_name(manifest)?)
    }
    /// # Errors
    /// Removes only this installation; hold the guard through subsequent file cleanup.
    pub fn unregister(&self) -> io::Result<()> {
        unregister_at(&self.key, &manifest_name(&self.manifest)?)
    }
}

fn owns_at(key_name: &[u16], name: &[u16]) -> io::Result<bool> {
    let Some(key) = open_at(key_name)? else {
        return Ok(false);
    };
    match value(&key)? {
        None => Ok(false),
        Some(prior) if prior == name => Ok(true),
        Some(_) => Err(denied()),
    }
}

fn register_at(key_name: &[u16], name: &[u16]) -> io::Result<()> {
    if !valid_wide(key_name) || !valid_wide(name) {
        return Err(denied());
    }
    let mut raw = null_mut();
    // SAFETY: HKCU only. The public caller supplies the fixed development name;
    // private tests use a disjoint random leaf. No elevation or machine-wide key.
    if unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_name.as_ptr(),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY,
            null(),
            &raw mut raw,
            null_mut(),
        )
    } != ERROR_SUCCESS
    {
        return Err(denied());
    }
    let key = Key(raw);
    if value(&key)?.is_some_and(|prior| prior != name) {
        return Err(denied());
    }
    if unsafe {
        RegSetValueExW(
            key.0,
            null(),
            0,
            REG_SZ,
            name.as_ptr().cast(),
            u32::try_from(name.len() * 2).map_err(|_| denied())?,
        )
    } != ERROR_SUCCESS
    {
        return Err(denied());
    }
    Ok(())
}

fn open_at(key_name: &[u16]) -> io::Result<Option<Key>> {
    if !valid_wide(key_name) {
        return Err(denied());
    }
    let mut raw = null_mut();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_name.as_ptr(),
            0,
            KEY_QUERY_VALUE | KEY_WOW64_64KEY,
            &raw mut raw,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status != ERROR_SUCCESS {
        return Err(denied());
    }
    Ok(Some(Key(raw)))
}
fn unregister_at(key_name: &[u16], name: &[u16]) -> io::Result<()> {
    if !valid_wide(name) {
        return Err(denied());
    }
    let Some(key) = open_at(key_name)? else {
        return Ok(());
    };
    if value(&key)?.as_deref() != Some(name) {
        return Err(denied());
    }
    drop(key);
    // SAFETY: fixed development name; ownership matched the exact manifest above.
    if unsafe { RegDeleteKeyExW(HKEY_CURRENT_USER, key_name.as_ptr(), KEY_WOW64_64KEY, 0) }
        != ERROR_SUCCESS
    {
        return Err(denied());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::{Child, Command, Stdio},
        time::{Duration, Instant},
    };
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }
    struct Cleanup {
        key: Vec<u16>,
        name: Vec<u16>,
    }
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = unregister_at(&self.key, &self.name);
        }
    }
    struct TestChild(Child);
    impl Drop for TestChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    #[test]
    fn registration_child() {
        let Some(root) = std::env::var_os("ORT_TEST_CHROME_SETUP_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let key = wide(&std::env::var("ORT_TEST_CHROME_SETUP_KEY").unwrap());
        std::fs::write(root.join("attempting"), b"SYNTHETIC").unwrap();
        let manifest = root.join("com.openresumetoolkit.dev.json");
        let owner = DevelopmentChromeRegistration::acquire_at(&manifest, key).unwrap();
        assert!(owner.is_registered().unwrap());
        owner.unregister().unwrap();
        std::fs::remove_file(manifest).unwrap();
        std::fs::write(root.join("finished"), b"SYNTHETIC").unwrap();
    }
    #[test]
    fn removal_in_another_process_waits_for_the_complete_publication() {
        let temp = tempfile::tempdir().unwrap();
        let directory = PrivateDirectory::create(&temp.path().join("private")).unwrap();
        let root = directory.path();
        let manifest = root.join("com.openresumetoolkit.dev.json");
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).unwrap();
        let key = format!(
            r"Software\OpenResumeToolkitDevelopmentTest-{}",
            hex::encode(random)
        );
        let cleanup = Cleanup {
            key: wide(&key),
            name: manifest_name(&manifest).unwrap(),
        };
        let owner =
            DevelopmentChromeRegistration::acquire_at(&manifest, cleanup.key.clone()).unwrap();
        std::fs::write(&manifest, b"SYNTHETIC COMPLETE MANIFEST").unwrap();
        owner.register().unwrap();
        let mut child = TestChild(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "registration::tests::registration_child"])
                .env("ORT_TEST_CHROME_SETUP_ROOT", root)
                .env("ORT_TEST_CHROME_SETUP_KEY", &key)
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let end = Instant::now() + Duration::from_secs(2);
        while !root.join("attempting").exists() {
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(100));
        assert!(manifest.is_file());
        assert!(!root.join("finished").exists());
        drop(owner);
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!manifest.exists());
        assert!(root.join("finished").exists());
        assert!(!owns_at(&cleanup.key, &cleanup.name).unwrap());
    }
    #[test]
    fn synthetic_registry_roundtrip_refuses_other_owners_and_preserves_unicode() {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).unwrap();
        let key = wide(&format!(
            r"Software\OpenResumeToolkitDevelopmentTest-{}",
            hex::encode(random)
        ));
        let name = wide(r"C:\Synthetic user\工具\com.openresumetoolkit.dev.json");
        let other = wide(r"C:\Another installation\com.openresumetoolkit.dev.json");
        let cleanup = Cleanup { key, name };
        register_at(&cleanup.key, &cleanup.name).unwrap();
        assert!(owns_at(&cleanup.key, &cleanup.name).unwrap());
        assert!(owns_at(&cleanup.key, &other).is_err());
        assert!(register_at(&cleanup.key, &other).is_err());
        assert!(unregister_at(&cleanup.key, &other).is_err());
        let opened = open_at(&cleanup.key).unwrap().unwrap();
        assert_eq!(value(&opened).unwrap(), Some(cleanup.name.clone()));
        drop(opened);
        unregister_at(&cleanup.key, &cleanup.name).unwrap();
        assert!(open_at(&cleanup.key).unwrap().is_none());
        assert!(!owns_at(&cleanup.key, &cleanup.name).unwrap());
    }
    #[test]
    fn invalid_names_are_refused_before_any_registry_operation() {
        assert!(register_at(&[], &[0]).is_err());
        assert!(unregister_at(&[], &[0]).is_err());
    }
}
