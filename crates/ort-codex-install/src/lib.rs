//! Narrow, offline installation boundary. The elevated executable only copies
//! one checksum-pinned regular file to one fixed, protected destination. It
//! never executes the downloaded runtime, launches a shell, or uses the network.
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const DESTINATION: &str = "/Library/Application Support/Open Resume Toolkit/Codex/codex";
pub type Result<T> = std::result::Result<T, &'static str>;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub version: String,
    pub executable_sha256: String,
    pub executable_bytes: u64,
    pub archive_sha256: String,
    pub archive_bytes: u64,
    pub download_url: String,
    pub archive_entry: String,
}
#[must_use]
/// # Panics
/// Panics only if the compatibility manifest embedded at build time is invalid.
pub fn manifest() -> &'static Manifest {
    static VALUE: std::sync::OnceLock<Manifest> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../apps/desktop/src-tauri/codex-compatibility.json"
        ))
        .expect("qualified runtime manifest")
    })
}

/// Bound reads even if the file changes after metadata inspection. All callers
/// receive fixed error codes; neither file content nor paths enter diagnostics.
/// # Errors
/// Rejects incorrect size, digest, executable headers, or failed reads/writes.
pub fn check_bytes(
    reader: &mut impl Read,
    writer: &mut impl Write,
    length: u64,
    digest: &str,
    macho: bool,
) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024];
    if macho {
        let mut header = [0_u8; 16];
        reader
            .read_exact(&mut header)
            .map_err(|_| "PLAN_INSTALL_VERIFY_FAILED")?;
        if header[..8] != [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1]
            || header[12..16] != [2, 0, 0, 0]
            || length < 16
        {
            return Err("PLAN_INSTALL_VERIFY_FAILED");
        }
        hasher.update(header);
        writer
            .write_all(&header)
            .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
        total = 16;
    }
    loop {
        let remaining = usize::try_from((length + 1 - total).min(buffer.len() as u64))
            .map_err(|_| "PLAN_INSTALL_VERIFY_FAILED")?;
        let count = reader
            .read(&mut buffer[..remaining])
            .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > length {
            return Err("PLAN_INSTALL_VERIFY_FAILED");
        }
        hasher.update(&buffer[..count]);
        writer
            .write_all(&buffer[..count])
            .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    }
    if total != length || hex::encode(hasher.finalize()) != digest {
        return Err("PLAN_INSTALL_VERIFY_FAILED");
    }
    Ok(())
}

/// # Errors
/// Rejects links, special files, unexpected lengths, and failed opens.
pub fn open_payload(path: &Path, expected_length: u64) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(
        i32::try_from((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits())
            .map_err(|_| "PLAN_INSTALL_UNSAFE_PATH")?,
    );
    let file = options.open(path).map_err(|_| "PLAN_INSTALL_UNSAFE_PATH")?;
    let metadata = file.metadata().map_err(|_| "PLAN_INSTALL_UNSAFE_PATH")?;
    if !metadata.is_file() || metadata.len() != expected_length {
        return Err("PLAN_INSTALL_VERIFY_FAILED");
    }
    #[cfg(unix)]
    if metadata.nlink() != 1 {
        return Err("PLAN_INSTALL_UNSAFE_PATH");
    }
    Ok(file)
}

/// # Errors
/// Rejects any payload that does not match the qualified executable.
pub fn verify_payload(path: &Path) -> Result<()> {
    let spec = manifest();
    check_bytes(
        &mut open_payload(path, spec.executable_bytes)?,
        &mut std::io::sink(),
        spec.executable_bytes,
        &spec.executable_sha256,
        true,
    )
}

/// Parent directories are checked from the filesystem root down. Once checked,
/// an unprivileged process cannot swap them; symlinks are never repaired/followed.
#[cfg(unix)]
fn protected_directory(path: &Path, owner: u32, anchor: &Path) -> Result<()> {
    if !path.starts_with(anchor) {
        return Err("PLAN_INSTALL_UNSAFE_PATH");
    }
    let mut parents: Vec<_> = path
        .ancestors()
        .take_while(|p| p.starts_with(anchor))
        .collect();
    parents.reverse();
    for parent in parents {
        match fs::symlink_metadata(parent) {
            Ok(meta) => {
                if !meta.is_dir()
                    || meta.file_type().is_symlink()
                    || meta.uid() != owner
                    || meta.mode() & 0o022 != 0
                {
                    return Err("PLAN_INSTALL_UNSAFE_PATH");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::DirBuilder::new()
                    .mode(0o755)
                    .create(parent)
                    .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
                let meta = fs::symlink_metadata(parent).map_err(|_| "PLAN_INSTALL_UNSAFE_PATH")?;
                if !meta.is_dir() || meta.uid() != owner || meta.mode() & 0o022 != 0 {
                    return Err("PLAN_INSTALL_UNSAFE_PATH");
                }
            }
            Err(_) => return Err("PLAN_INSTALL_UNSAFE_PATH"),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn install_at(
    source: &Path,
    destination: &Path,
    spec: &Manifest,
    owner: u32,
    anchor: &Path,
) -> Result<()> {
    let mut input = open_payload(source, spec.executable_bytes)?;
    let parent = destination.parent().ok_or("PLAN_INSTALL_UNSAFE_PATH")?;
    protected_directory(parent, owner, anchor)?;
    match fs::symlink_metadata(destination) {
        Ok(meta)
            if !meta.is_file()
                || meta.file_type().is_symlink()
                || meta.uid() != owner
                || meta.nlink() != 1
                || meta.mode() & 0o022 != 0 =>
        {
            return Err("PLAN_INSTALL_UNSAFE_PATH");
        }
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => return Err("PLAN_INSTALL_UNSAFE_PATH"),
    }
    let mut staged =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    // Copy from the already-open descriptor, then verify the bytes we will
    // publish. No root operation trusts an earlier unprivileged verification.
    check_bytes(
        &mut input,
        staged.as_file_mut(),
        spec.executable_bytes,
        &spec.executable_sha256,
        true,
    )?;
    staged
        .as_file_mut()
        .sync_all()
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    check_bytes(
        &mut open_payload(staged.path(), spec.executable_bytes)?,
        &mut std::io::sink(),
        spec.executable_bytes,
        &spec.executable_sha256,
        true,
    )?;
    staged
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o755))
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    staged
        .as_file()
        .sync_all()
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    staged
        .persist(destination)
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    Ok(())
}

/// There are no destination, version, digest, URL, or command-line overrides.
/// # Errors
/// Rejects unsupported platforms, non-root callers, invalid arguments, unsafe
/// paths, altered payloads, and filesystem failures before atomic publication.
pub fn privileged_main(arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<()> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Err("PLAN_PLATFORM_UNSUPPORTED");
    }
    if !rustix::process::geteuid().is_root() {
        return Err("PLAN_INSTALL_PERMISSION_DENIED");
    }
    let args: Vec<_> = arguments.collect();
    if args.len() != 1 {
        return Err("PLAN_INSTALL_ARGUMENTS_INVALID");
    }
    let source = PathBuf::from(&args[0]);
    if !source.is_absolute() {
        return Err("PLAN_INSTALL_UNSAFE_PATH");
    }
    #[cfg(unix)]
    return install_at(
        &source,
        Path::new(DESTINATION),
        manifest(),
        0,
        Path::new("/"),
    );
    #[cfg(not(unix))]
    Err("PLAN_PLATFORM_UNSUPPORTED")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
