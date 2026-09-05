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

#[cfg(target_os = "macos")]
#[test]
#[ignore = "creates/deletes a synthetic Keychain item; explicit native opt-in required"]
fn native_upsert_negative_control_can_replace_a_competing_key() {
    use security_framework::os::macos::keychain::SecKeychain;

    assert_eq!(std::env::var(OPT_IN_ENVIRONMENT).as_deref(), Ok("1"));
    let _no_prompts = SecKeychain::disable_user_interaction().expect("disable test-process UI");
    let mut random_id = [0_u8; 16];
    getrandom::fill(&mut random_id).expect("random test address");
    let reference =
        VaultReference::new("platform-test", &hex::encode(random_id), "upsert-control").unwrap();
    let vault = OsDatabaseKeyVault::new();
    let _cleanup = CredentialCleanup {
        vault: &vault,
        reference: &reference,
    };
    let first = keyring::Entry::new(reference.service(), reference.account()).unwrap();
    let second = keyring::Entry::new(reference.service(), reference.account()).unwrap();
    // Deterministically schedule the old check-then-upsert interleaving. Both
    // callers observe absence before either writes. The later upsert overwrites
    // the first winner, demonstrating why it cannot implement store_new().
    assert!(matches!(first.get_secret(), Err(keyring::Error::NoEntry)));
    assert!(matches!(second.get_secret(), Err(keyring::Error::NoEntry)));
    first.set_secret(&[1; 32]).expect("first synthetic upsert");
    second
        .set_secret(&[2; 32])
        .expect("second synthetic upsert");
    assert!(
        vault
            .load(&reference)
            .unwrap()
            .expose_for(|bytes| bytes == &[2; 32])
    );
    vault.delete(&reference).expect("remove exact control item");
    assert!(matches!(vault.load(&reference), Err(VaultError::Missing)));
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "creates/deletes synthetic Keychain items; explicit native opt-in required"]
fn native_independent_vaults_cannot_replace_concurrent_winner() {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use security_framework::os::macos::keychain::SecKeychain;

    assert_eq!(std::env::var(OPT_IN_ENVIRONMENT).as_deref(), Ok("1"));
    // This is a separate --exact test process. An unavailable/locked Keychain
    // fails the test instead of prompting or changing the user's lock state.
    let _no_prompts = SecKeychain::disable_user_interaction().expect("disable test-process UI");

    for _ in 0..4 {
        let mut random_id = [0_u8; 16];
        getrandom::fill(&mut random_id).expect("random test address");
        let reference = VaultReference::new("platform-test", &hex::encode(random_id), "race")
            .expect("synthetic address");
        let cleanup_vault = OsDatabaseKeyVault::new();
        let _cleanup = CredentialCleanup {
            vault: &cleanup_vault,
            reference: &reference,
        };
        let start = Arc::new(Barrier::new(8));
        let attempts: Vec<_> = (1_u8..=8)
            .map(|candidate| {
                let reference = reference.clone();
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    // Deliberately independent adapters: no shared Rust mutex.
                    let vault = OsDatabaseKeyVault::new();
                    let key = DatabaseKey::from_bytes(vec![candidate; 32]).expect("synthetic key");
                    start.wait();
                    (candidate, vault.store_new(&reference, &key))
                })
            })
            .collect();
        let outcomes: Vec<_> = attempts
            .into_iter()
            .map(|attempt| attempt.join().expect("join all competing writers"))
            .collect();
        let winners: Vec<_> = outcomes
            .iter()
            .filter(|(_, result)| result.is_ok())
            .collect();
        assert_eq!(
            winners.len(),
            1,
            "exactly one exclusive creation must succeed"
        );
        for (_, result) in &outcomes {
            assert!(matches!(result, Ok(()) | Err(VaultError::AlreadyExists)));
        }
        let loaded = cleanup_vault
            .load(&reference)
            .expect("load winning synthetic item");
        assert!(loaded.expose_for(|bytes| bytes.iter().all(|byte| *byte == winners[0].0)));
        cleanup_vault
            .delete(&reference)
            .expect("remove exact test item");
        assert!(matches!(
            cleanup_vault.load(&reference),
            Err(VaultError::Missing)
        ));
    }
}

impl Drop for CredentialCleanup<'_> {
    fn drop(&mut self) {
        let _ = self.vault.delete(self.reference);
    }
}
