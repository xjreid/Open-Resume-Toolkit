//! The Chrome manifest is the single authority for the desktop and browser.
use super::{registration_root, unavailable, valid_id};
use crate::BridgeError;
use ort_windows::{
    DevelopmentChromeRegistration, PrivateDirectory, reject_reparse_path, secure_file,
};
use serde::Deserialize;
use std::{
    fs::OpenOptions,
    io::Read,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostManifest {
    name: String,
    #[serde(rename = "description")]
    _description: String,
    path: PathBuf,
    #[serde(rename = "type")]
    transport: String,
    allowed_origins: Vec<String>,
}

/// # Errors
/// Requires a private, complete, registered development manifest. Readers hold
/// the same operation lock as setup/removal, so partial state cannot enable capture.
pub fn registered_extension_id() -> Result<String, BridgeError> {
    let root = registration_root();
    let path = root.join("com.openresumetoolkit.dev.json");
    let registration = DevelopmentChromeRegistration::acquire(&path).map_err(unavailable)?;
    if !registration.is_registered().map_err(unavailable)? {
        return Err(BridgeError::Unavailable);
    }
    let _directory = PrivateDirectory::open(&root).map_err(unavailable)?;
    let _guards = reject_reparse_path(&path).map_err(unavailable)?;
    secure_file(&path).map_err(unavailable)?;
    let file = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .map_err(unavailable)?;
    let mut bytes = Vec::new();
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(unavailable)?;
    if bytes.len() > 4096 {
        return Err(BridgeError::Malformed);
    }
    manifest_extension_id(&bytes, &root)
}

fn manifest_extension_id(bytes: &[u8], root: &Path) -> Result<String, BridgeError> {
    let value: HostManifest = serde_json::from_slice(bytes).map_err(|_| BridgeError::Malformed)?;
    if value.name != "com.openresumetoolkit.dev"
        || value.transport != "stdio"
        || value.allowed_origins.len() != 1
    {
        return Err(BridgeError::Authentication);
    }
    let id = value.allowed_origins[0]
        .strip_prefix("chrome-extension://")
        .and_then(|s| s.strip_suffix('/'))
        .filter(|id| valid_id(id))
        .ok_or(BridgeError::WrongOrigin)?;
    let hash = value
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_prefix("ort-native-host-dev-"))
        .and_then(|s| s.strip_suffix(".exe"))
        .ok_or(BridgeError::Authentication)?;
    if !value.path.is_absolute()
        || value.path.parent() != Some(root)
        || hash.len() != 64
        || !hash.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(BridgeError::Authentication);
    }
    secure_file(&value.path).map_err(unavailable)?;
    Ok(id.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_manifest_requires_one_origin_and_an_owned_host_image() {
        let temp = ort_windows::testing::tempdir().unwrap();
        let binary = temp
            .path()
            .join(format!("ort-native-host-dev-{}.exe", "0".repeat(64)));
        std::fs::write(&binary, b"SYNTHETIC").unwrap();
        let id = "abcdefghijklmnopabcdefghijklmnop";
        let value = serde_json::json!({"name":"com.openresumetoolkit.dev", "description":"Synthetic",
            "path":binary, "type":"stdio", "allowed_origins":[format!("chrome-extension://{id}/")]});
        let check = |value: &serde_json::Value| {
            manifest_extension_id(&serde_json::to_vec(value).unwrap(), temp.path())
        };
        assert_eq!(check(&value).unwrap(), id);
        for (field, replacement) in [
            ("name", serde_json::json!("com.openresumetoolkit")),
            (
                "allowed_origins",
                serde_json::json!(["chrome-extension://invalid/"]),
            ),
            ("allowed_origins", serde_json::json!([])),
            (
                "allowed_origins",
                serde_json::json!([
                    format!("chrome-extension://{id}/"),
                    format!("chrome-extension://{id}/")
                ]),
            ),
        ] {
            let mut invalid = value.clone();
            invalid[field] = replacement;
            assert!(check(&invalid).is_err());
        }
        std::fs::remove_file(binary).unwrap();
        assert!(check(&value).is_err());
    }
}
