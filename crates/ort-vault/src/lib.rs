//! Narrow boundary around the operating-system credential vault.
//!
//! Database keys are deliberately non-serializable, non-cloneable, redacted in
//! debug output, and exposed only for the duration of an operation.

use std::fmt;
use std::sync::Mutex;

use zeroize::Zeroize;

const DATABASE_KEY_BYTES: usize = 32;
const COMPONENT_MAX_BYTES: usize = 64;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VaultError {
    #[error("the credential reference is invalid")]
    InvalidReference,
    #[error("secure random generation is unavailable")]
    RandomUnavailable,
    #[error("the operating-system credential vault is unavailable")]
    Unavailable,
    #[error("the database key is missing")]
    Missing,
    #[error("a database key already exists")]
    AlreadyExists,
    #[error("the stored database key is invalid")]
    CorruptSecret,
}

/// Stable, non-secret address for one profile's database key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VaultReference {
    service: String,
    account: String,
}

impl VaultReference {
    /// Builds a validated, channel-scoped credential address.
    ///
    /// # Errors
    /// Returns `InvalidReference` when any component is empty, oversized, or
    /// contains characters that could blur the namespace boundary.
    pub fn new(channel: &str, install_id: &str, profile_id: &str) -> Result<Self, VaultError> {
        for value in [channel, install_id, profile_id] {
            validate_component(value)?;
        }

        Ok(Self {
            service: format!("com.openresumetoolkit.{channel}.database"),
            account: format!("install-{install_id}.profile-{profile_id}"),
        })
    }

    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    #[must_use]
    pub fn account(&self) -> &str {
        &self.account
    }
}

fn validate_component(value: &str) -> Result<(), VaultError> {
    let valid = !value.is_empty()
        && value.len() <= COMPONENT_MAX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');

    if valid {
        Ok(())
    } else {
        Err(VaultError::InvalidReference)
    }
}

/// A 256-bit database key that clears its memory on drop.
pub struct DatabaseKey([u8; DATABASE_KEY_BYTES]);

impl DatabaseKey {
    /// Generates a 256-bit key from the operating system random source.
    ///
    /// # Errors
    /// Returns `RandomUnavailable` if the operating system cannot supply bytes.
    pub fn generate() -> Result<Self, VaultError> {
        let mut bytes = [0_u8; DATABASE_KEY_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| VaultError::RandomUnavailable)?;
        Ok(Self(bytes))
    }

    /// Takes ownership of exactly 32 key bytes and clears the input allocation.
    ///
    /// # Errors
    /// Returns `CorruptSecret` when the supplied key is not 256 bits.
    pub fn from_bytes(mut bytes: Vec<u8>) -> Result<Self, VaultError> {
        if bytes.len() != DATABASE_KEY_BYTES {
            bytes.zeroize();
            return Err(VaultError::CorruptSecret);
        }

        let mut key = [0_u8; DATABASE_KEY_BYTES];
        key.copy_from_slice(&bytes);
        bytes.zeroize();
        Ok(Self(key))
    }

    pub fn expose_for<T>(&self, operation: impl FnOnce(&[u8; DATABASE_KEY_BYTES]) -> T) -> T {
        operation(&self.0)
    }
}

impl fmt::Debug for DatabaseKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DatabaseKey([REDACTED])")
    }
}

impl Drop for DatabaseKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub trait DatabaseKeyVault: Send + Sync {
    /// Loads one database key.
    ///
    /// # Errors
    /// Returns a safe vault error when the item is missing, corrupt, or inaccessible.
    fn load(&self, reference: &VaultReference) -> Result<DatabaseKey, VaultError>;
    /// Stores a key only if the address does not already exist.
    ///
    /// # Errors
    /// Returns `AlreadyExists` instead of overwriting an existing key.
    fn store_new(&self, reference: &VaultReference, key: &DatabaseKey) -> Result<(), VaultError>;
    /// Deletes exactly one credential and treats a missing item as already deleted.
    ///
    /// # Errors
    /// Returns `Unavailable` when the operating-system store cannot be accessed.
    fn delete(&self, reference: &VaultReference) -> Result<(), VaultError>;
}

/// macOS Keychain / Windows Credential Manager implementation.
pub struct OsDatabaseKeyVault {
    operations: Mutex<()>,
}

impl Default for OsDatabaseKeyVault {
    fn default() -> Self {
        Self {
            operations: Mutex::new(()),
        }
    }
}

impl OsDatabaseKeyVault {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn entry(reference: &VaultReference) -> Result<keyring::Entry, VaultError> {
        keyring::Entry::new(reference.service(), reference.account())
            .map_err(|_| VaultError::Unavailable)
    }
}

impl DatabaseKeyVault for OsDatabaseKeyVault {
    fn load(&self, reference: &VaultReference) -> Result<DatabaseKey, VaultError> {
        let _guard = self
            .operations
            .lock()
            .map_err(|_| VaultError::Unavailable)?;
        let bytes = Self::entry(reference)?
            .get_secret()
            .map_err(|error| map_read_error(&error))?;
        DatabaseKey::from_bytes(bytes)
    }

    fn store_new(&self, reference: &VaultReference, key: &DatabaseKey) -> Result<(), VaultError> {
        let _guard = self
            .operations
            .lock()
            .map_err(|_| VaultError::Unavailable)?;
        let entry = Self::entry(reference)?;

        match entry.get_secret() {
            Ok(mut existing) => {
                existing.zeroize();
                return Err(VaultError::AlreadyExists);
            }
            Err(keyring::Error::NoEntry) => {}
            Err(_) => return Err(VaultError::Unavailable),
        }

        key.expose_for(|bytes| entry.set_secret(bytes))
            .map_err(|_| VaultError::Unavailable)
    }

    fn delete(&self, reference: &VaultReference) -> Result<(), VaultError> {
        let _guard = self
            .operations
            .lock()
            .map_err(|_| VaultError::Unavailable)?;
        match Self::entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(VaultError::Unavailable),
        }
    }
}

fn map_read_error(error: &keyring::Error) -> VaultError {
    match error {
        keyring::Error::NoEntry => VaultError::Missing,
        keyring::Error::BadEncoding(_) | keyring::Error::TooLong(_, _) => VaultError::CorruptSecret,
        _ => VaultError::Unavailable,
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod testing {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::{DatabaseKey, DatabaseKeyVault, VaultError, VaultReference};

    #[derive(Default)]
    pub struct MemoryDatabaseKeyVault {
        values: Mutex<HashMap<(String, String), Vec<u8>>>,
    }

    impl MemoryDatabaseKeyVault {
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Replaces a key to exercise mismatch handling without touching an OS vault.
        ///
        /// # Errors
        /// Returns `Unavailable` if the in-memory test store is poisoned.
        pub fn replace_for_test(
            &self,
            reference: &VaultReference,
            key: &DatabaseKey,
        ) -> Result<(), VaultError> {
            let mut values = self.values.lock().map_err(|_| VaultError::Unavailable)?;
            let address = (reference.service.clone(), reference.account.clone());
            key.expose_for(|bytes| values.insert(address, bytes.to_vec()));
            Ok(())
        }
    }

    impl DatabaseKeyVault for MemoryDatabaseKeyVault {
        fn load(&self, reference: &VaultReference) -> Result<DatabaseKey, VaultError> {
            let values = self.values.lock().map_err(|_| VaultError::Unavailable)?;
            let bytes = values
                .get(&(reference.service.clone(), reference.account.clone()))
                .cloned()
                .ok_or(VaultError::Missing)?;
            DatabaseKey::from_bytes(bytes)
        }

        fn store_new(
            &self,
            reference: &VaultReference,
            key: &DatabaseKey,
        ) -> Result<(), VaultError> {
            let mut values = self.values.lock().map_err(|_| VaultError::Unavailable)?;
            let address = (reference.service.clone(), reference.account.clone());
            if values.contains_key(&address) {
                return Err(VaultError::AlreadyExists);
            }
            key.expose_for(|bytes| values.insert(address, bytes.to_vec()));
            Ok(())
        }

        fn delete(&self, reference: &VaultReference) -> Result<(), VaultError> {
            let mut values = self.values.lock().map_err(|_| VaultError::Unavailable)?;
            values.remove(&(reference.service.clone(), reference.account.clone()));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DatabaseKey, VaultError, VaultReference};

    #[test]
    fn database_key_debug_is_redacted() {
        let key = DatabaseKey::from_bytes(vec![0xAB; 32]).expect("valid key");
        assert_eq!(format!("{key:?}"), "DatabaseKey([REDACTED])");
    }

    #[test]
    fn credential_components_reject_separators_and_empty_values() {
        assert_eq!(
            VaultReference::new("dev/other", "install", "profile"),
            Err(VaultError::InvalidReference)
        );
        assert_eq!(
            VaultReference::new("dev", "", "profile"),
            Err(VaultError::InvalidReference)
        );
    }

    #[test]
    fn credential_reference_is_namespaced() {
        let reference = VaultReference::new("dev", "one", "two").expect("valid reference");
        assert_eq!(reference.service(), "com.openresumetoolkit.dev.database");
        assert_eq!(reference.account(), "install-one.profile-two");
    }
}
