use ort_domain::{CommandResponse, DeleteAllLocalDataRequest, DeleteAllLocalDataResponse};
use ort_storage::{AllLocalDataDeletion, EncryptedStore, StorageError};
use ort_vault::{
    DatabaseKeyVault, OsDatabaseKeyVault, OsProviderCredentialVault, ProviderCredentialVault,
};
use tauri::{Manager, WebviewWindow};

use super::{
    DesktopState, DesktopStorage, ai_request::AiRequestGate, ai_settings, pdf_preview::PdfState,
    text_export::ExportState, window_not_authorized,
};

#[tauri::command]
pub(crate) async fn delete_all_local_data(
    window: WebviewWindow,
    request: DeleteAllLocalDataRequest,
) -> CommandResponse<DeleteAllLocalDataResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("LOCAL_DATA_OPERATION_BUSY", true);
    };
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let gate = app.state::<AiRequestGate>();
        let Some(_ai_lease) = gate.begin(uuid::Uuid::now_v7()) else {
            return failure("LOCAL_DATA_OPERATION_BUSY", true);
        };
        let state = app.state::<DesktopState>();
        let previews = app.state::<PdfState>();
        delete_and_reinitialize(
            &state,
            &previews,
            &OsDatabaseKeyVault::new(),
            &OsProviderCredentialVault::new(),
        )
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("LOCAL_DATA_DELETE_OUTCOME_UNKNOWN", true),
    }
}

fn delete_and_reinitialize<P: ProviderCredentialVault>(
    state: &DesktopState,
    previews: &PdfState,
    vault: &dyn DatabaseKeyVault,
    provider_vault: &P,
) -> CommandResponse<DeleteAllLocalDataResponse> {
    let Ok(store) = state.take_store() else {
        return failure("STORAGE_UNAVAILABLE", true);
    };
    if let Err(error) = store.validate_all_data_deletion_boundary() {
        let _ = state.replace_storage(DesktopStorage::Ready(store));
        return deletion_failure(&error);
    }
    let credential = match ai_settings::saved_credential_reference(&store) {
        Ok(value) => value,
        Err(error) => {
            let _ = state.replace_storage(DesktopStorage::Ready(store));
            return deletion_failure(&error);
        }
    };
    if credential
        .as_ref()
        .is_some_and(|reference| provider_vault.delete(reference).is_err())
    {
        let _ = state.replace_storage(DesktopStorage::Ready(store));
        return failure("LOCAL_DATA_CREDENTIAL_DELETE_UNAVAILABLE", true);
    }
    let Some(root) = store.database_path().parent().map(ToOwned::to_owned) else {
        let _ = state.replace_storage(DesktopStorage::Unavailable);
        return failure("LOCAL_DATA_DELETE_UNSAFE", false);
    };
    let channel = store.manifest().channel.clone();
    drop(store); // Close SQLCipher/WAL handles before committing deletion intent.

    let deletion = match EncryptedStore::delete_all_local_data(&root, &channel, vault) {
        Ok(value) => value,
        Err(error) => {
            // A refused deletion must not consume a separately staged restore.
            // Reopen only the same active root that was just closed.
            let reopened = EncryptedStore::open_or_initialize(&root, &channel, vault)
                .map_or(DesktopStorage::Unavailable, DesktopStorage::Ready);
            let _ = state.replace_storage(reopened);
            return deletion_failure(&error);
        }
    };

    let _ = previews.clear();
    match deletion {
        AllLocalDataDeletion::CleanupPending => {
            let _ = state.replace_storage(DesktopStorage::Unavailable);
            CommandResponse::success(DeleteAllLocalDataResponse::CleanupPending {
                restart_required: true,
            })
        }
        AllLocalDataDeletion::Deleted => {
            let fresh = EncryptedStore::open_or_activate_pending_restore(&root, &channel, vault)
                .map(|(store, _)| store);
            let fresh_profile_ready = fresh.is_ok();
            let replacement = fresh.map_or(DesktopStorage::Unavailable, DesktopStorage::Ready);
            let stored = state.replace_storage(replacement).is_ok();
            CommandResponse::success(DeleteAllLocalDataResponse::Deleted {
                fresh_profile_ready: fresh_profile_ready && stored,
            })
        }
    }
}

fn deletion_failure(error: &StorageError) -> CommandResponse<DeleteAllLocalDataResponse> {
    match error {
        StorageError::UnsafeLocation
        | StorageError::InvalidManifest
        | StorageError::IncompleteInitialization => failure("LOCAL_DATA_DELETE_UNSAFE", false),
        _ => failure("LOCAL_DATA_DELETE_UNAVAILABLE", true),
    }
}

fn failure(code: &str, retryable: bool) -> CommandResponse<DeleteAllLocalDataResponse> {
    CommandResponse::failure(code, "errors.localDataDelete", retryable)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use ort_domain::ResumeDocument;
    use ort_vault::testing::MemoryDatabaseKeyVault;
    use ort_vault::{ProviderCredentialReference, ProviderSecret, VaultError};
    use tempfile::TempDir;

    use super::*;

    struct NoopProviderVault;
    impl ProviderCredentialVault for NoopProviderVault {
        fn use_secret<T>(
            &self,
            _reference: &ProviderCredentialReference,
            _operation: impl FnOnce(&ProviderSecret) -> T,
        ) -> Result<T, VaultError> {
            Err(VaultError::Missing)
        }

        fn store_new(
            &self,
            _reference: &ProviderCredentialReference,
            _secret: &ProviderSecret,
        ) -> Result<(), VaultError> {
            Ok(())
        }

        fn delete(&self, _reference: &ProviderCredentialReference) -> Result<(), VaultError> {
            Ok(())
        }
    }

    struct RecordingProviderVault(AtomicBool);
    impl ProviderCredentialVault for RecordingProviderVault {
        fn use_secret<T>(
            &self,
            _reference: &ProviderCredentialReference,
            _operation: impl FnOnce(&ProviderSecret) -> T,
        ) -> Result<T, VaultError> {
            Err(VaultError::Missing)
        }

        fn store_new(
            &self,
            _reference: &ProviderCredentialReference,
            _secret: &ProviderSecret,
        ) -> Result<(), VaultError> {
            Ok(())
        }

        fn delete(&self, _reference: &ProviderCredentialReference) -> Result<(), VaultError> {
            self.0.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn deletion_replaces_the_active_store_with_a_new_empty_identity() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("default");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(&root, "test", &vault).unwrap();
        store
            .create_draft(&ResumeDocument::empty("Synthetic deleted draft"))
            .unwrap();
        let previous_profile_id = store.manifest().profile_id;
        let state = DesktopState {
            reviews: std::sync::Arc::default(),
            storage: Mutex::new(DesktopStorage::Ready(store)),
        };
        let previews = PdfState::default();

        let response = delete_and_reinitialize(&state, &previews, &vault, &NoopProviderVault);
        assert!(matches!(
            response,
            CommandResponse::Success {
                value: DeleteAllLocalDataResponse::Deleted {
                    fresh_profile_ready: true
                },
                ..
            }
        ));
        state
            .with_store(|fresh| {
                assert_ne!(fresh.manifest().profile_id, previous_profile_id);
                assert!(fresh.load_draft()?.is_none());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn deletion_removes_the_active_provider_vault_item() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("default");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(&root, "test", &vault).unwrap();
        store
            .save_setting(
                "ai.connection.v1",
                None,
                &serde_json::json!({
                    "mode": "direct_api",
                    "provider": "openai",
                    "preset": "balanced",
                    "credentialId": uuid::Uuid::now_v7(),
                }),
            )
            .unwrap();
        let state = DesktopState {
            reviews: std::sync::Arc::default(),
            storage: Mutex::new(DesktopStorage::Ready(store)),
        };
        let provider_vault = RecordingProviderVault(AtomicBool::new(false));
        let response =
            delete_and_reinitialize(&state, &PdfState::default(), &vault, &provider_vault);
        assert!(matches!(response, CommandResponse::Success { .. }));
        assert!(provider_vault.0.load(Ordering::SeqCst));
    }

    #[test]
    fn unavailable_storage_cannot_be_misreported_as_deleted() {
        let state = DesktopState {
            reviews: std::sync::Arc::default(),
            storage: Mutex::new(DesktopStorage::Unavailable),
        };
        let response = delete_and_reinitialize(
            &state,
            &PdfState::default(),
            &MemoryDatabaseKeyVault::new(),
            &NoopProviderVault,
        );
        let CommandResponse::Failure { error, .. } = response else {
            panic!("unavailable storage must fail");
        };
        assert_eq!(error.code, "STORAGE_UNAVAILABLE");
        assert!(error.retryable);
        assert!(error.details.is_empty());
    }

    #[test]
    fn refused_deletion_reopens_the_same_active_profile() {
        use std::fs;

        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("default");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(&root, "test", &vault).unwrap();
        store
            .create_draft(&ResumeDocument::empty("Preserve refused deletion"))
            .unwrap();
        fs::write(root.join("unknown-entry"), b"do not remove").unwrap();
        let state = DesktopState {
            reviews: std::sync::Arc::default(),
            storage: Mutex::new(DesktopStorage::Ready(store)),
        };

        let response =
            delete_and_reinitialize(&state, &PdfState::default(), &vault, &NoopProviderVault);
        let CommandResponse::Failure { error, .. } = response else {
            panic!("unsafe deletion must fail");
        };
        assert_eq!(error.code, "LOCAL_DATA_DELETE_UNSAFE");
        state
            .with_store(|reopened| {
                assert_eq!(
                    reopened.load_draft()?.unwrap().document.title,
                    "Preserve refused deletion"
                );
                Ok(())
            })
            .unwrap();
        assert!(root.join("unknown-entry").is_file());
    }
}
