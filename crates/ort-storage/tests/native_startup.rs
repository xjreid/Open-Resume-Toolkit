//! Run as a separate test executable so `SQLCipher` is not already initialized.
//! Stage labels contain no content, keys, paths, or native error messages.
use ort_domain::ResumeDocument;
use ort_storage::EncryptedStore;
use ort_vault::testing::MemoryDatabaseKeyVault;
use rusqlite::Connection;
use tempfile::TempDir;

#[test]
fn native_cipher_startup_and_encrypted_profile_round_trip() {
    eprintln!("native-startup: opening empty in-memory SQLCipher connection");
    let connection = Connection::open_in_memory().expect("initialize native SQLCipher");
    eprintln!("native-startup: checking native cipher availability");
    let version: String = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .unwrap();
    assert!(!version.trim().is_empty());
    eprintln!("native-startup: verifying raw native logging is compiled out");
    // A runtime request cannot reactivate the allocation-recursive Windows
    // logger. Failure here means Cargo's native build policy was not applied.
    let log_status: String = connection
        .query_row("PRAGMA cipher_log = 'stderr'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(log_status, "1", "native logging must be unavailable");
    drop(connection);

    eprintln!("native-startup: allocating synthetic profile and memory vault");
    let root = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    eprintln!("native-startup: opening encrypted profile");
    let store = EncryptedStore::open_or_initialize(root.path(), "test", &vault).unwrap();
    eprintln!("native-startup: writing and publishing synthetic draft");
    let original = store
        .create_draft(&ResumeDocument::empty("Synthetic startup probe"))
        .unwrap();
    store.publish_draft(original.revision).unwrap();
    eprintln!("native-startup: verifying integrity and closing profile");
    store.verify_integrity().unwrap();
    drop(store);
    eprintln!("native-startup: reopening encrypted profile");
    let reopened = EncryptedStore::open_or_initialize(root.path(), "test", &vault).unwrap();
    assert_eq!(
        reopened.load_draft().unwrap().unwrap().document,
        original.document
    );
    assert_eq!(
        reopened.load_latest_published().unwrap().unwrap().document,
        original.document
    );
    drop(reopened);
    eprintln!("native-startup: complete");
}
