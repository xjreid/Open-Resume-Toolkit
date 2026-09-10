//! Explicit native opt-in only. Never compiled into the application.
//! Uses disposable profiles and platform-test Keychain items, not app data.

use super::*;
use ort_vault::OsDatabaseKeyVault;
use security_framework::os::macos::keychain::SecKeychain;
use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};
use std::process::{Child, Command};
use std::time::Instant;
use tempfile::TempDir;

const CHANNEL: &str = "platform-test";
const MARKER: &str = "ORT M1 native synthetic encrypted marker";
const CHILD: &str = "native_qualification::native_child";

fn opt_in() {
    assert_eq!(std::env::var("ORT_RUN_OS_VAULT_TESTS").as_deref(), Ok("1"));
}

// The parent kills only its own child once the real production operation reaches
// the selected boundary. Neither the hook nor its environment lookup exists in
// a non-test build. It does not approximate a crash by copying a live database.
pub(super) fn crash_checkpoint(point: &str) {
    if std::env::var("ORT_M1_CRASH_POINT").as_deref() == Ok(point) {
        opt_in();
        let ready = std::env::var_os("ORT_M1_READY").expect("child readiness path");
        fs::write(ready, point).expect("signal exact crash boundary");
        for _ in 0..30 {
            std::thread::sleep(Duration::from_secs(1));
        }
        panic!("qualification parent did not terminate its child within the deadline");
    }
}

struct NativeProfile {
    // Declared before temporary so exact vault cleanup runs while files exist.
    references: Vec<VaultReference>,
    temporary: TempDir,
}

impl NativeProfile {
    fn new() -> Self {
        Self::in_directory(&std::env::temp_dir())
    }

    fn in_directory(directory: &Path) -> Self {
        let temporary = tempfile::Builder::new()
            .prefix("ort-m1-native-")
            .tempdir_in(directory)
            .unwrap();
        fs::write(temporary.path().join("qualification-only"), b"1").unwrap();
        Self {
            references: vec![],
            temporary,
        }
    }

    fn root(&self) -> PathBuf {
        self.temporary.path().join("profiles/default")
    }

    fn initialize(&mut self) -> EncryptedStore {
        let store =
            EncryptedStore::open_or_initialize(&self.root(), CHANNEL, &OsDatabaseKeyVault::new())
                .unwrap();
        self.references
            .push(store.manifest().vault_reference().unwrap());
        store.create_draft(&ResumeDocument::empty(MARKER)).unwrap();
        store.publish_draft(1).unwrap();
        store
    }

    fn child(&self, action: &str, point: Option<&str>) -> Child {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", CHILD, "--ignored", "--nocapture"])
            .env("ORT_M1_ROOT", self.temporary.path())
            .env("ORT_M1_ACTION", action)
            .env("ORT_M1_READY", self.temporary.path().join("ready"))
            .env_remove("ORT_M1_CRASH_POINT");
        if let Some(point) = point {
            command.env("ORT_M1_CRASH_POINT", point);
        }
        command.spawn().unwrap()
    }

    fn run(&self, action: &str) {
        let mut child = self.child(action, None);
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "native child failed: {action}");
                return;
            }
            if Instant::now() > deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("native child timed out: {action}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn crash(&self, action: &str, point: &str) {
        let ready = self.temporary.path().join("ready");
        assert!(!ready.exists());
        let mut child = self.child(action, Some(point));
        let deadline = Instant::now() + Duration::from_secs(30);
        while !ready.exists() {
            assert!(
                child.try_wait().unwrap().is_none(),
                "child exited before crash boundary"
            );
            if Instant::now() > deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("crash boundary was not reached");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        child.kill().expect("SIGKILL owned child");
        let status = child.wait().expect("reap owned child");
        assert_eq!(status.signal(), Some(9));
        fs::remove_file(ready).unwrap();
    }
}

#[test]
#[ignore = "fills only a harness-created 64 MiB disk image; explicit native opt-in required"]
fn native_low_disk() {
    opt_in();
    let _no_ui = SecKeychain::disable_user_interaction().unwrap();
    let volume = PathBuf::from(
        std::env::var_os("ORT_M1_BOUNDED_VOLUME").expect("use native qualification harness"),
    );
    assert_eq!(
        fs::read(volume.join("ort-m1-bounded-volume")).unwrap(),
        b"64-MiB-disposable-image"
    );
    let mut profile = NativeProfile::in_directory(&volume);
    let store = profile.initialize();
    store
        .connection
        .lock()
        .unwrap()
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .unwrap();
    let filler = profile.temporary.path().join("bounded-filler");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&filler)
        .unwrap();
    let block = vec![0xa5; 1024 * 1024];
    let mut full = false;
    // Even an incorrect harness path cannot create an unbounded host-disk fill.
    for _ in 0..80 {
        if let Err(error) = file.write_all(&block).and_then(|()| file.sync_all()) {
            assert_eq!(error.raw_os_error(), Some(28), "must observe real ENOSPC");
            full = true;
            break;
        }
    }
    assert!(
        full,
        "bounded image did not reach ENOSPC within the hard cap"
    );
    drop(file);
    let large = Value::from("synthetic native disk-full record ".repeat(1800));
    // A failed 1 MiB allocation can leave smaller usable extents. Bound further
    // writes and require a real database failure, rather than assuming the very
    // next small transaction must fail.
    let mut failed_key = None;
    let mut committed = Vec::new();
    for index in 0..64 {
        let key = format!("qualification.disk{index}");
        match store.save_setting(&key, None, &large) {
            Ok(_) => committed.push(key),
            Err(StorageError::Unavailable) => {
                failed_key = Some(key);
                break;
            }
            Err(error) => panic!("unexpected failure during native disk-full write: {error}"),
        }
    }
    fs::remove_file(&filler).unwrap();
    let failed_key = failed_key.expect("a bounded SQLCipher write must fail on the full image");
    assert!(store.load_setting(&failed_key).unwrap().is_none());
    assert_eq!(store.load_draft().unwrap().unwrap().revision, 1);
    store.verify_integrity().unwrap();
    drop(store);
    let vault = OsDatabaseKeyVault::new();
    let reopened = EncryptedStore::open_or_initialize(&profile.root(), CHANNEL, &vault).unwrap();
    assert!(reopened.load_setting(&failed_key).unwrap().is_none());
    for key in committed {
        assert_eq!(reopened.load_setting(&key).unwrap().unwrap().value, large);
    }
    assert_eq!(
        reopened.load_draft().unwrap().unwrap().document.title,
        MARKER
    );
    reopened
        .save_setting("qualification.disk", None, &Value::from(1))
        .unwrap();
    assert_encrypted(&profile.root(), true);
    println!(
        "PASS actual disk-image ENOSPC, failed SQLCipher write rollback, reopen and save after space recovery"
    );
}

#[test]
#[ignore = "read-only access-denial probe of the developer app key; native opt-in required"]
fn native_untrusted_process_cannot_load_installed_key() {
    opt_in();
    let _no_ui = SecKeychain::disable_user_interaction().unwrap();
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let root = home.join("Library/Application Support/com.openresumetoolkit.dev/profiles/default");
    let manifest = read_manifest(&root.join(MANIFEST_FILENAME), "dev").unwrap();
    let reference = manifest.vault_reference().unwrap();
    // Item metadata presence is a positive control for a real matching address;
    // the security tool is never asked to retrieve or print the secret.
    let status = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s",
            reference.service(),
            "-a",
            reference.account(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "exact installed key metadata must exist");
    assert!(
        matches!(
            OsDatabaseKeyVault::new().load(&reference),
            Err(VaultError::Unavailable)
        ),
        "unapproved helper must not load the installed application's key"
    );
    println!(
        "PASS installed key exists but distinct helper cannot load it with interaction disabled"
    );
}

impl Drop for NativeProfile {
    fn drop(&mut self) {
        for reference in &self.references {
            assert_eq!(
                reference.service(),
                "com.openresumetoolkit.platform-test.database"
            );
            OsDatabaseKeyVault::new()
                .delete(reference)
                .expect("clean exact synthetic key");
        }
    }
}

fn file_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

fn assert_encrypted(root: &Path, require_wal: bool) {
    let markers = [
        MARKER.as_bytes().to_vec(),
        MARKER.encode_utf16().flat_map(u16::to_le_bytes).collect(),
        MARKER.encode_utf16().flat_map(u16::to_be_bytes).collect(),
    ];
    for name in [DATABASE_FILENAME, "profile.db-wal"] {
        let path = root.join(name);
        if name.ends_with("-wal") && !require_wal && !path.exists() {
            continue;
        }
        let bytes = fs::read(&path).unwrap();
        if name.ends_with("-wal") && require_wal {
            assert!(bytes.len() > 32);
        }
        assert!(!bytes.starts_with(b"SQLite format 3"));
        for marker in &markers {
            assert!(!bytes.windows(marker.len()).any(|window| window == marker));
        }
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
#[ignore = "native child; invoked only by the opted-in qualification parent"]
fn native_child() {
    opt_in();
    let _no_ui = SecKeychain::disable_user_interaction().unwrap();
    let parent = PathBuf::from(std::env::var_os("ORT_M1_ROOT").unwrap());
    assert!(
        parent
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("ort-m1-native-")
    );
    assert_eq!(fs::read(parent.join("qualification-only")).unwrap(), b"1");
    let root = parent.join("profiles/default");
    let action = std::env::var("ORT_M1_ACTION").unwrap();
    let vault = OsDatabaseKeyVault::new();
    if action == "activate" {
        EncryptedStore::open_or_activate_pending_restore(&root, CHANNEL, &vault).unwrap();
        return;
    }
    if action == "delete-all" {
        EncryptedStore::delete_all_local_data(&root, CHANNEL, &vault).unwrap();
        return;
    }
    let store = EncryptedStore::open_or_initialize(&root, CHANNEL, &vault).unwrap();
    if action == "delete-safety" {
        store.delete_retained_safety_copy(CHANNEL, &vault).unwrap();
        return;
    }
    let draft = store.load_draft().unwrap().unwrap();
    assert_eq!(draft.document.title, MARKER);
    if action == "wal-write" {
        store.save_draft(draft.revision, &draft.document).unwrap();
        assert_encrypted(&root, true);
        crash_checkpoint("wal-committed");
    } else {
        assert_eq!(action, "reopen");
    }
    store.verify_integrity().unwrap();
}

#[test]
#[ignore = "real Keychain, SQLCipher and SIGKILL; explicit native opt-in required"]
fn native_storage_failure_matrix() {
    opt_in();
    let _no_ui = SecKeychain::disable_user_interaction().unwrap();
    let vault = OsDatabaseKeyVault::new();

    // Real independent processes reopen the same native-vault-backed profile;
    // one is killed with committed data still in its encrypted WAL.
    let mut profile = NativeProfile::new();
    let store = profile.initialize();
    assert_encrypted(&profile.root(), true);
    drop(store);
    profile.run("reopen");
    profile.crash("wal-write", "wal-committed");
    assert_encrypted(&profile.root(), true);
    profile.run("reopen");
    let reopened = EncryptedStore::open_or_initialize(&profile.root(), CHANNEL, &vault).unwrap();
    assert_eq!(reopened.load_draft().unwrap().unwrap().revision, 2);
    assert_eq!(
        reopened.load_latest_published().unwrap().unwrap().revision,
        1
    );
    drop(reopened);
    println!(
        "PASS native independent-process reopen, committed WAL SIGKILL recovery, encrypted markers"
    );

    for wrong in [false, true] {
        let mut profile = NativeProfile::new();
        drop(profile.initialize());
        let reference = &profile.references[0];
        let original = vault.load(reference).unwrap();
        let before = file_snapshot(&profile.root());
        vault.delete(reference).unwrap();
        if wrong {
            vault
                .store_new(reference, &DatabaseKey::generate().unwrap())
                .unwrap();
        }
        let expected = if wrong {
            StorageError::DatabaseKeyMismatch
        } else {
            StorageError::VaultKeyUnavailable
        };
        assert_eq!(
            EncryptedStore::open_or_initialize(&profile.root(), CHANNEL, &vault).err(),
            Some(expected)
        );
        assert_eq!(file_snapshot(&profile.root()), before);
        vault.delete(reference).unwrap();
        vault.store_new(reference, &original).unwrap();
        profile.run("reopen");
    }
    println!(
        "PASS native missing/wrong Keychain keys, exact file nonmutation, original-key recovery"
    );

    let mut corrupted = NativeProfile::new();
    drop(corrupted.initialize());
    let database = corrupted.root().join(DATABASE_FILENAME);
    let mut bytes = fs::read(&database).unwrap();
    bytes[4196] ^= 0xa5;
    fs::write(&database, bytes).unwrap();
    let before = file_snapshot(&corrupted.root());
    assert_eq!(
        EncryptedStore::open_or_initialize(&corrupted.root(), CHANNEL, &vault).err(),
        Some(StorageError::IntegrityFailure)
    );
    // SQLite may create/remove empty journal bookkeeping; database and manifest
    // must be byte-identical and no successful store may be returned.
    let after = file_snapshot(&corrupted.root());
    for name in [DATABASE_FILENAME, MANIFEST_FILENAME] {
        assert_eq!(before[name], after[name]);
    }
    println!("PASS native ciphertext corruption refusal without database/manifest rewrite");

    for point in [
        "migration-before-commit",
        "migration-after-commit",
        "manifest-handoff",
    ] {
        let mut profile = NativeProfile::new();
        let store = profile.initialize();
        store
            .connection
            .lock()
            .unwrap()
            .execute_batch(
                "DROP TABLE render_manifests; DELETE FROM schema_migrations WHERE version = 2;",
            )
            .unwrap();
        let mut manifest = store.manifest().clone();
        drop(store);
        manifest.schema_version = 1;
        fs::write(
            profile.root().join(MANIFEST_FILENAME),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        profile.crash("reopen", point);
        profile.run("reopen");
        let store = EncryptedStore::open_or_initialize(&profile.root(), CHANNEL, &vault).unwrap();
        assert_eq!(store.manifest().schema_version, 2);
        assert_eq!(store.load_draft().unwrap().unwrap().revision, 1);
        assert_eq!(store.load_latest_published().unwrap().unwrap().revision, 1);
        assert!(!profile.root().join(PREVIOUS_MANIFEST_FILENAME).exists());
        assert!(!profile.root().join(MANIFEST_UPDATE_FILENAME).exists());
        println!("PASS native SIGKILL at {point} and exact schema/data recovery");
    }

    qualify_portable_key_separation_and_deletion(&vault);
}

fn qualify_portable_key_separation_and_deletion(vault: &OsDatabaseKeyVault) {
    // Portable recovery uses a second real native key. Secret-shaped settings
    // make export fail closed; no device identity/key appears in a valid payload.
    let mut source = NativeProfile::new();
    let store = source.initialize();
    let passphrase =
        BackupPassphrase::new("disposable synthetic native test passphrase".into()).unwrap();
    store
        .save_setting(
            "provider.api_key",
            None,
            &Value::from("synthetic credential"),
        )
        .unwrap();
    assert_eq!(
        store.create_portable_backup(&passphrase, "0.0.0-dev"),
        Err(StorageError::InvalidData)
    );
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "DELETE FROM settings WHERE setting_key = 'provider.api_key'",
            [],
        )
        .unwrap();
    let backup = store
        .create_portable_backup(&passphrase, "0.0.0-dev")
        .unwrap();
    let plaintext =
        serde_json::to_vec(&restore_backup(&backup, &passphrase).unwrap().profile).unwrap();
    let source_key = vault.load(&source.references[0]).unwrap();
    source_key.expose_for(|key| {
        assert!(!plaintext.windows(key.len()).any(|w| w == key));
        assert!(
            !plaintext
                .windows(64)
                .any(|w| w == hex::encode(key).as_bytes())
        );
    });
    for id in [store.manifest().install_id, store.manifest().profile_id] {
        assert!(!String::from_utf8_lossy(&plaintext).contains(&id.to_string()));
    }
    let destination = source.temporary.path().join("other/profiles/default");
    let target = EncryptedStore::open_or_initialize(&destination, CHANNEL, vault).unwrap();
    let target_ref = target.manifest().vault_reference().unwrap();
    source.references.push(target_ref.clone());
    let target_key = vault.load(&target_ref).unwrap();
    assert!(source_key.expose_for(|a| target_key.expose_for(|b| a != b)));
    target
        .restore_portable_backup(&backup, &passphrase)
        .unwrap();
    assert_eq!(target.load_draft().unwrap().unwrap().document.title, MARKER);
    drop(target);
    assert_eq!(
        EncryptedStore::delete_all_local_data(&destination, CHANNEL, vault).unwrap(),
        AllLocalDataDeletion::Deleted
    );
    assert!(matches!(vault.load(&target_ref), Err(VaultError::Missing)));
    assert!(!destination.exists());
    assert_eq!(store.load_draft().unwrap().unwrap().document.title, MARKER);
    assert!(vault.load(&source.references[0]).is_ok());
    println!(
        "PASS native portable key separation and exact-target deletion with other profile/key preserved"
    );
}

#[test]
#[ignore = "real Keychain and owned-child SIGKILL at M2 recovery boundaries"]
fn native_m2_recovery_crashes() {
    opt_in();
    let _no_ui = SecKeychain::disable_user_interaction().unwrap();
    let vault = OsDatabaseKeyVault::new();
    for point in [
        "restore-old-moved",
        "restore-promoted",
        "rollback-safety-removed",
        "safety-delete-renamed",
        "delete-intent",
        "delete-keys-removed",
        "delete-directory-removed",
    ] {
        qualify_m2_recovery_crash(point, &vault);
    }
}

fn qualify_m2_recovery_crash(point: &str, vault: &OsDatabaseKeyVault) {
    let mut profile = NativeProfile::new();
    let store = profile.initialize();
    let passphrase = BackupPassphrase::new("M2 disposable recovery phrase".into()).unwrap();
    let backup = store
        .create_portable_backup(&passphrase, "0.0.0-dev")
        .unwrap();
    let external = profile.temporary.path().join("preserved.ort-backup");
    fs::write(&external, &backup).unwrap();
    let mut edited = store.load_draft().unwrap().unwrap().document;
    edited.title = "M2 later synthetic draft".into();
    store.save_draft(1, &edited).unwrap();
    store
        .stage_portable_restore(&backup, &passphrase, CHANNEL, vault)
        .unwrap();
    let parent = profile.root().parent().unwrap().to_path_buf();
    profile.references.push(
        read_manifest(
            &parent
                .join(RESTORE_STAGING_DIRECTORY)
                .join(MANIFEST_FILENAME),
            CHANNEL,
        )
        .unwrap()
        .vault_reference()
        .unwrap(),
    );
    drop(store);
    if point.starts_with("restore-") {
        profile.crash("activate", point);
        let (restored, _) =
            EncryptedStore::open_or_activate_pending_restore(&profile.root(), CHANNEL, vault)
                .unwrap();
        assert_eq!(
            restored.load_draft().unwrap().unwrap().document.title,
            MARKER
        );
        assert_eq!(
            restored.load_latest_published().unwrap().unwrap().revision,
            1
        );
        restored.verify_integrity().unwrap();
    } else {
        let (restored, _) =
            EncryptedStore::open_or_activate_pending_restore(&profile.root(), CHANNEL, vault)
                .unwrap();
        if point == "rollback-safety-removed" {
            restored.stage_safety_rollback(CHANNEL, vault).unwrap();
        }
        drop(restored);
        let action = if point == "rollback-safety-removed" {
            "activate"
        } else if point == "safety-delete-renamed" {
            "delete-safety"
        } else {
            "delete-all"
        };
        profile.crash(action, point);
        let (reopened, _) =
            EncryptedStore::open_or_activate_pending_restore(&profile.root(), CHANNEL, vault)
                .unwrap();
        if action == "delete-all" {
            assert!(reopened.load_draft().unwrap().is_none());
            assert!(reopened.load_latest_published().unwrap().is_none());
            for reference in &profile.references {
                assert!(matches!(vault.load(reference), Err(VaultError::Missing)));
            }
            profile
                .references
                .push(reopened.manifest().vault_reference().unwrap());
        } else {
            let expected = if action == "activate" {
                "M2 later synthetic draft"
            } else {
                MARKER
            };
            assert_eq!(
                reopened.load_draft().unwrap().unwrap().document.title,
                expected
            );
            assert_eq!(
                reopened.load_latest_published().unwrap().unwrap().revision,
                1
            );
        }
        reopened.verify_integrity().unwrap();
    }
    assert_eq!(fs::read(external).unwrap(), backup);
    assert!(!parent.join(RESTORE_MARKER_FILENAME).exists());
    assert!(!parent.join(DELETE_ALL_MARKER_FILENAME).exists());
    println!("PASS M2 owned-child SIGKILL recovery: {point}");
}
