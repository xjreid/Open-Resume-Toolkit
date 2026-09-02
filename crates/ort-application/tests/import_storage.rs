//! Synthetic end-to-end core proof. The in-memory vault deliberately avoids
//! native Keychain/Credential Manager access; no app profile is opened.
use ort_application::import_review::{ImportReview, ReviewDecision, TextTarget};
use ort_documents::{
    import::{ImportProposal, InputFormat, ValidatedExtraction},
    render_plain_text,
};
use ort_domain::{ResumeDocument, VersionedResumeResponse};
use ort_storage::{EncryptedStore, StorageError, VersionedResume};
use ort_vault::testing::MemoryDatabaseKeyVault;
use serde_json::json;
use tempfile::TempDir;

fn versioned(value: VersionedResume) -> VersionedResumeResponse {
    VersionedResumeResponse {
        revision: value.revision,
        document: value.document,
    }
}

fn review(base: VersionedResumeResponse) -> ImportReview {
    let wire = serde_json::to_vec(&json!({"version":1,"format":"docx","pageCount":1,
        "blocks":[{"page":1,"kind":"paragraph","text":"Synthetic reviewed contribution — 示例"}]}))
    .unwrap();
    let source = ValidatedExtraction::decode(&wire, InputFormat::Docx).unwrap();
    let mut review = ImportReview::new(base, ImportProposal::map(source)).unwrap();
    review
        .decide(
            0,
            ReviewDecision::Text {
                text: "Synthetic reviewed contribution — 示例".to_owned(),
                is_bullet: false,
                target: TextTarget::NewSection("Projects".to_owned()),
            },
        )
        .unwrap();
    review
}

#[test]
fn confirmed_candidate_saves_once_without_changing_published_snapshot_and_survives_restart() {
    eprintln!("import-round-trip: allocating synthetic profile and memory vault");
    let root = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    eprintln!("import-round-trip: opening encrypted profile");
    let store = EncryptedStore::open_or_initialize(root.path(), "test", &vault).unwrap();
    eprintln!("import-round-trip: creating and publishing draft");
    let mut document = ResumeDocument::empty("Synthetic import");
    "Synthetic Person".clone_into(&mut document.contact.full_name);
    let original = store.create_draft(&document).unwrap();
    let published = store.publish_draft(original.revision).unwrap();
    let base = versioned(original);
    eprintln!("import-round-trip: mapping and preparing review");
    let review = review(base.clone());
    let candidate = review.prepare(&base).unwrap();
    // Merely constructing/reviewing/preparing a candidate has no side effects.
    assert_eq!(store.load_draft().unwrap().unwrap().document, document);
    let expected = candidate.expected_revision.unwrap();
    eprintln!("import-round-trip: committing candidate and checking replay");
    let saved = store.save_draft(expected, &candidate.document).unwrap();
    assert!(matches!(
        store.save_draft(expected, &candidate.document),
        Err(StorageError::RevisionConflict)
    ));
    assert_eq!(
        store.load_latest_published().unwrap().unwrap().document,
        published.document
    );
    assert_eq!(
        review.proposal().source().blocks()[0].text,
        "Synthetic reviewed contribution — 示例"
    );
    eprintln!("import-round-trip: closing and reopening profile");
    drop(store);
    let reopened = EncryptedStore::open_or_initialize(root.path(), "test", &vault).unwrap();
    let loaded = reopened.load_draft().unwrap().unwrap();
    assert_eq!(loaded.revision, saved.revision);
    assert_eq!(loaded.document, saved.document);
    eprintln!("import-round-trip: rendering saved document");
    assert!(
        render_plain_text(&loaded.document)
            .unwrap()
            .contains("Synthetic reviewed contribution — 示例")
    );
    assert_eq!(
        reopened.load_latest_published().unwrap().unwrap().document,
        document
    );
}

#[test]
fn edit_racing_after_preparation_is_not_overwritten_by_import_commit() {
    eprintln!("import-commit-race: allocating synthetic profile and memory vault");
    let root = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    eprintln!("import-commit-race: opening encrypted profile");
    let store = EncryptedStore::open_or_initialize(root.path(), "test", &vault).unwrap();
    eprintln!("import-commit-race: preparing import and racing a saved edit");
    let document = ResumeDocument::empty("Synthetic concurrent draft");
    let base = versioned(store.create_draft(&document).unwrap());
    let review = review(base.clone());
    let candidate = review.prepare(&base).unwrap();
    let mut edited = base.document.clone();
    "Later user edit".clone_into(&mut edited.contact.full_name);
    store.save_draft(base.revision, &edited).unwrap();
    assert!(matches!(
        store.save_draft(candidate.expected_revision.unwrap(), &candidate.document),
        Err(StorageError::RevisionConflict)
    ));
    assert_eq!(store.load_draft().unwrap().unwrap().document, edited);
    // Failed storage commit leaves the caller-owned review/source intact.
    assert_eq!(review.proposal().items().len(), 1);
}
