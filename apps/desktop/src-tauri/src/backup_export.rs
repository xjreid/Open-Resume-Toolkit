use ort_backup::{BackupPassphrase, inspect_backup, restore_backup};
use ort_domain::{
    BackupRecoveryStatusRequest, BackupRecoveryStatusResponse, CommandResponse,
    DeleteSafetyCopyRequest, DeleteSafetyCopyResponse, ExportBackupRequest, ExportBackupResponse,
    RestoreBackupRequest, RestoreBackupResponse, RollbackSafetyCopyRequest,
    RollbackSafetyCopyResponse, ValidateBackupRequest, ValidateBackupResponse,
};
use ort_platform::{
    ExportDestination, ExportFileType, ExportWriteError, NativeInputError, read_native_backup,
};
use ort_storage::StorageError;
use ort_vault::OsDatabaseKeyVault;
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use super::{DesktopState, DesktopStorage, text_export::ExportState, window_not_authorized};

#[tauri::command]
pub(crate) async fn export_portable_backup(
    window: WebviewWindow,
    request: ExportBackupRequest,
) -> CommandResponse<ExportBackupResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let exports = window.app_handle().state::<ExportState>();
    let Some(lease) = exports.begin() else {
        return backup_failure("BACKUP_BUSY");
    };

    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        export_with_dialog(&window, request.payload.passphrase)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => backup_failure("BACKUP_OUTCOME_UNKNOWN"),
    }
}

#[tauri::command]
pub(crate) async fn validate_portable_backup(
    window: WebviewWindow,
    request: ValidateBackupRequest,
) -> CommandResponse<ValidateBackupResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let exports = window.app_handle().state::<ExportState>();
    let Some(lease) = exports.begin() else {
        return validation_failure("BACKUP_BUSY");
    };

    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        validate_with_dialog(&window, request.payload.passphrase)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => validation_failure("BACKUP_OUTCOME_UNKNOWN"),
    }
}

#[tauri::command]
pub(crate) async fn restore_portable_backup(
    window: WebviewWindow,
    request: RestoreBackupRequest,
) -> CommandResponse<RestoreBackupResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let exports = window.app_handle().state::<ExportState>();
    let Some(lease) = exports.begin() else {
        return restore_failure("BACKUP_BUSY");
    };

    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        restore_with_dialog(&window, request.payload.passphrase)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => restore_failure("RESTORE_OUTCOME_UNKNOWN"),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn load_backup_recovery_status(
    window: WebviewWindow,
    request: BackupRecoveryStatusRequest,
) -> CommandResponse<BackupRecoveryStatusResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let state = window.app_handle().state::<DesktopState>();
    let DesktopStorage::Ready(store) = &state.storage else {
        return recovery_failure("STORAGE_UNAVAILABLE");
    };
    match store.backup_recovery_status() {
        Ok(status) => CommandResponse::success(BackupRecoveryStatusResponse {
            safety_copy_available: status.safety_copy_available,
            restart_operation_pending: status.restart_operation_pending,
            safety_cleanup_pending: status.safety_cleanup_pending,
        }),
        Err(error) => recovery_storage_failure(&error),
    }
}

#[tauri::command]
pub(crate) async fn rollback_safety_copy(
    window: WebviewWindow,
    request: RollbackSafetyCopyRequest,
) -> CommandResponse<RollbackSafetyCopyResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let exports = window.app_handle().state::<ExportState>();
    let Some(lease) = exports.begin() else {
        return recovery_failure("BACKUP_BUSY");
    };
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        rollback_safety_copy_blocking(&window)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => recovery_failure("RECOVERY_OUTCOME_UNKNOWN"),
    }
}

#[tauri::command]
pub(crate) async fn delete_safety_copy(
    window: WebviewWindow,
    request: DeleteSafetyCopyRequest,
) -> CommandResponse<DeleteSafetyCopyResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let exports = window.app_handle().state::<ExportState>();
    let Some(lease) = exports.begin() else {
        return recovery_failure("BACKUP_BUSY");
    };
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        delete_safety_copy_blocking(&window)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => recovery_failure("RECOVERY_OUTCOME_UNKNOWN"),
    }
}

fn export_with_dialog(
    window: &WebviewWindow,
    passphrase: String,
) -> CommandResponse<ExportBackupResponse> {
    let Ok(passphrase) = BackupPassphrase::new(passphrase) else {
        return backup_failure("INVALID_BACKUP_PASSPHRASE");
    };
    let selection = window
        .dialog()
        .file()
        .set_parent(window)
        .set_title("Create encrypted backup — choose a new filename")
        .set_file_name("open-resume-toolkit.ort-backup")
        .add_filter("Open Resume Toolkit backup", &["ort-backup"])
        .blocking_save_file();
    let Some(selection) = selection else {
        return CommandResponse::success(ExportBackupResponse::Cancelled);
    };
    let FilePath::Path(path) = selection else {
        return backup_failure("BACKUP_INVALID_DESTINATION");
    };
    // Hold directory authority before reading the encrypted profile. No path,
    // passphrase, profile content, vault identifier or OS error crosses back to JS.
    let destination = match ExportDestination::for_native_dialog(&path, ExportFileType::Backup) {
        Ok(value) => value,
        Err(error) => return backup_write_failure(&error),
    };
    let state = window.app_handle().state::<DesktopState>();
    let DesktopStorage::Ready(store) = &state.storage else {
        return backup_failure("STORAGE_UNAVAILABLE");
    };
    let bytes = match store.create_portable_backup(&passphrase, env!("CARGO_PKG_VERSION")) {
        Ok(value) => value,
        Err(error) => return backup_storage_failure(&error),
    };
    let Ok(header) = inspect_backup(&bytes) else {
        return backup_failure("BACKUP_INVALID_CONTENT");
    };
    match destination.write(&bytes) {
        Ok(receipt) => CommandResponse::success(ExportBackupResponse::Exported {
            byte_count: bytes.len(),
            format_major: header.format_major,
            format_minor: header.format_minor,
            cleanup_pending: receipt.cleanup_pending,
            durability_unconfirmed: receipt.durability_unconfirmed,
        }),
        Err(error) => backup_write_failure(&error),
    }
}

fn validate_with_dialog(
    window: &WebviewWindow,
    passphrase: String,
) -> CommandResponse<ValidateBackupResponse> {
    let Ok(passphrase) = BackupPassphrase::new(passphrase) else {
        return validation_failure("INVALID_BACKUP_PASSPHRASE");
    };
    let selection = window
        .dialog()
        .file()
        .set_parent(window)
        .set_title("Validate encrypted backup — no data will be replaced")
        .add_filter("Open Resume Toolkit backup", &["ort-backup"])
        .blocking_pick_file();
    let Some(selection) = selection else {
        return CommandResponse::success(ValidateBackupResponse::Cancelled);
    };
    let FilePath::Path(path) = selection else {
        return validation_failure("BACKUP_INVALID_OR_PASSPHRASE");
    };
    let bytes = match read_native_backup(&path) {
        Ok(value) => value,
        Err(error) => return backup_read_failure(&error),
    };
    validate_backup_bytes(&bytes, &passphrase)
}

fn validate_backup_bytes(
    bytes: &[u8],
    passphrase: &BackupPassphrase,
) -> CommandResponse<ValidateBackupResponse> {
    let Ok(backup) = restore_backup(bytes, passphrase) else {
        return validation_failure("BACKUP_INVALID_OR_PASSPHRASE");
    };
    CommandResponse::success(ValidateBackupResponse::Validated {
        byte_count: bytes.len(),
        format_major: backup.manifest.format_major,
        format_minor: backup.manifest.format_minor,
        app_version: backup.manifest.app_version,
        database_schema: backup.manifest.database_schema,
        document_schema: backup.manifest.document_schema,
        created_at: backup.manifest.created_at,
        master_drafts: backup.manifest.inventory.master_drafts,
        published_resumes: backup.manifest.inventory.published_resumes,
        settings: backup.manifest.inventory.settings,
        render_manifests: backup.manifest.inventory.render_manifests,
    })
}

fn restore_with_dialog(
    window: &WebviewWindow,
    passphrase: String,
) -> CommandResponse<RestoreBackupResponse> {
    let Ok(passphrase) = BackupPassphrase::new(passphrase) else {
        return restore_failure("INVALID_BACKUP_PASSPHRASE");
    };
    let selection = window
        .dialog()
        .file()
        .set_parent(window)
        .set_title("Replace saved profile from encrypted backup")
        .add_filter("Open Resume Toolkit backup", &["ort-backup"])
        .blocking_pick_file();
    let Some(selection) = selection else {
        return CommandResponse::success(RestoreBackupResponse::Cancelled);
    };
    let FilePath::Path(path) = selection else {
        return restore_failure("BACKUP_INVALID_OR_PASSPHRASE");
    };
    let bytes = match read_native_backup(&path) {
        Ok(value) => value,
        Err(error) => return restore_read_failure(&error),
    };
    let state = window.app_handle().state::<DesktopState>();
    let DesktopStorage::Ready(store) = &state.storage else {
        return restore_failure("STORAGE_UNAVAILABLE");
    };
    let vault = OsDatabaseKeyVault::new();
    match store.stage_portable_restore(&bytes, &passphrase, &store.manifest().channel, &vault) {
        Ok(()) => CommandResponse::success(RestoreBackupResponse::Staged {
            restart_required: true,
            safety_copy_retained: true,
        }),
        Err(error) => restore_storage_failure(&error),
    }
}

fn rollback_safety_copy_blocking(
    window: &WebviewWindow,
) -> CommandResponse<RollbackSafetyCopyResponse> {
    let state = window.app_handle().state::<DesktopState>();
    let DesktopStorage::Ready(store) = &state.storage else {
        return recovery_failure("STORAGE_UNAVAILABLE");
    };
    let vault = OsDatabaseKeyVault::new();
    match store.stage_safety_rollback(&store.manifest().channel, &vault) {
        Ok(()) => CommandResponse::success(RollbackSafetyCopyResponse {
            restart_required: true,
            current_profile_retained: true,
        }),
        Err(error) => recovery_storage_failure(&error),
    }
}

fn delete_safety_copy_blocking(
    window: &WebviewWindow,
) -> CommandResponse<DeleteSafetyCopyResponse> {
    let state = window.app_handle().state::<DesktopState>();
    let DesktopStorage::Ready(store) = &state.storage else {
        return recovery_failure("STORAGE_UNAVAILABLE");
    };
    let vault = OsDatabaseKeyVault::new();
    match store.delete_retained_safety_copy(&store.manifest().channel, &vault) {
        Ok(deleted) => CommandResponse::success(DeleteSafetyCopyResponse { deleted }),
        Err(error) => recovery_storage_failure(&error),
    }
}

fn backup_storage_failure(error: &StorageError) -> CommandResponse<ExportBackupResponse> {
    match error {
        StorageError::InvalidData => backup_failure("BACKUP_INVALID_CONTENT"),
        _ => backup_failure("BACKUP_UNAVAILABLE"),
    }
}

fn backup_write_failure(error: &ExportWriteError) -> CommandResponse<ExportBackupResponse> {
    backup_failure(match error {
        ExportWriteError::AlreadyExists => "BACKUP_ALREADY_EXISTS",
        ExportWriteError::InvalidDestination => "BACKUP_INVALID_DESTINATION",
        ExportWriteError::InvalidContent => "BACKUP_INVALID_CONTENT",
        ExportWriteError::Unavailable => "BACKUP_UNAVAILABLE",
    })
}

fn backup_read_failure(error: &NativeInputError) -> CommandResponse<ValidateBackupResponse> {
    validation_failure(match error {
        NativeInputError::Unavailable => "BACKUP_READ_UNAVAILABLE",
        NativeInputError::InvalidSelection | NativeInputError::InvalidContent => {
            "BACKUP_INVALID_OR_PASSPHRASE"
        }
    })
}

fn restore_read_failure(error: &NativeInputError) -> CommandResponse<RestoreBackupResponse> {
    restore_failure(match error {
        NativeInputError::Unavailable => "BACKUP_READ_UNAVAILABLE",
        NativeInputError::InvalidSelection | NativeInputError::InvalidContent => {
            "BACKUP_INVALID_OR_PASSPHRASE"
        }
    })
}

fn restore_storage_failure(error: &StorageError) -> CommandResponse<RestoreBackupResponse> {
    restore_failure(match error {
        StorageError::InvalidData => "BACKUP_INVALID_OR_PASSPHRASE",
        StorageError::RevisionConflict => "RESTORE_RECOVERY_PENDING",
        StorageError::VaultKeyUnavailable => "RESTORE_VAULT_UNAVAILABLE",
        _ => "RESTORE_UNAVAILABLE",
    })
}

fn recovery_storage_failure<T: serde::Serialize>(error: &StorageError) -> CommandResponse<T> {
    recovery_failure(match error {
        StorageError::NotFound => "SAFETY_COPY_NOT_FOUND",
        StorageError::RevisionConflict => "RECOVERY_BUSY",
        StorageError::IntegrityFailure
        | StorageError::DatabaseKeyMismatch
        | StorageError::InvalidData
        | StorageError::VaultKeyUnavailable => "SAFETY_COPY_UNAVAILABLE",
        _ => "RECOVERY_UNAVAILABLE",
    })
}

fn backup_failure(code: &str) -> CommandResponse<ExportBackupResponse> {
    CommandResponse::failure(code, "errors.backupExport", false)
}

fn validation_failure(code: &str) -> CommandResponse<ValidateBackupResponse> {
    CommandResponse::failure(code, "errors.backupValidation", false)
}

fn restore_failure(code: &str) -> CommandResponse<RestoreBackupResponse> {
    CommandResponse::failure(code, "errors.backupRestore", false)
}

fn recovery_failure<T: serde::Serialize>(code: &str) -> CommandResponse<T> {
    CommandResponse::failure(code, "errors.backupRecovery", false)
}

#[cfg(test)]
mod tests {
    use ort_backup::{BackupExportRequestV1, PortableProfileV1, create_backup};

    use super::*;

    #[test]
    fn backup_errors_are_bounded_and_non_sensitive() {
        for (error, code) in [
            (ExportWriteError::AlreadyExists, "BACKUP_ALREADY_EXISTS"),
            (
                ExportWriteError::InvalidDestination,
                "BACKUP_INVALID_DESTINATION",
            ),
            (ExportWriteError::InvalidContent, "BACKUP_INVALID_CONTENT"),
            (ExportWriteError::Unavailable, "BACKUP_UNAVAILABLE"),
        ] {
            let CommandResponse::Failure { error, .. } = backup_write_failure(&error) else {
                panic!("expected failure");
            };
            assert_eq!(error.code, code);
            assert!(error.details.is_empty());
        }
    }

    #[test]
    fn backup_validation_authenticates_before_returning_content_free_inventory() {
        let passphrase = BackupPassphrase::new("synthetic validation phrase".to_owned()).unwrap();
        let bytes = create_backup(
            &passphrase,
            BackupExportRequestV1 {
                app_version: "0.0.0-dev".to_owned(),
                created_at: "2026-09-03T12:00:00Z".to_owned(),
                profile: PortableProfileV1::default(),
            },
        )
        .unwrap();
        let CommandResponse::Success {
            value:
                ValidateBackupResponse::Validated {
                    byte_count,
                    master_drafts,
                    published_resumes,
                    settings,
                    render_manifests,
                    ..
                },
            ..
        } = validate_backup_bytes(&bytes, &passphrase)
        else {
            panic!("expected validated backup");
        };
        assert_eq!(byte_count, bytes.len());
        assert_eq!(
            (master_drafts, published_resumes, settings, render_manifests),
            (0, 0, 0, 0)
        );

        let wrong = BackupPassphrase::new("wrong validation phrase".to_owned()).unwrap();
        let CommandResponse::Failure { error, .. } = validate_backup_bytes(&bytes, &wrong) else {
            panic!("expected uniform invalid response");
        };
        assert_eq!(error.code, "BACKUP_INVALID_OR_PASSPHRASE");
        assert!(error.details.is_empty());
    }

    #[test]
    fn restore_errors_are_stable_and_do_not_expose_storage_details() {
        for (error, code) in [
            (StorageError::InvalidData, "BACKUP_INVALID_OR_PASSPHRASE"),
            (StorageError::RevisionConflict, "RESTORE_RECOVERY_PENDING"),
            (
                StorageError::VaultKeyUnavailable,
                "RESTORE_VAULT_UNAVAILABLE",
            ),
            (StorageError::Unavailable, "RESTORE_UNAVAILABLE"),
        ] {
            let CommandResponse::Failure { error, .. } = restore_storage_failure(&error) else {
                panic!("expected failure");
            };
            assert_eq!(error.code, code);
            assert!(error.details.is_empty());
        }
    }

    #[test]
    fn recovery_errors_are_stable_and_content_free() {
        for (error, code) in [
            (StorageError::NotFound, "SAFETY_COPY_NOT_FOUND"),
            (StorageError::RevisionConflict, "RECOVERY_BUSY"),
            (StorageError::IntegrityFailure, "SAFETY_COPY_UNAVAILABLE"),
            (StorageError::InvalidData, "SAFETY_COPY_UNAVAILABLE"),
            (StorageError::Unavailable, "RECOVERY_UNAVAILABLE"),
        ] {
            let CommandResponse::Failure { error, .. } =
                recovery_storage_failure::<DeleteSafetyCopyResponse>(&error)
            else {
                panic!("expected failure");
            };
            assert_eq!(error.code, code);
            assert!(error.details.is_empty());
        }
    }
}
