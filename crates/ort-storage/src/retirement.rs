//! Crash-resumable provider cleanup shared by every permanent profile retirement.
//! The inventory contains only scoped IDs, and is durable before any key is removed.
use crate::{
    AI_CONNECTION_SETTING_KEY, DatabaseKeyVault, Deserialize, File, OpenOptions, Path,
    ProfileManifest, Read, Serialize, StorageError, Uuid, Value, Write, fs, known_optional_file,
    open_existing_profile, reject_symlink, remove_exact_optional_file,
    set_private_file_permissions, sync_directory,
};
use ort_vault::{ProviderCredentialReference, ProviderCredentialVault};

pub(super) const INVENTORY: &str = ".ort-provider-cleanup.json";
pub(super) const INVENTORY_TMP: &str = ".ort-provider-cleanup.tmp";
const MAX_INVENTORY_BYTES: u64 = 64 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Credential {
    provider: String,
    credential_id: Uuid,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inventory {
    version: u8,
    install_id: Uuid,
    profile_id: Uuid,
    credentials: Vec<Credential>,
}
impl Inventory {
    fn references(
        &self,
        manifest: &ProfileManifest,
    ) -> Result<Vec<ProviderCredentialReference>, StorageError> {
        if self.version != 1
            || self.install_id != manifest.install_id
            || self.profile_id != manifest.profile_id
        {
            return Err(StorageError::InvalidData);
        }
        self.credentials
            .iter()
            .map(|credential| {
                if !matches!(
                    credential.provider.as_str(),
                    "openai" | "anthropic" | "gemini"
                ) {
                    return Err(StorageError::InvalidData);
                }
                ProviderCredentialReference::new(
                    &manifest.channel,
                    &manifest.install_id.to_string(),
                    &manifest.profile_id.to_string(),
                    &credential.provider,
                    &credential.credential_id.to_string(),
                )
                .map_err(|_| StorageError::InvalidData)
            })
            .collect()
    }
}

// Read only the address fields. Retiring a legacy profile must not migrate its
// settings or depend on the current AI model catalog.
fn credentials_from_setting(value: Value) -> Result<Vec<Credential>, StorageError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct StoredCredential {
        provider: String,
        credential_id: Uuid,
    }
    #[derive(Deserialize)]
    struct Roster {
        keys: Vec<StoredCredential>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Legacy {
        mode: String,
        provider: Option<String>,
        credential_id: Option<Uuid>,
    }
    if value.get("keys").is_some() {
        return serde_json::from_value::<Roster>(value)
            .map(|roster| {
                roster
                    .keys
                    .into_iter()
                    .map(|key| Credential {
                        provider: key.provider,
                        credential_id: key.credential_id,
                    })
                    .collect()
            })
            .map_err(|_| StorageError::InvalidData);
    }
    let legacy: Legacy = serde_json::from_value(value).map_err(|_| StorageError::InvalidData)?;
    if !matches!(legacy.mode.as_str(), "direct_api" | "no_ai") {
        return Err(StorageError::InvalidData);
    }
    match (legacy.provider, legacy.credential_id) {
        (Some(provider), Some(credential_id)) => Ok(vec![Credential {
            provider,
            credential_id,
        }]),
        (None, None) => Ok(Vec::new()),
        _ => Err(StorageError::InvalidData),
    }
}

pub(super) fn prepare(
    root: &Path,
    channel: &str,
    vault: &dyn DatabaseKeyVault,
) -> Result<(), StorageError> {
    let path = root.join(INVENTORY);
    if known_optional_file(&path)? {
        return Ok(());
    }
    let store = open_existing_profile(root, channel, vault)?;
    let manifest = store.manifest();
    let credentials = store
        .load_setting(AI_CONNECTION_SETTING_KEY)?
        .map(|setting| credentials_from_setting(setting.value))
        .transpose()?
        .unwrap_or_default();
    let inventory = Inventory {
        version: 1,
        install_id: manifest.install_id,
        profile_id: manifest.profile_id,
        credentials,
    };
    inventory.references(manifest)?;
    let bytes = serde_json::to_vec(&inventory).map_err(|_| StorageError::InvalidData)?;
    if bytes.len() as u64 > MAX_INVENTORY_BYTES {
        return Err(StorageError::InvalidData);
    }
    drop(store);
    let temporary = root.join(INVENTORY_TMP);
    // A crash before rename leaves only an uncommitted inventory; regenerate it.
    remove_exact_optional_file(&temporary)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| StorageError::Unavailable)?;
    set_private_file_permissions(&temporary)?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| StorageError::Unavailable)?;
    fs::rename(&temporary, &path).map_err(|_| StorageError::Unavailable)?;
    sync_directory(root)
}

pub(super) fn remove_credentials<P: ProviderCredentialVault>(
    root: &Path,
    manifest: &ProfileManifest,
    providers: &P,
) -> Result<(), StorageError> {
    let path = root.join(INVENTORY);
    reject_symlink(&path)?;
    let file = File::open(path).map_err(|_| StorageError::Unavailable)?;
    let metadata = file.metadata().map_err(|_| StorageError::Unavailable)?;
    if !metadata.is_file() || metadata.len() > MAX_INVENTORY_BYTES {
        return Err(StorageError::InvalidData);
    }
    let mut bytes = Vec::new();
    file.take(MAX_INVENTORY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StorageError::Unavailable)?;
    if bytes.len() as u64 > MAX_INVENTORY_BYTES {
        return Err(StorageError::InvalidData);
    }
    let inventory: Inventory =
        serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidData)?;
    for reference in inventory.references(manifest)? {
        providers
            .delete(&reference)
            .map_err(|_| StorageError::VaultKeyUnavailable)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AllLocalDataDeletion, BackupPassphrase, DATABASE_FILENAME, DELETE_ALL_MARKER_FILENAME,
        EncryptedStore, MANIFEST_FILENAME, RESTORE_SAFETY_DIRECTORY, SAFETY_DELETE_DIRECTORY,
        VaultReference, read_manifest, recover_pending_safety_deletion,
        remove_exact_database_files,
    };
    use ort_vault::testing::{MemoryDatabaseKeyVault, MemoryProviderCredentialVault};
    use ort_vault::{ProviderSecret, VaultError};
    #[cfg(windows)]
    use ort_windows::testing as tempfile;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct FailingProviders {
        inner: MemoryProviderCredentialVault,
        fail: AtomicBool,
    }
    impl ProviderCredentialVault for FailingProviders {
        fn use_secret<T>(
            &self,
            reference: &ProviderCredentialReference,
            operation: impl FnOnce(&ProviderSecret) -> T,
        ) -> Result<T, VaultError> {
            self.inner.use_secret(reference, operation)
        }
        fn store_new(
            &self,
            reference: &ProviderCredentialReference,
            secret: &ProviderSecret,
        ) -> Result<(), VaultError> {
            self.inner.store_new(reference, secret)
        }
        fn delete(&self, reference: &ProviderCredentialReference) -> Result<(), VaultError> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(VaultError::Unavailable);
            }
            self.inner.delete(reference)
        }
    }
    fn providers() -> FailingProviders {
        FailingProviders {
            inner: MemoryProviderCredentialVault::new(),
            fail: AtomicBool::new(false),
        }
    }
    fn save_key(
        store: &EncryptedStore,
        providers: &FailingProviders,
    ) -> ProviderCredentialReference {
        let id = Uuid::now_v7();
        let manifest = store.manifest();
        let reference = ProviderCredentialReference::new(
            &manifest.channel,
            &manifest.install_id.to_string(),
            &manifest.profile_id.to_string(),
            "openai",
            &id.to_string(),
        )
        .unwrap();
        providers
            .store_new(
                &reference,
                &ProviderSecret::from_bytes(b"synthetic-provider-secret".to_vec()).unwrap(),
            )
            .unwrap();
        store.save_setting(AI_CONNECTION_SETTING_KEY, None, &serde_json::json!({"keys":[{"provider":"openai","credentialId":id,"paused":true,"removed":false,"cleanupRequired":true}]})).unwrap();
        reference
    }
    fn restore(
        root: &Path,
        vault: &MemoryDatabaseKeyVault,
        providers: &FailingProviders,
    ) -> (EncryptedStore, ProviderCredentialReference, VaultReference) {
        let old = EncryptedStore::open_or_initialize(root, "test", vault).unwrap();
        let reference = save_key(&old, providers);
        let database_reference = old.manifest().vault_reference().unwrap();
        let passphrase = BackupPassphrase::new("synthetic retirement passphrase".into()).unwrap();
        let bytes = old
            .create_portable_backup(&passphrase, "0.0.0-dev")
            .unwrap();
        old.stage_portable_restore(&bytes, &passphrase, "test", vault)
            .unwrap();
        drop(old);
        let (active, promoted) =
            EncryptedStore::open_or_activate_pending_restore(root, "test", vault, providers)
                .unwrap();
        assert!(promoted);
        assert!(
            active
                .load_setting(AI_CONNECTION_SETTING_KEY)
                .unwrap()
                .is_none()
        );
        assert!(providers.use_secret(&reference, |_| ()).is_ok()); // Rollback still owns it.
        (active, reference, database_reference)
    }
    #[test]
    fn safety_retirement_retries_credentials_before_destroying_their_database_key() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("default");
        let vault = MemoryDatabaseKeyVault::new();
        let providers = providers();
        let (active, old_key, old_database) = restore(&root, &vault, &providers);
        let active_key = save_key(&active, &providers);
        providers.fail.store(true, Ordering::SeqCst);
        assert_eq!(
            active.delete_retained_safety_copy("test", &vault, &providers),
            Err(StorageError::VaultKeyUnavailable)
        );
        assert!(vault.load(&old_database).is_ok());
        assert!(
            temp.path()
                .join(SAFETY_DELETE_DIRECTORY)
                .join(INVENTORY)
                .is_file()
        );
        drop(active);
        providers.fail.store(false, Ordering::SeqCst);
        let (active, _) =
            EncryptedStore::open_or_activate_pending_restore(&root, "test", &vault, &providers)
                .unwrap();
        assert_eq!(
            providers.use_secret(&old_key, |_| ()),
            Err(VaultError::Missing)
        );
        assert_eq!(vault.load(&old_database).err(), Some(VaultError::Missing));
        assert!(providers.use_secret(&active_key, |_| ()).is_ok());
        assert!(
            !active
                .backup_recovery_status()
                .unwrap()
                .safety_cleanup_pending
        );
    }
    #[test]
    fn delete_all_inventories_active_and_retained_profiles_and_retries_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("default");
        let vault = MemoryDatabaseKeyVault::new();
        let providers = providers();
        let (active, old_key, old_database) = restore(&root, &vault, &providers);
        let active_key = save_key(&active, &providers);
        let active_database = active.manifest().vault_reference().unwrap();
        drop(active);
        providers.fail.store(true, Ordering::SeqCst);
        assert_eq!(
            EncryptedStore::delete_all_local_data(&root, "test", &vault, &providers).unwrap(),
            AllLocalDataDeletion::CleanupPending
        );
        for candidate in [&root, &temp.path().join(RESTORE_SAFETY_DIRECTORY)] {
            assert!(candidate.join(INVENTORY).is_file());
        }
        assert!(vault.load(&old_database).is_ok());
        assert!(vault.load(&active_database).is_ok());
        providers.fail.store(false, Ordering::SeqCst);
        let (fresh, _) =
            EncryptedStore::open_or_activate_pending_restore(&root, "test", &vault, &providers)
                .unwrap();
        for key in [&old_key, &active_key] {
            assert_eq!(providers.use_secret(key, |_| ()), Err(VaultError::Missing));
        }
        for key in [&old_database, &active_database] {
            assert_eq!(vault.load(key).err(), Some(VaultError::Missing));
        }
        assert!(
            fresh
                .load_setting(AI_CONNECTION_SETTING_KEY)
                .unwrap()
                .is_none()
        );
        assert!(!temp.path().join(DELETE_ALL_MARKER_FILENAME).exists());
    }
    #[test]
    fn malformed_rosters_preserve_every_profile_key_and_credential() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("default");
        let vault = MemoryDatabaseKeyVault::new();
        let providers = providers();
        let store = EncryptedStore::open_or_initialize(&root, "test", &vault).unwrap();
        let key = save_key(&store, &providers);
        let database_key = store.manifest().vault_reference().unwrap();
        let setting = store
            .load_setting(AI_CONNECTION_SETTING_KEY)
            .unwrap()
            .unwrap();
        store
            .save_setting(
                AI_CONNECTION_SETTING_KEY,
                Some(setting.revision),
                &serde_json::json!({"unexpected":true}),
            )
            .unwrap();
        drop(store);
        assert_eq!(
            EncryptedStore::delete_all_local_data(&root, "test", &vault, &providers).unwrap(),
            AllLocalDataDeletion::CleanupPending
        );
        assert!(vault.load(&database_key).is_ok());
        assert!(providers.use_secret(&key, |_| ()).is_ok());
        assert!(root.join(DATABASE_FILENAME).is_file());
        assert!(!root.join(INVENTORY).exists());
    }

    #[test]
    fn completed_cleanup_can_resume_after_manifest_removal() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(SAFETY_DELETE_DIRECTORY);
        let vault = MemoryDatabaseKeyVault::new();
        let providers = providers();
        let store = EncryptedStore::open_or_initialize(&root, "test", &vault).unwrap();
        let key = save_key(&store, &providers);
        let reference = store.manifest().vault_reference().unwrap();
        drop(store);
        prepare(&root, "test", &vault).unwrap();
        let manifest = read_manifest(&root.join(MANIFEST_FILENAME), "test").unwrap();
        remove_credentials(&root, &manifest, &providers).unwrap();
        vault.delete(&reference).unwrap();
        remove_exact_database_files(&root.join(DATABASE_FILENAME)).unwrap();
        fs::remove_file(root.join(MANIFEST_FILENAME)).unwrap();
        recover_pending_safety_deletion(temp.path(), "test", &vault, &providers).unwrap();
        assert!(!root.exists());
        assert_eq!(providers.use_secret(&key, |_| ()), Err(VaultError::Missing));
    }
}
