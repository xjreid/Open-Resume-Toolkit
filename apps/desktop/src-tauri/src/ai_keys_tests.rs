use super::*;
use ort_vault::{VaultError, testing::MemoryDatabaseKeyVault};
use std::{collections::HashMap, sync::Mutex};
use tempfile::TempDir;

#[derive(Default)]
struct TestVault {
    secrets: Mutex<HashMap<String, Vec<u8>>>,
    fail_store: bool,
    fail_delete: bool,
}
impl ProviderCredentialVault for TestVault {
    fn use_secret<T>(
        &self,
        reference: &ProviderCredentialReference,
        operation: impl FnOnce(&ProviderSecret) -> T,
    ) -> Result<T, VaultError> {
        let bytes = self
            .secrets
            .lock()
            .unwrap()
            .get(reference.account())
            .cloned()
            .ok_or(VaultError::Missing)?;
        Ok(operation(&ProviderSecret::from_bytes(bytes)?))
    }
    fn store_new(
        &self,
        reference: &ProviderCredentialReference,
        secret: &ProviderSecret,
    ) -> Result<(), VaultError> {
        self.secrets.lock().unwrap().insert(
            reference.account().into(),
            secret.expose_for(<[u8]>::to_vec),
        );
        if self.fail_store {
            Err(VaultError::Unavailable)
        } else {
            Ok(())
        }
    }
    fn delete(&self, reference: &ProviderCredentialReference) -> Result<(), VaultError> {
        if self.fail_delete {
            return Err(VaultError::Unavailable);
        }
        self.secrets.lock().unwrap().remove(reference.account());
        Ok(())
    }
}
fn fixture() -> (TempDir, EncryptedStore, TestVault) {
    let temp = tempfile::tempdir().unwrap();
    let store =
        EncryptedStore::open_or_initialize(temp.path(), "test", &MemoryDatabaseKeyVault::new())
            .unwrap();
    (temp, store, TestVault::default())
}
#[test]
fn presets_are_saved_per_key_without_switching_primary_and_unavailable_models_are_rejected() {
    let (_temp, store, vault) = fixture();
    let first = add(&store, &vault, "openai");
    let primary = first.keys[0].credential_id;
    change_key(
        &store,
        &vault,
        ChangeAiKeyRequest {
            credential_id: primary,
            action: AiKeyAction::SelectPrimary,
        },
    )
    .unwrap();
    let second = add(&store, &vault, "anthropic").keys[1].credential_id;
    let (mut registry, revision) = load_registry(&store).unwrap();
    registry.keys[1].preset = "quality".into();
    registry.keys[1].paused = true;
    save_registry(&store, &registry, revision).unwrap();
    let result = set_key_preset(
        &store,
        &SetAiKeyPresetRequest {
            credential_id: second,
            preset: "balanced".into(),
        },
    )
    .unwrap();
    assert_eq!(result.primary_credential_id, Some(primary));
    assert_eq!(result.keys[1].preset, "balanced");
    assert!(result.keys[1].paused);
    assert_eq!(
        set_key_preset(
            &store,
            &SetAiKeyPresetRequest {
                credential_id: second,
                preset: "economy".into()
            }
        )
        .unwrap_err(),
        "AI_PRESET_UNAVAILABLE"
    );
}
#[test]
fn renamed_keys_keep_identity_and_primary_and_accept_default_reset() {
    let (_temp, store, vault) = fixture();
    let registry = add(&store, &vault, "openai");
    let id = registry.keys[0].credential_id;
    change_key(
        &store,
        &vault,
        ChangeAiKeyRequest {
            credential_id: id,
            action: AiKeyAction::SelectPrimary,
        },
    )
    .unwrap();
    let renamed = rename_key(
        &store,
        &RenameAiKeyRequest {
            credential_id: id,
            name: "  Personal  ".into(),
        },
    )
    .unwrap();
    assert_eq!(renamed.keys[0].name.as_deref(), Some("Personal"));
    assert!(
        renamed.keys[0]
            .created_at
            .parse::<jiff::Timestamp>()
            .is_ok()
    );
    assert_eq!(renamed.primary_credential_id, Some(id));
    assert_eq!(renamed.keys[0].provider, "openai");
    assert_eq!(
        load_registry(&store).unwrap().0.keys[0].name.as_deref(),
        Some("Personal")
    );
    for invalid in ["x".repeat(81), "invalid\nname".into()] {
        assert_eq!(
            rename_key(
                &store,
                &RenameAiKeyRequest {
                    credential_id: id,
                    name: invalid
                }
            )
            .unwrap_err(),
            "AI_KEY_NAME_INVALID"
        );
    }
    let reset = rename_key(
        &store,
        &RenameAiKeyRequest {
            credential_id: id,
            name: " ".into(),
        },
    )
    .unwrap();
    assert!(reset.keys[0].name.is_none());
    assert_eq!(reset.primary_credential_id, Some(id));
    assert_eq!(vault.secrets.lock().unwrap().len(), 1);
}
fn add(store: &EncryptedStore, vault: &TestVault, provider: &str) -> AiKeyRegistry {
    add_key(
        store,
        vault,
        AddAiKeyRequest {
            provider: provider.into(),
            api_key: "synthetic-provider-key".into(),
            name: None,
        },
    )
    .unwrap()
}
#[test]
fn adding_a_key_saves_an_optional_trimmed_name() {
    let (_temp, store, vault) = fixture();
    let named = add_key(
        &store,
        &vault,
        AddAiKeyRequest {
            provider: "openai".into(),
            api_key: "synthetic-provider-key".into(),
            name: Some("  Research key  ".into()),
        },
    )
    .unwrap();
    assert_eq!(named.keys[0].name.as_deref(), Some("Research key"));

    let unnamed = add_key(
        &store,
        &vault,
        AddAiKeyRequest {
            provider: "gemini".into(),
            api_key: "synthetic-provider-key".into(),
            name: Some("   ".into()),
        },
    )
    .unwrap();
    assert!(unnamed.keys[1].name.is_none());
}
fn action(
    store: &EncryptedStore,
    vault: &TestVault,
    id: Uuid,
    action: AiKeyAction,
) -> AiKeyRegistry {
    change_key(
        store,
        vault,
        ChangeAiKeyRequest {
            credential_id: id,
            action,
        },
    )
    .unwrap()
}
#[test]
fn legacy_key_keeps_identity_provider_and_active_or_paused_state() {
    let id = Uuid::from_u128(42);
    for mode in ["direct_api", "no_ai"] {
        let registry = AiKeyRegistry::from_value(
            json!({ "mode": mode, "provider": "openai", "preset": "balanced", "credentialId": id }),
        )
        .unwrap();
        assert_eq!(registry.keys[0].credential_id, id);
        assert!(
            registry.keys[0]
                .created_at
                .parse::<jiff::Timestamp>()
                .is_ok()
        );
        assert_eq!(registry.keys[0].provider, "openai");
        assert_eq!(registry.keys[0].paused, mode == "no_ai");
        assert_eq!(
            registry.primary_credential_id,
            (mode == "direct_api").then_some(id)
        );
    }
}

#[test]
fn numbered_rosters_migrate_once_to_persisted_creation_timestamps() {
    let (_temp, store, _vault) = fixture();
    let id = Uuid::now_v7();
    store
        .save_setting(
            SETTING,
            None,
            &json!({
                "keys": [{
                    "credentialId": id,
                    "identificationNumber": 1,
                    "provider": "openai",
                    "preset": "balanced",
                    "paused": false,
                    "removed": false,
                    "cleanupRequired": false
                }],
                "primaryCredentialId": id,
                "nextIdentificationNumber": 2
            }),
        )
        .unwrap();

    let registry = load_registry(&store).unwrap().0;
    assert!(
        registry.keys[0]
            .created_at
            .parse::<jiff::Timestamp>()
            .is_ok()
    );
    let persisted = store.load_setting(SETTING).unwrap().unwrap().value;
    assert!(persisted.get("nextIdentificationNumber").is_none());
    assert!(persisted["keys"][0].get("identificationNumber").is_none());
    assert_eq!(
        persisted["keys"][0]["createdAt"],
        registry.keys[0].created_at
    );
}

#[test]
fn adding_keys_assigns_creation_dates_without_serializing_secrets() {
    let (_temp, store, vault) = fixture();
    let first = add(&store, &vault, "openai").keys[0].credential_id;
    assert!(
        load_registry(&store)
            .unwrap()
            .0
            .primary_credential_id
            .is_none()
    );
    action(&store, &vault, first, AiKeyAction::SelectPrimary);
    let second = add(&store, &vault, "anthropic");
    assert_eq!(second.primary_credential_id, Some(first));
    assert!(second.keys[1].created_at.parse::<jiff::Timestamp>().is_ok());
    assert_eq!(vault.secrets.lock().unwrap().len(), 2);
    action(&store, &vault, first, AiKeyAction::Remove);
    let third = add(&store, &vault, "openai");
    assert!(third.keys[2].created_at.parse::<jiff::Timestamp>().is_ok());
    assert!(third.keys[0].removed);
    assert_ne!(third.keys[0].credential_id, third.keys[2].credential_id);
    assert!(third.primary_credential_id.is_none());
    let saved = store
        .load_setting(SETTING)
        .unwrap()
        .unwrap()
        .value
        .to_string();
    assert!(!saved.contains("synthetic-provider-key"));
    assert!(!saved.contains("apiKey"));
    assert!(!saved.contains("identificationNumber"));
    assert!(!saved.contains("nextIdentificationNumber"));
}

#[test]
fn removed_key_data_deletion_rejects_saved_keys_and_forgets_only_removed_tombstones() {
    let (_temp, store, vault) = fixture();
    let removed = add(&store, &vault, "openai").keys[0].credential_id;
    let active = add(&store, &vault, "anthropic").keys[1].credential_id;
    action(&store, &vault, removed, AiKeyAction::Remove);

    assert_eq!(
        delete_removed_key_data(
            &store,
            &DeleteRemovedKeyDataRequest {
                credential_ids: vec![active],
            },
        )
        .unwrap_err(),
        "AI_KEY_NOT_REMOVED"
    );
    let deleted = delete_removed_key_data(
        &store,
        &DeleteRemovedKeyDataRequest {
            credential_ids: vec![removed],
        },
    )
    .unwrap();
    assert_eq!(deleted.cleared_operations, 0);
    assert_eq!(deleted.registry.keys.len(), 1);
    assert_eq!(deleted.registry.keys[0].credential_id, active);
    assert!(!deleted.registry.keys[0].removed);
}
#[test]
fn primary_switch_pause_and_unpause_have_no_implicit_failover() {
    let (_temp, store, vault) = fixture();
    add(&store, &vault, "openai");
    let registry = add(&store, &vault, "gemini");
    let first = registry.keys[0].credential_id;
    let second = registry.keys[1].credential_id;
    action(&store, &vault, first, AiKeyAction::SelectPrimary);
    assert_eq!(
        action(&store, &vault, second, AiKeyAction::SelectPrimary).primary_credential_id,
        Some(second)
    );
    assert_eq!(
        request_connection(&store, None).unwrap().credential_id,
        Some(second)
    );
    assert!(
        action(&store, &vault, second, AiKeyAction::Pause)
            .primary_credential_id
            .is_none()
    );
    assert_eq!(request_connection(&store, None).unwrap().mode, "no_ai");
    assert_eq!(
        request_connection(&store, Some(second)).unwrap().mode,
        "no_ai"
    );
    assert_eq!(
        change_key(
            &store,
            &vault,
            ChangeAiKeyRequest {
                credential_id: second,
                action: AiKeyAction::SelectPrimary
            }
        )
        .unwrap_err(),
        "AI_KEY_PAUSED"
    );
    assert!(
        action(&store, &vault, second, AiKeyAction::Unpause)
            .primary_credential_id
            .is_none()
    );
    assert_eq!(
        request_connection(&store, Some(first))
            .unwrap()
            .credential_id,
        Some(first)
    );
    assert!(
        request_connection(&store, None)
            .unwrap()
            .credential_id
            .is_none()
    );
}

#[test]
fn clearing_primary_is_explicit_and_idempotent() {
    let (_temp, store, vault) = fixture();
    let id = add(&store, &vault, "openai").keys[0].credential_id;
    action(&store, &vault, id, AiKeyAction::SelectPrimary);
    assert_eq!(
        load_registry(&store).unwrap().0.primary_credential_id,
        Some(id)
    );

    assert!(
        clear_primary(&store)
            .unwrap()
            .primary_credential_id
            .is_none()
    );
    assert!(
        clear_primary(&store)
            .unwrap()
            .primary_credential_id
            .is_none()
    );
    assert_eq!(request_connection(&store, None).unwrap().mode, "no_ai");
}

#[test]
fn explicit_test_can_use_paused_key_without_unpausing_or_selecting_it() {
    let (_temp, store, vault) = fixture();
    let registry = add(&store, &vault, "openai");
    let id = registry.keys[0].credential_id;
    action(&store, &vault, id, AiKeyAction::SelectPrimary);
    action(&store, &vault, id, AiKeyAction::Pause);
    assert_eq!(test_connection(&store, id).unwrap().mode, "direct_api");
    assert_eq!(request_connection(&store, Some(id)).unwrap().mode, "no_ai");
    assert_eq!(request_connection(&store, None).unwrap().mode, "no_ai");
    let (mut registry, revision) = load_registry(&store).unwrap();
    assert!(registry.keys[0].paused);
    assert!(registry.primary_credential_id.is_none());
    registry.keys[0].cleanup_required = true;
    let revision = save_registry(&store, &registry, revision).unwrap();
    assert_eq!(test_connection(&store, id).unwrap().mode, "no_ai");
    registry.keys[0].removed = true;
    save_registry(&store, &registry, Some(revision)).unwrap();
    assert!(test_connection(&store, id).is_err());
    assert!(test_connection(&store, Uuid::now_v7()).is_err());
}
#[test]
fn non_primary_removal_preserves_primary_and_all_keys_can_be_found_for_data_deletion() {
    let (_temp, store, vault) = fixture();
    add(&store, &vault, "anthropic");
    let registry = add(&store, &vault, "anthropic");
    let first = registry.keys[0].credential_id;
    let second = registry.keys[1].credential_id;
    action(&store, &vault, first, AiKeyAction::SelectPrimary);
    assert_eq!(saved_credential_references(&store).unwrap().len(), 2);
    assert_eq!(
        action(&store, &vault, second, AiKeyAction::Remove).primary_credential_id,
        Some(first)
    );
    assert!(request_connection(&store, Some(second)).is_err());
    assert_eq!(saved_credential_references(&store).unwrap().len(), 1);
    suspend_for_deletion(&store).unwrap();
    assert!(
        load_registry(&store)
            .unwrap()
            .0
            .primary_credential_id
            .is_none()
    );
    assert!(
        load_registry(&store)
            .unwrap()
            .0
            .keys
            .iter()
            .all(|key| key.paused)
    );
}
#[test]
fn partial_vault_write_retains_a_removable_disabled_address() {
    let (_temp, store, mut vault) = fixture();
    let primary = add(&store, &vault, "openai").keys[0].credential_id;
    action(&store, &vault, primary, AiKeyAction::SelectPrimary);
    vault.fail_store = true;
    assert_eq!(
        add_key(
            &store,
            &vault,
            AddAiKeyRequest {
                provider: "gemini".into(),
                api_key: "synthetic-provider-key".into(),
                name: None,
            }
        )
        .unwrap_err(),
        "AI_CREDENTIAL_CLEANUP_REQUIRED"
    );
    let registry = load_registry(&store).unwrap().0;
    assert!(registry.keys[1].cleanup_required && registry.keys[1].paused);
    assert_eq!(registry.primary_credential_id, Some(primary));
    let id = registry.keys[1].credential_id;
    assert!(
        change_key(
            &store,
            &vault,
            ChangeAiKeyRequest {
                credential_id: id,
                action: AiKeyAction::SelectPrimary
            }
        )
        .is_err()
    );
    action(&store, &vault, id, AiKeyAction::Remove);
    assert_eq!(vault.secrets.lock().unwrap().len(), 1);
}
#[test]
fn failed_removal_clears_primary_and_can_be_retried() {
    let (_temp, store, mut vault) = fixture();
    let id = add(&store, &vault, "openai").keys[0].credential_id;
    action(&store, &vault, id, AiKeyAction::SelectPrimary);
    vault.fail_delete = true;
    assert_eq!(
        change_key(
            &store,
            &vault,
            ChangeAiKeyRequest {
                credential_id: id,
                action: AiKeyAction::Remove
            }
        )
        .unwrap_err(),
        "AI_CREDENTIAL_CLEANUP_REQUIRED"
    );
    let registry = load_registry(&store).unwrap().0;
    assert!(registry.primary_credential_id.is_none());
    assert!(registry.keys[0].paused && registry.keys[0].cleanup_required);
    assert!(!registry.keys[0].removed);
    vault.fail_delete = false;
    assert!(action(&store, &vault, id, AiKeyAction::Remove).keys[0].removed);
}
#[test]
fn invalid_creation_dates_and_paused_primary_fail_closed() {
    let (_temp, store, vault) = fixture();
    add(&store, &vault, "openai");
    let mut registry = add(&store, &vault, "gemini");
    registry.keys[1].created_at = "not-a-date".into();
    assert!(registry.validate().is_err());
    registry.keys[1].created_at = jiff::Timestamp::now().to_string();
    registry.primary_credential_id = Some(registry.keys[0].credential_id);
    registry.keys[0].paused = true;
    assert!(registry.validate().is_err());
    assert!(AiKeyRegistry::from_value(json!({ "mode": "no_ai", "provider": "openai", "preset": "balanced", "credentialId": null })).is_err());
}

#[test]
fn legacy_migration_keeps_existing_cap_identity_and_new_rosters_are_excluded_from_backups() {
    use ort_storage::ai_activity::{AiCapPolicy, AiPeriod};
    let (_temp, store, vault) = fixture();
    let id = Uuid::from_u128(42);
    store.save_setting(SETTING, None, &json!({ "mode": "direct_api", "provider": "openai", "preset": "balanced", "credentialId": id })).unwrap();
    store
        .save_ai_cap_policy(&AiCapPolicy {
            credential_id: id,
            period: AiPeriod::AllTime,
            currency: "USD".into(),
            time_zone: "UTC".into(),
            limit_micros: 1_000_000,
            activated_at_unix_ms: 1,
            period_start_unix_ms: 1,
            period_end_unix_ms: None,
            expected_revision: None,
        })
        .unwrap();
    let registry = add(&store, &vault, "anthropic");
    assert_eq!(registry.primary_credential_id, Some(id));
    assert_eq!(registry.keys[0].credential_id, id);
    assert_eq!(
        store.ai_cap_policies(id).unwrap()[0].limit_micros,
        1_000_000
    );
    assert!(
        store
            .ai_cap_policies(registry.keys[1].credential_id)
            .unwrap()
            .is_empty()
    );
    let phrase =
        ort_backup::BackupPassphrase::new("synthetic roster backup phrase".into()).unwrap();
    let bytes = store.create_portable_backup(&phrase, "0.0.0-dev").unwrap();
    let backup = ort_backup::restore_backup(&bytes, &phrase).unwrap();
    assert!(!backup.profile.settings.contains_key(SETTING));
}
