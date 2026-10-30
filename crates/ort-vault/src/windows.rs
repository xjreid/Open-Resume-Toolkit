//! Explicit device-local credentials; no global default-store mutation.
use std::collections::HashMap;

use keyring_core::api::CredentialStoreApi;

use crate::{VaultError, map_read_error};

pub(super) fn entry(service: &str, account: &str) -> Result<keyring::Entry, VaultError> {
    let store = windows_native_keyring_store::Store::new().map_err(|_| VaultError::Unavailable)?;
    // Keep the existing user.service target mapping, so upgrading persistence
    // never creates a second address or loses access to an existing key.
    let modifiers = HashMap::from([("persistence", "Local")]);
    let inner = store
        .build(service, account, Some(&modifiers))
        .map_err(|_| VaultError::Unavailable)?;
    Ok(keyring::Entry { inner })
}

/// Called only while holding the native current-user vault mutex.
/// Existing Enterprise/Session entries are rewritten with the SAME secret and
/// metadata using the local specifier, never deleted/replaced with a new key.
pub(super) fn ensure_local(entry: &keyring::Entry) -> Result<(), VaultError> {
    let attributes = entry
        .inner
        .get_attributes()
        .map_err(|e| map_read_error(&e))?;
    if attributes.get("persistence").map(String::as_str) != Some("Local") {
        // An empty attribute update preserves the existing username, alias,
        // comment, and secret; the backend applies the specifier's persistence.
        entry
            .inner
            .update_attributes(&HashMap::new())
            .map_err(|_| VaultError::Unavailable)?;
        let updated = entry
            .inner
            .get_attributes()
            .map_err(|_| VaultError::Unavailable)?;
        if updated.get("persistence").map(String::as_str) != Some("Local") {
            return Err(VaultError::Unavailable);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Cleanup(keyring::Entry);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete_credential();
        }
    }

    #[test]
    fn local_specifier_keeps_the_legacy_address_without_accessing_the_vault() {
        let entry = entry(
            "com.openresumetoolkit.dev.database",
            "install-one.profile-two",
        )
        .unwrap();
        let debug = format!("{:?}", entry.inner);
        assert!(debug.contains("Local"));
        assert!(debug.contains("install-one.profile-two.com.openresumetoolkit.dev.database"));
        assert!(!debug.contains("Enterprise"));
    }

    #[test]
    #[ignore = "creates one disposable synthetic Windows credential; explicit native opt-in required"]
    fn existing_enterprise_secret_becomes_local_without_changing_bytes() {
        assert_eq!(std::env::var("ORT_RUN_OS_VAULT_TESTS").as_deref(), Ok("1"));
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).unwrap();
        let account = hex::encode(random);
        let service = "com.openresumetoolkit.platform-test.persistence";
        let _guard = crate::native_lock().unwrap();
        let legacy = keyring::Entry::new(service, &account).unwrap();
        let cleanup = Cleanup(legacy);
        cleanup
            .0
            .set_secret(b"SYNTHETIC-NOT-A-REAL-SECRET")
            .unwrap();
        assert_eq!(
            cleanup.0.inner.get_attributes().unwrap()["persistence"],
            "Enterprise"
        );
        let local = entry(service, &account).unwrap();
        ensure_local(&local).unwrap();
        assert_eq!(
            local.inner.get_attributes().unwrap()["persistence"],
            "Local"
        );
        let bytes = zeroize::Zeroizing::new(local.get_secret().unwrap());
        assert_eq!(bytes.as_slice(), b"SYNTHETIC-NOT-A-REAL-SECRET");
        // Repeating the policy check is idempotent and preserves the same key.
        ensure_local(&local).unwrap();
    }
}
