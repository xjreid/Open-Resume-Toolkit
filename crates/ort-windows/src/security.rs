//! Current-user private objects, created with explicit security descriptors.
use std::{
    ffi::{OsStr, c_void},
    fs::File,
    io,
    marker::PhantomData,
    mem::{size_of, zeroed},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Component, Path, PathBuf},
    ptr::{addr_of, null, null_mut},
    rc::Rc,
};
use windows_sys::Win32::{
    Foundation::{
        ERROR_ALREADY_EXISTS, ERROR_INSUFFICIENT_BUFFER, GENERIC_ALL, GetLastError,
        INVALID_HANDLE_VALUE, LocalFree, WAIT_ABANDONED, WAIT_OBJECT_0,
    },
    Security::{
        ACCESS_ALLOWED_ACE, ACL,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetSecurityInfo, SDDL_REVISION_1, SE_FILE_OBJECT, SE_KERNEL_OBJECT, SetSecurityInfo,
        },
        DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetSecurityDescriptorControl,
        GetSecurityDescriptorDacl, GetSecurityDescriptorOwner, GetTokenInformation,
        INHERIT_ONLY_ACE, OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
        TokenUser,
    },
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateDirectoryW, CreateFileW, FILE_ALL_ACCESS,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_SHARE_READ,
        FILE_SHARE_WRITE, GetDriveTypeW, GetFileInformationByHandle, GetFinalPathNameByHandleW,
        GetVolumeInformationByHandleW, OPEN_EXISTING,
    },
    System::Threading::{
        CreateMutexExW, GetCurrentProcess, MUTEX_MODIFY_STATE, OpenProcessToken, ReleaseMutex,
        WaitForSingleObject,
    },
};

const READ_CONTROL: u32 = 0x0002_0000;
const WRITE_DAC: u32 = 0x0004_0000;
const SYNCHRONIZE: u32 = 0x0010_0000;
const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;

fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "private Windows object unavailable",
    )
}

fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut units: Vec<_> = value.encode_wide().collect();
    if units.contains(&0) {
        return Err(denied());
    }
    units.push(0);
    Ok(units)
}

struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: only pointers returned by LocalAlloc-family security APIs enter here.
        unsafe { LocalFree(self.0) };
    }
}

pub(super) fn user_sid_string() -> io::Result<String> {
    let mut token = null_mut();
    // SAFETY: valid pseudo-process handle and writable handle output.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) } == 0 {
        return Err(denied());
    }
    // SAFETY: the successful call transferred an owned token handle.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut required = 0;
    // SAFETY: null buffer/zero size is the documented size query.
    unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            null_mut(),
            0,
            &raw mut required,
        )
    };
    // SAFETY: GetLastError is thread-local and follows the size query immediately.
    if unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER
        || required < u32::try_from(size_of::<TOKEN_USER>()).map_err(|_| denied())?
        || required > 64 * 1024
    {
        return Err(denied());
    }
    let mut buffer = vec![0_usize; (required as usize).div_ceil(size_of::<usize>())];
    // SAFETY: allocation is aligned for TOKEN_USER and contains required writable bytes.
    if unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            required,
            &raw mut required,
        )
    } == 0
    {
        return Err(denied());
    }
    // SAFETY: successful TokenUser query initialized this structure in the live buffer.
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let mut string = null_mut();
    // SAFETY: User.Sid points inside the live token-information buffer.
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &raw mut string) } == 0 {
        return Err(denied());
    }
    let allocation = LocalAllocation(string.cast());
    let mut length = 0;
    // SAFETY: SID strings have a documented small bound; stop at NUL and retain allocation.
    while length < 256 && unsafe { *string.add(length) } != 0 {
        length += 1;
    }
    if length == 256 {
        return Err(denied());
    }
    // SAFETY: the NUL scan proved these length UTF-16 units belong to the allocation.
    let result = String::from_utf16(unsafe { std::slice::from_raw_parts(string, length) })
        .map_err(|_| denied());
    drop(allocation);
    result
}

pub(super) struct PrivateSecurity(LocalAllocation);
impl PrivateSecurity {
    pub(super) fn new() -> io::Result<Self> {
        let user = user_sid_string()?;
        let sddl = format!("O:{user}G:{user}D:P(A;OICI;GA;;;{user})(A;OICI;GA;;;SY)");
        let text = wide(OsStr::new(&sddl))?;
        let mut descriptor = null_mut();
        // SAFETY: NUL-terminated SDDL and valid output; returned memory uses LocalFree.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                SDDL_REVISION_1,
                &raw mut descriptor,
                null_mut(),
            )
        } == 0
        {
            return Err(denied());
        }
        Ok(Self(LocalAllocation(descriptor)))
    }

    pub(super) fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>())
                .expect("Windows structure size"),
            lpSecurityDescriptor: self.0.0,
            bInheritHandle: 0,
        }
    }

    fn owner_and_acl(&self) -> io::Result<(*mut c_void, *mut ACL)> {
        descriptor_parts(self.0.0)
    }

    pub(super) fn verify(&self, handle: &impl AsRawHandle, kind: i32) -> io::Result<()> {
        self.verify_access(handle, kind, true)
    }

    fn verify_access(
        &self,
        handle: &impl AsRawHandle,
        kind: i32,
        require_protected: bool,
    ) -> io::Result<()> {
        let mut owner = null_mut();
        let mut dacl = null_mut();
        let mut descriptor = null_mut();
        // SAFETY: handle remains borrowed; outputs are tied to the allocated descriptor.
        let status = unsafe {
            GetSecurityInfo(
                handle.as_raw_handle(),
                kind,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &raw mut owner,
                null_mut(),
                &raw mut dacl,
                null_mut(),
                &raw mut descriptor,
            )
        };
        if status != 0 {
            return Err(denied());
        }
        let allocation = LocalAllocation(descriptor);
        let (expected_owner, expected_acl) = self.owner_and_acl()?;
        let mut control = 0;
        let mut revision = 0;
        // SAFETY: both descriptors/owner SIDs remain live during the verification.
        if unsafe { GetSecurityDescriptorControl(descriptor, &raw mut control, &raw mut revision) }
            == 0
            || (require_protected && control & SE_DACL_PROTECTED == 0)
            || owner.is_null()
            || dacl.is_null()
            || unsafe { EqualSid(owner, expected_owner) } == 0
        {
            return Err(denied());
        }
        // SAFETY: OS returned valid ACLs. Only the exact user and SYSTEM allow ACEs qualify.
        let count = unsafe { (*dacl).AceCount };
        if count != 2 {
            return Err(denied());
        }
        let mut matched = [false; 2];
        for index in 0..u32::from(count) {
            let actual = allowed_ace(dacl, index)?;
            let mut found = false;
            for candidate in 0..2 {
                let expected = allowed_ace(expected_acl, candidate)?;
                // SAFETY: ACE SID pointers remain inside their live descriptors.
                if unsafe {
                    EqualSid(
                        addr_of!((*actual).SidStart).cast_mut().cast(),
                        addr_of!((*expected).SidStart).cast_mut().cast(),
                    )
                } != 0
                {
                    // Generic rights may be expanded by the object manager on creation.
                    let mask = unsafe { (*actual).Mask };
                    let required = if kind == SE_FILE_OBJECT {
                        FILE_ALL_ACCESS
                    } else {
                        0x001f_0001
                    };
                    if mask & GENERIC_ALL == 0 && mask & required != required {
                        return Err(denied());
                    }
                    matched[candidate as usize] = true;
                    found = true;
                }
            }
            if !found {
                return Err(denied());
            }
        }
        drop(allocation);
        if matched == [true, true] {
            Ok(())
        } else {
            Err(denied())
        }
    }
}

fn descriptor_parts(descriptor: PSECURITY_DESCRIPTOR) -> io::Result<(*mut c_void, *mut ACL)> {
    let mut owner = null_mut();
    let mut defaulted = 0;
    let mut present = 0;
    let mut dacl = null_mut();
    // SAFETY: callers hold a security descriptor allocated by Windows throughout use.
    if unsafe { GetSecurityDescriptorOwner(descriptor, &raw mut owner, &raw mut defaulted) } == 0
        || unsafe {
            GetSecurityDescriptorDacl(
                descriptor,
                &raw mut present,
                &raw mut dacl,
                &raw mut defaulted,
            )
        } == 0
        || present == 0
        || dacl.is_null()
        || owner.is_null()
    {
        return Err(denied());
    }
    Ok((owner, dacl))
}

fn allowed_ace(acl: *mut ACL, index: u32) -> io::Result<*const ACCESS_ALLOWED_ACE> {
    let mut ace = null_mut();
    // SAFETY: Windows validates the index against a valid live ACL.
    if unsafe { GetAce(acl, index, &raw mut ace) } == 0 || ace.is_null() {
        return Err(denied());
    }
    let ace = ace.cast::<ACCESS_ALLOWED_ACE>();
    // SAFETY: GetAce returned an ACE header. Verify its kind/size before accessing its SID.
    let header = unsafe { &(*ace).Header };
    if header.AceType != ACCESS_ALLOWED_ACE_TYPE
        || u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0
        || usize::from(header.AceSize) < size_of::<ACCESS_ALLOWED_ACE>()
    {
        return Err(denied());
    }
    Ok(ace)
}

/// Cross-process serialization for ORT vault operations in the current account.
/// Does not claim to defend against malicious software running as the same user.
/// A guard must be released on the acquiring thread, so it is intentionally !Send.
pub struct CredentialLock {
    handle: OwnedHandle,
    _thread: PhantomData<Rc<()>>,
}
impl CredentialLock {
    /// Acquires the current-user vault lock with a five-second deadline.
    /// # Errors
    /// Rejects inaccessible/spoofed objects and lock timeouts.
    pub fn acquire() -> io::Result<Self> {
        Self::acquire_scope("Vault")
    }
    pub(super) fn chrome_registration() -> io::Result<Self> {
        Self::acquire_scope("ChromeDevelopment")
    }
    fn acquire_scope(scope: &'static str) -> io::Result<Self> {
        let security = PrivateSecurity::new()?;
        // Global named mutexes (unlike global file mappings) do not require
        // SeCreateGlobalPrivilege. Include the SID and verify the object ACL so
        // concurrent logon sessions of the same account share one lock.
        let name = wide(OsStr::new(&format!(
            "Global\\OpenResumeToolkit.{scope}.v1.{}",
            user_sid_string()?
        )))?;
        let attributes = security.attributes();
        // SAFETY: borrowed descriptor/name live through creation; handle is not inherited.
        let raw = unsafe {
            CreateMutexExW(
                &raw const attributes,
                name.as_ptr(),
                0,
                SYNCHRONIZE | MUTEX_MODIFY_STATE | READ_CONTROL,
            )
        };
        if raw.is_null() {
            return Err(denied());
        }
        // SAFETY: successful CreateMutexExW returned a new owned handle.
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        security.verify(&handle, SE_KERNEL_OBJECT)?;
        // SAFETY: valid owned mutex. An abandoned owner requires re-reading vault state,
        // which every operation does after acquiring this guard.
        if !matches!(
            unsafe { WaitForSingleObject(handle.as_raw_handle(), 5_000) },
            WAIT_OBJECT_0 | WAIT_ABANDONED
        ) {
            return Err(denied());
        }
        Ok(Self {
            handle,
            _thread: PhantomData,
        })
    }
}
impl Drop for CredentialLock {
    fn drop(&mut self) {
        // SAFETY: !Send ensures this thread owns the acquired mutex until drop.
        unsafe { ReleaseMutex(self.handle.as_raw_handle()) };
    }
}

fn open_checked(path: &Path, write_acl: bool) -> io::Result<File> {
    open_checked_on_volume(path, write_acl, true)
}

fn open_checked_on_volume(path: &Path, write_acl: bool, require_ntfs: bool) -> io::Result<File> {
    let text = wide(path.as_os_str())?;
    // SAFETY: absolute NUL-terminated path, no inherited handle, no final reparse follow.
    let raw = unsafe {
        CreateFileW(
            text.as_ptr(),
            // Attribute-only handles do not establish the desired sharing
            // barrier. FILE_READ_DATA is FILE_LIST_DIRECTORY for directories.
            FILE_READ_DATA
                | FILE_READ_ATTRIBUTES
                | READ_CONTROL
                | if write_acl { WRITE_DAC } else { 0 },
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful CreateFileW transfers one file handle.
    let file = unsafe { File::from_raw_handle(raw) };
    // SAFETY: zero is valid initialization for the output structure.
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    // SAFETY: borrowed live file handle and writable output.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &raw mut information) } == 0
        || information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
            && information.nNumberOfLinks != 1)
    {
        return Err(denied());
    }
    let mut filesystem = [0_u16; 16];
    // SAFETY: only filesystem-name output is requested and its size matches the allocation.
    if require_ntfs
        && (unsafe {
            GetVolumeInformationByHandleW(
                file.as_raw_handle(),
                null_mut(),
                0,
                null_mut(),
                null_mut(),
                null_mut(),
                filesystem.as_mut_ptr(),
                u32::try_from(filesystem.len()).map_err(|_| denied())?,
            )
        } == 0
            || filesystem[..5] != [78, 84, 70, 83, 0])
    {
        return Err(denied());
    }
    Ok(file)
}

/// Holds every existing ancestor against deletion/rename while a native path is used.
/// Only local drive paths on NTFS qualify; UNC/device paths and alternate streams do not.
/// # Errors
/// Rejects reparse points, ambiguous paths, hard-linked files, and unsupported volumes.
pub fn reject_reparse_path(path: &Path) -> io::Result<Vec<File>> {
    guard_path(path, true)
}

/// Pins a native-dialog source for a bounded read. Unlike managed application
/// data, an existing source may live on a removable/non-NTFS drive.
/// # Errors
/// Rejects remote/device paths, reparse redirection, ambiguous names, and hard links.
pub fn selected_source_handles(path: &Path) -> io::Result<Vec<File>> {
    guard_path(path, false)
}

fn guard_path(path: &Path, private_volume: bool) -> io::Result<Vec<File>> {
    let components: Vec<_> = path.components().collect();
    if components.len() > 128 {
        return Err(denied());
    }
    let Some(Component::Prefix(prefix)) = components.first() else {
        return Err(denied());
    };
    if !matches!(
        prefix.kind(),
        std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
    ) || !path.is_absolute()
        || components.iter().any(|part| match part {
            Component::ParentDir | Component::CurDir => true,
            Component::Normal(name) => {
                name.to_string_lossy().contains([':', '\0'])
                    || name.to_string_lossy().ends_with([' ', '.'])
            }
            _ => false,
        })
    {
        return Err(denied());
    }
    let root = path.ancestors().last().ok_or_else(denied)?;
    let drive = wide(root.as_os_str())?;
    // SAFETY: NUL-terminated drive root. Reject mapped network drives independently
    // of a remote server's filesystem name. Only selected sources permit removable drives.
    let drive_type = unsafe { GetDriveTypeW(drive.as_ptr()) };
    if drive_type != 3 && (private_volume || drive_type != 2) {
        return Err(denied());
    }
    let mut ancestors: Vec<_> = path.ancestors().collect();
    ancestors.reverse();
    let mut guards = Vec::new();
    for ancestor in ancestors {
        match open_checked_on_volume(ancestor, false, private_volume) {
            Ok(file) => guards.push(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(error) => return Err(error),
        }
    }
    Ok(guards)
}

/// A verified private NTFS directory; its ancestors cannot be renamed while held.
pub struct PrivateDirectory {
    _ancestors: Vec<File>,
    path: PathBuf,
}
impl PrivateDirectory {
    /// Opens an existing private directory without creating or repairing it.
    /// # Errors
    /// Rejects missing, redirected, unowned, or broadly accessible roots.
    pub fn open(path: &Path) -> io::Result<Self> {
        let guards = reject_reparse_path(path)?;
        let file = open_checked(path, false)?;
        if !file.metadata()?.is_dir() {
            return Err(denied());
        }
        PrivateSecurity::new()?.verify(&file, SE_FILE_OBJECT)?;
        Ok(Self {
            _ancestors: guards,
            path: path.to_owned(),
        })
    }

    /// Creates missing directories with a private DACL before any content is written.
    /// Existing directories must already satisfy the same policy; never repair broad roots.
    /// # Errors
    /// Refuses unowned, redirected, shared, or unsupported filesystem locations.
    pub fn create(path: &Path) -> io::Result<Self> {
        let _guards = reject_reparse_path(path)?;
        let security = PrivateSecurity::new()?;
        if path.try_exists()? {
            return Self::open(path);
        }
        let parent = path.parent().ok_or_else(denied)?;
        let _parent = if parent.try_exists()? {
            None
        } else {
            Some(Self::create(parent)?)
        };
        let text = wide(path.as_os_str())?;
        let attributes = security.attributes();
        // SAFETY: complete private descriptor applies atomically at directory creation.
        if unsafe { CreateDirectoryW(text.as_ptr(), &raw const attributes) } == 0
            && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS
        {
            return Err(denied());
        }
        let guards = reject_reparse_path(path)?;
        let file = open_checked(path, false)?;
        if !file.metadata()?.is_dir() {
            return Err(denied());
        }
        security.verify(&file, SE_FILE_OBJECT)?;
        Ok(Self {
            _ancestors: guards,
            path: path.to_owned(),
        })
    }

    /// Creates one private child of an already held directory capability.
    /// # Errors
    /// Rejects path-bearing names, unknown handles, redirects, and unsafe volumes.
    pub fn create_child(parent: &impl AsRawHandle, name: &OsStr) -> io::Result<Self> {
        let child = Path::new(name);
        if child.components().count() != 1
            || !matches!(child.components().next(), Some(Component::Normal(_)))
        {
            return Err(denied());
        }
        let mut buffer = vec![0_u16; 32_768];
        // SAFETY: borrowed directory handle and appropriately sized writable UTF-16 buffer.
        let length = unsafe {
            GetFinalPathNameByHandleW(parent.as_raw_handle(), buffer.as_mut_ptr(), 32_768, 0)
        };
        if length == 0 || length >= 32_768 {
            return Err(denied());
        }
        let parent = std::ffi::OsString::from_wide(&buffer[..length as usize]);
        let path = PathBuf::from(parent).join(name);
        let _guards = reject_reparse_path(&path)?;
        let security = PrivateSecurity::new()?;
        let attributes = security.attributes();
        let text = wide(path.as_os_str())?;
        // SAFETY: private security is applied atomically; unlike create(),
        // this operation must never adopt or clean an existing stage.
        if unsafe { CreateDirectoryW(text.as_ptr(), &raw const attributes) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Self::create(&path)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Protects one application-managed file and checks the DACL using its open handle.
/// New sensitive files must be created inside a `PrivateDirectory` so inheritance
/// already restricts access before this explicit protected DACL is applied.
/// # Errors
/// Rejects redirects/hard links, unsafe parents, or failed ACL verification.
pub fn secure_file(path: &Path) -> io::Result<()> {
    let _parent = PrivateDirectory::create(path.parent().ok_or_else(denied)?)?;
    let _guards = reject_reparse_path(path)?;
    let security = PrivateSecurity::new()?;
    let file = open_checked(path, true)?;
    if !file.metadata()?.is_file() {
        return Err(denied());
    }
    // Inherited user/SYSTEM-only ACLs are safe at creation. Reject broadened
    // existing files instead of "repairing" a possible confidentiality breach.
    security.verify_access(&file, SE_FILE_OBJECT, false)?;
    let (_, dacl) = security.owner_and_acl()?;
    // SAFETY: file remains pinned; a private DACL is installed, ownership is unchanged.
    if unsafe {
        SetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            dacl,
            null(),
        )
    } != 0
    {
        return Err(denied());
    }
    security.verify(&file, SE_FILE_OBJECT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::mpsc, thread, time::Duration};

    #[test]
    fn private_directory_pins_ancestors_and_protects_inherited_files() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("private");
        let directory = PrivateDirectory::create(&root).unwrap();
        let file = root.join("synthetic.txt");
        fs::write(&file, b"SYNTHETIC").unwrap();
        secure_file(&file).unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"SYNTHETIC");
        // Directory replacement cannot redirect subsequent pathname operations
        // while the capability is held, even by another cooperating process.
        assert!(fs::rename(&root, temporary.path().join("moved")).is_err());
        drop(directory);
        fs::rename(&root, temporary.path().join("moved")).unwrap();
    }

    #[test]
    fn ambiguous_and_remote_paths_are_rejected_before_creation() {
        for value in [
            r"relative\profile",
            r"\\server\share\profile",
            r"\\.\C:\profile",
            r"C:\profile\..\other",
            r"C:\profile\file:stream",
            r"C:\profile\trailing. leading\bad.",
        ] {
            assert!(reject_reparse_path(Path::new(value)).is_err(), "{value}");
        }
    }

    #[test]
    fn existing_broad_root_is_refused_without_changing_permissions() {
        let temporary = tempfile::tempdir().unwrap();
        assert!(PrivateDirectory::create(temporary.path()).is_err());
        assert!(temporary.path().is_dir());
    }

    #[test]
    fn hard_linked_private_files_are_refused() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("private");
        let _directory = PrivateDirectory::create(&root).unwrap();
        let source = root.join("source.txt");
        fs::write(&source, b"SYNTHETIC").unwrap();
        fs::hard_link(&source, root.join("alias.txt")).unwrap();
        assert!(secure_file(&source).is_err());
    }

    #[test]
    fn independent_threads_wait_for_the_same_native_vault_owner() {
        let owner = CredentialLock::acquire().unwrap();
        let (send, receive) = mpsc::channel();
        let thread = thread::spawn(move || {
            let _owner = CredentialLock::acquire().unwrap();
            send.send(()).unwrap();
        });
        assert!(receive.recv_timeout(Duration::from_millis(100)).is_err());
        drop(owner);
        receive.recv_timeout(Duration::from_secs(3)).unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn broadened_file_permissions_are_refused_without_repairing_the_file() {
        use std::{os::windows::process::CommandExt, process::Command};
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("private");
        let _directory = PrivateDirectory::create(&root).unwrap();
        let file = root.join("synthetic.txt");
        fs::write(&file, b"SYNTHETIC").unwrap();
        secure_file(&file).unwrap();
        // Windows' own ACL utility provides an independent mutation/negative
        // control. Pass data arguments directly; no shell is involved.
        let output = Command::new("icacls.exe")
            .arg(&file)
            .args(["/grant", "*S-1-1-0:(R)"])
            .creation_flags(0x0800_0000)
            .output()
            .unwrap();
        assert!(output.status.success(), "synthetic ACL mutation failed");
        assert!(secure_file(&file).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"SYNTHETIC");
    }

    #[test]
    fn junctions_are_rejected_even_when_the_target_contains_a_valid_file() {
        use std::{os::windows::process::CommandExt, process::Command};
        let temporary = tempfile::tempdir().unwrap();
        let target = temporary.path().join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("synthetic.txt"), b"SYNTHETIC").unwrap();
        let junction = temporary.path().join("junction");
        // Fixed script; fixture paths are data in the child environment and
        // never interpolated into PowerShell code or shell command strings.
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command",
                "New-Item -ItemType Junction -Path $env:ORT_TEST_JUNCTION_PATH -Target $env:ORT_TEST_JUNCTION_TARGET -ErrorAction Stop | Out-Null"])
            .env("ORT_TEST_JUNCTION_PATH", &junction)
            .env("ORT_TEST_JUNCTION_TARGET", &target)
            .creation_flags(0x0800_0000)
            .output().unwrap();
        assert!(
            output.status.success(),
            "synthetic junction creation failed"
        );
        assert_eq!(
            fs::read(junction.join("synthetic.txt")).unwrap(),
            b"SYNTHETIC"
        );
        assert!(reject_reparse_path(&junction.join("synthetic.txt")).is_err());
        assert!(PrivateDirectory::create(&junction).is_err());
        fs::remove_dir(&junction).unwrap();
        assert_eq!(
            fs::read(target.join("synthetic.txt")).unwrap(),
            b"SYNTHETIC"
        );
    }
}
