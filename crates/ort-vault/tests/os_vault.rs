use ort_vault::{DatabaseKey, DatabaseKeyVault, OsDatabaseKeyVault, VaultError, VaultReference};

const OPT_IN_ENVIRONMENT: &str = "ORT_RUN_OS_VAULT_TESTS";

#[test]
#[ignore = "creates and deletes a temporary OS credential; run through just test-platform-vault"]
fn native_database_key_round_trip_and_overwrite_denial() {
    assert_eq!(
        std::env::var(OPT_IN_ENVIRONMENT).as_deref(),
        Ok("1"),
        "native vault tests require explicit opt-in"
    );

    let mut random_id = [0_u8; 16];
    getrandom::fill(&mut random_id).expect("OS random source must be available");
    let install_id = hex::encode(random_id);
    let reference = VaultReference::new("platform-test", &install_id, "synthetic")
        .expect("test vault reference");
    let vault = OsDatabaseKeyVault::new();
    let cleanup = CredentialCleanup {
        vault: &vault,
        reference: &reference,
    };
    vault.delete(&reference).expect("remove stale test item");

    let expected = DatabaseKey::generate().expect("generate synthetic database key");
    vault
        .store_new(&reference, &expected)
        .expect("store temporary OS credential");
    let loaded = vault
        .load(&reference)
        .expect("load temporary OS credential");
    let matches = expected.expose_for(|expected_bytes| {
        loaded.expose_for(|loaded_bytes| expected_bytes == loaded_bytes)
    });
    assert!(matches, "the OS vault must return the exact key bytes");

    let replacement = DatabaseKey::generate().expect("generate replacement key");
    assert_eq!(
        vault.store_new(&reference, &replacement),
        Err(VaultError::AlreadyExists),
        "the vault adapter must not overwrite an existing database key"
    );

    vault
        .delete(&reference)
        .expect("delete temporary OS credential");
    assert!(matches!(vault.load(&reference), Err(VaultError::Missing)));
    drop(cleanup);
}

struct CredentialCleanup<'a> {
    vault: &'a OsDatabaseKeyVault,
    reference: &'a VaultReference,
}

impl Drop for CredentialCleanup<'_> {
    fn drop(&mut self) {
        let _ = self.vault.delete(self.reference);
    }
}
