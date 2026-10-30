//! Explicit per-user Chrome development setup. Never reached by native messaging.
use ort_windows::{
    DevelopmentChromeRegistration, PrivateDirectory, reject_reparse_path, secure_file,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Seek, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
const HOST: &str = "com.openresumetoolkit.dev";
const REGISTRATION: &str = "registration.json";
const MANIFEST: &str = "com.openresumetoolkit.dev.json";
const MAX_IMAGE: u64 = 64 * 1024 * 1024;
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Development host setup refused",
    )
}

pub(super) fn command() -> Option<io::Result<()>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let operation = args.first()?.to_str()?;
    if !matches!(
        operation,
        "--register-development" | "--unregister-development"
    ) {
        return None;
    }
    Some(if args.len() == 1 {
        setup(operation == "--register-development")
    } else {
        Err(denied())
    })
}
fn setup(install: bool) -> io::Result<()> {
    let id = option_env!("ORT_DEV_CHROME_EXTENSION_ID").ok_or_else(denied)?;
    if id.len() != 32 || !id.bytes().all(|c| (b'a'..=b'p').contains(&c)) {
        return Err(denied());
    }
    let root = ort_ipc::development::registration_root();
    let manifest = root.join(MANIFEST);
    let registration = DevelopmentChromeRegistration::acquire(&manifest)?;
    let _directory = PrivateDirectory::create(&root)?;
    if !install {
        let binary = registered_binary(&root, &manifest)?;
        registration.unregister()?;
        for name in [REGISTRATION, MANIFEST] {
            let path = root.join(name);
            if path.try_exists()? {
                let guards = reject_reparse_path(&path)?;
                secure_file(&path)?;
                drop(guards);
                fs::remove_file(path)?;
            }
        }
        if let Some(binary) = binary {
            // Windows may keep a running host's image open. The registry and
            // capability metadata have already been removed; never force-kill it.
            let _ = fs::remove_file(binary);
        }
        return Ok(());
    }
    install_host(&root, &std::env::current_exe()?, id, || {
        registration.register()
    })
}

fn install_host(
    root: &Path,
    source: &Path,
    id: &str,
    register: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let manifest = root.join(MANIFEST);
    // One authoritative file supplies both Chrome and the desktop's extension
    // identity. No independently committed registration.json can drift from it.
    let previous = read_manifest(&manifest)?;
    registered_binary(root, &manifest)?; // Validate an existing owned image before mutation.
    let binary = copy_image(source, root)?;
    let manifest_value = serde_json::json!({"name": HOST,
        "description": "Open Resume Toolkit development bridge",
        "path": binary.to_str().ok_or_else(denied)?, "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{id}/")]});
    atomic_json(root, MANIFEST, &manifest_value)?;
    if let Err(error) = register() {
        match previous {
            Some(value) => atomic_json(root, MANIFEST, &value)?,
            None => fs::remove_file(&manifest)?,
        }
        return Err(error);
    }
    Ok(())
}

fn read_manifest(manifest: &Path) -> io::Result<Option<serde_json::Value>> {
    if !manifest.try_exists()? {
        return Ok(None);
    }
    let _guards = reject_reparse_path(manifest)?;
    secure_file(manifest)?;
    let file = OpenOptions::new().read(true).share_mode(1).open(manifest)?;
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(denied());
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| denied())
}
fn registered_binary(root: &Path, manifest: &Path) -> io::Result<Option<PathBuf>> {
    let Some(value) = read_manifest(manifest)? else {
        return Ok(None);
    };
    if value["name"] != HOST {
        return Err(denied());
    }
    let path = PathBuf::from(value["path"].as_str().ok_or_else(denied)?);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(denied)?;
    let hash = name
        .strip_prefix("ort-native-host-dev-")
        .and_then(|s| s.strip_suffix(".exe"))
        .ok_or_else(denied)?;
    if path.parent() != Some(root)
        || hash.len() != 64
        || !hash.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(denied());
    }
    if !path.try_exists()? {
        return Ok(None);
    }
    secure_file(&path)?;
    Ok(Some(path))
}
fn copy_image(source: &Path, root: &Path) -> io::Result<PathBuf> {
    let _guards = reject_reparse_path(source)?;
    let mut file = OpenOptions::new().read(true).share_mode(1).open(source)?;
    let length = file.metadata()?.len();
    if length == 0 || length > MAX_IMAGE {
        return Err(denied());
    }
    let digest = image_digest(&mut file)?;
    let binary = root.join(format!("ort-native-host-dev-{digest}.exe"));
    if binary.try_exists()? {
        let _guards = reject_reparse_path(&binary)?;
        secure_file(&binary)?;
        let mut existing = OpenOptions::new().read(true).share_mode(1).open(&binary)?;
        if existing.metadata()?.len() != length {
            return Err(denied());
        }
        if image_digest(&mut existing)? != digest {
            return Err(denied());
        }
    } else {
        file.rewind()?;
        let temporary = root.join(format!("register-image-{}.tmp", uuid::Uuid::now_v7()));
        let result = (|| {
            let mut destination = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            secure_file(&temporary)?;
            io::copy(&mut file, &mut destination)?;
            destination.sync_all()?;
            drop(destination);
            // Publish only complete bytes, without replacing any existing image.
            fs::hard_link(&temporary, &binary)
        })();
        let cleaned = fs::remove_file(&temporary);
        result?;
        cleaned?;
        secure_file(&binary)?;
    }
    Ok(binary)
}
fn image_digest(file: &mut fs::File) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut buffer = [0; 8192];
    let mut total = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_IMAGE {
            return Err(denied());
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex::encode(hash.finalize()))
}
fn atomic_json(root: &Path, name: &str, value: &serde_json::Value) -> io::Result<()> {
    let path = root.join(name);
    if path.try_exists()? {
        secure_file(&path)?;
    }
    let temporary = root.join(format!("register-{}.tmp", uuid::Uuid::now_v7()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        secure_file(&temporary)?;
        serde_json::to_writer(&mut file, value).map_err(|_| denied())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_failed_registry_commit_restores_the_previous_complete_manifest() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("private");
        let _directory = PrivateDirectory::create(&root).unwrap();
        let source = temporary.path().join("synthetic-host.bin");
        fs::write(&source, b"FIRST SYNTHETIC HOST").unwrap();
        let id = "abcdefghijklmnopabcdefghijklmnop";
        assert!(install_host(&root, &source, id, || Err(denied())).is_err());
        assert!(!root.join(MANIFEST).exists());
        install_host(&root, &source, id, || Ok(())).unwrap();
        let previous = fs::read(root.join(MANIFEST)).unwrap();
        let previous_binary = registered_binary(&root, &root.join(MANIFEST))
            .unwrap()
            .unwrap();
        fs::write(&source, b"SECOND SYNTHETIC HOST").unwrap();
        assert!(
            install_host(&root, &source, "ponmlkjihgfedcbaponmlkjihgfedcba", || Err(
                denied()
            ))
            .is_err()
        );
        assert_eq!(fs::read(root.join(MANIFEST)).unwrap(), previous);
        assert_eq!(fs::read(previous_binary).unwrap(), b"FIRST SYNTHETIC HOST");
        assert!(!root.join(REGISTRATION).exists());
    }
    #[test]
    fn private_host_publication_is_idempotent_and_refuses_a_tampered_existing_image() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("private");
        let _directory = PrivateDirectory::create(&root).unwrap();
        let source = temporary.path().join("synthetic-image.bin");
        fs::write(&source, b"SYNTHETIC-HOST-IMAGE").unwrap();
        let binary = copy_image(&source, &root).unwrap();
        secure_file(&binary).unwrap(); // Includes the single-link/private-ACL postcondition.
        assert_eq!(fs::read(&binary).unwrap(), b"SYNTHETIC-HOST-IMAGE");
        assert_eq!(copy_image(&source, &root).unwrap(), binary);
        let value = serde_json::json!({"mode":"development","synthetic":"工具"});
        atomic_json(&root, REGISTRATION, &value).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &fs::read(root.join(REGISTRATION)).unwrap()
            )
            .unwrap(),
            value
        );
        assert!(!fs::read_dir(&root).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")
        }));
        fs::write(&binary, b"CHANGED").unwrap();
        assert!(copy_image(&source, &root).is_err());
        assert_eq!(fs::read(&binary).unwrap(), b"CHANGED");
    }
}
