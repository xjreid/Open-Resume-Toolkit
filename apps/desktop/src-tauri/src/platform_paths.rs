//! Native composition paths, never supplied by renderer IPC.
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupError {
    Unavailable,
    #[cfg(windows)]
    LegacyProfile,
}
impl StartupError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "STORAGE_UNAVAILABLE",
            #[cfg(windows)]
            Self::LegacyProfile => "WINDOWS_LEGACY_PROFILE_REQUIRES_BACKUP",
        }
    }
}

pub(crate) fn data_root(app: &AppHandle) -> Result<PathBuf, StartupError> {
    #[cfg(windows)]
    {
        let local = app
            .path()
            .app_local_data_dir()
            .map_err(|_| StartupError::Unavailable)?;
        let roaming = app
            .path()
            .app_data_dir()
            .map_err(|_| StartupError::Unavailable)?;
        windows_data_root(&local, &roaming)
    }
    #[cfg(not(windows))]
    app.path()
        .app_data_dir()
        .map_err(|_| StartupError::Unavailable)
}

#[cfg(windows)]
fn windows_data_root(
    local: &std::path::Path,
    roaming: &std::path::Path,
) -> Result<PathBuf, StartupError> {
    // Validate local paths before even probing a possible legacy profile;
    // a redirected/mapped network location must not trigger filesystem I/O.
    let _local = ort_windows::reject_reparse_path(local).map_err(|_| StartupError::Unavailable)?;
    let _roaming =
        ort_windows::reject_reparse_path(roaming).map_err(|_| StartupError::Unavailable)?;
    // A prior experimental Windows profile must be recovered deliberately
    // through encrypted backup, never hidden behind a new empty profile.
    for path in [local.join("profiles"), roaming.join("profiles")] {
        let _guards =
            ort_windows::reject_reparse_path(&path).map_err(|_| StartupError::Unavailable)?;
        match std::fs::read_dir(&path) {
            Ok(mut entries) => match entries.next() {
                Some(Ok(_)) => return Err(StartupError::LegacyProfile),
                Some(Err(_)) => return Err(StartupError::Unavailable),
                None => {} // An empty legacy directory contains no profile to recover.
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(StartupError::Unavailable),
        }
    }
    let root = local.join("private");
    ort_windows::PrivateDirectory::create(&root).map_err(|_| StartupError::Unavailable)?;
    Ok(root)
}

/// Storage startup failures are recoverable UI state, never shell construction errors.
pub(crate) fn initialize_storage(app: &AppHandle) -> crate::DesktopStorage {
    use crate::DesktopStorage;
    if !crate::development_identity_allowed(&app.config().identifier) {
        return DesktopStorage::Unavailable;
    }
    if app.try_state::<ort_storage::ProfileLifecycle>().is_none() {
        let lifecycle = data_root(app).and_then(|root| {
            ort_storage::ProfileLifecycle::acquire(&root.join("profiles/default"), "dev")
                .map_err(|_| StartupError::Unavailable)
        });
        match lifecycle {
            Ok(owner) => {
                app.manage(owner);
            }
            Err(error) => return DesktopStorage::StartupBlocked(error),
        }
    }
    let Some(owner) = app.try_state::<ort_storage::ProfileLifecycle>() else {
        return DesktopStorage::Unavailable;
    };
    match ort_storage::EncryptedStore::open_or_activate_pending_restore(
        owner.root(),
        owner.channel(),
        &ort_vault::OsDatabaseKeyVault::new(),
        &ort_vault::OsProviderCredentialVault::new(),
    ) {
        Ok((store, _)) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|value| i64::try_from(value.as_millis()).ok());
            if now.is_some_and(|now| store.recover_ai_attempts(now).is_ok())
                && crate::chatgpt_plan::reset_server_on_launch(&store).is_ok()
            {
                DesktopStorage::Ready(store)
            } else {
                DesktopStorage::Unavailable
            }
        }
        Err(_) => DesktopStorage::Unavailable,
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn fresh_and_empty_legacy_directories_allow_private_storage() {
        for empty_legacy in [false, true] {
            let temp = ort_windows::testing::tempdir().unwrap();
            let local = temp.path().join("local");
            let roaming = temp.path().join("roaming");
            if empty_legacy {
                std::fs::create_dir_all(local.join("profiles")).unwrap();
                std::fs::create_dir_all(roaming.join("profiles")).unwrap();
            }
            assert_eq!(
                windows_data_root(&local, &roaming).unwrap(),
                local.join("private")
            );
            assert!(!local.join("private/profiles/default").exists());
        }
    }
    #[test]
    fn legacy_data_is_preserved_and_recovery_can_be_retried_after_it_is_moved() {
        for location in ["local", "roaming"] {
            let temp = ort_windows::testing::tempdir().unwrap();
            let local = temp.path().join("local");
            let roaming = temp.path().join("roaming");
            let legacy = temp.path().join(location).join("profiles");
            std::fs::create_dir_all(&legacy).unwrap();
            std::fs::write(legacy.join("synthetic-profile"), b"PRESERVE").unwrap();
            assert_eq!(
                windows_data_root(&local, &roaming),
                Err(StartupError::LegacyProfile)
            );
            assert_eq!(
                std::fs::read(legacy.join("synthetic-profile")).unwrap(),
                b"PRESERVE"
            );
            assert!(!local.join("private").exists());
            std::fs::rename(
                &legacy,
                temp.path().join(location).join("retained-profiles"),
            )
            .unwrap();
            assert!(windows_data_root(&local, &roaming).is_ok());
        }
    }
}
