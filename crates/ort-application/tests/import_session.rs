use ort_application::{
    import_review::{ReviewDecision, ReviewError, TextTarget},
    import_session::{CommitError, REVIEW_LIFETIME, ReviewOwner, ReviewSessions, SessionError},
};
use ort_documents::import::{ImportProposal, InputFormat, ValidatedExtraction};
use ort_domain::{ResumeDocument, VersionedResumeResponse};
use ort_storage::{EncryptedStore, StorageError};
use ort_vault::testing::MemoryDatabaseKeyVault;
use std::time::Instant;
use tempfile::TempDir;

fn proposal() -> ImportProposal {
    ImportProposal::map(ValidatedExtraction::decode(br#"{"version":1,"format":"docx","pageCount":1,"blocks":[{"page":1,"kind":"paragraph","text":"Synthetic contribution"}]}"#, InputFormat::Docx).unwrap())
}
fn base() -> VersionedResumeResponse {
    VersionedResumeResponse {
        revision: 1,
        document: ResumeDocument::empty("Synthetic review"),
    }
}
fn decision() -> ReviewDecision {
    ReviewDecision::Text {
        text: "Synthetic contribution".into(),
        is_bullet: true,
        target: TextTarget::NewSection("Projects".into()),
    }
}

#[test]
fn owners_tokens_expiry_and_cancellation_cannot_cross_sessions() {
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let other = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let token = sessions.begin(owner, base(), proposal(), now).unwrap();
    assert!(matches!(
        sessions.begin(other, base(), proposal(), now),
        Err(SessionError::Busy)
    ));
    assert!(matches!(
        sessions.read(other, token, now),
        Err(SessionError::Unavailable)
    ));
    assert_eq!(
        sessions.cancel(other, token, now),
        Err(SessionError::Unavailable)
    );
    assert_eq!(
        sessions.replace_choices(
            other,
            token,
            now,
            ort_domain::ImportChoices::decode(br#"{"choices":[{"kind":"reject"}]}"#).unwrap()
        ),
        Err(SessionError::Unavailable)
    );
    assert!(
        sessions
            .read(owner, token, now)
            .unwrap()
            .decision(0)
            .is_none()
    );
    sessions.close_owner(other);
    assert!(sessions.read(owner, token, now).is_ok());
    sessions.cancel(owner, token, now).unwrap();
    let next = sessions.begin(owner, base(), proposal(), now).unwrap();
    assert_eq!(
        sessions.cancel(owner, token, now),
        Err(SessionError::Unavailable)
    );
    assert!(sessions.read(owner, next, now).is_ok());
    assert!(matches!(
        sessions.read(owner, next, now + REVIEW_LIFETIME),
        Err(SessionError::Unavailable)
    ));
    let final_token = sessions
        .begin(owner, base(), proposal(), now + REVIEW_LIFETIME)
        .unwrap();
    sessions.close_owner(owner);
    assert!(matches!(
        sessions.read(owner, final_token, now + REVIEW_LIFETIME),
        Err(SessionError::Unavailable)
    ));
}

#[test]
fn validation_and_storage_failure_preserve_review_but_success_retires_it() {
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let base = base();
    let token = sessions
        .begin(owner, base.clone(), proposal(), now)
        .unwrap();
    let result = sessions.commit::<()>(owner, token, now, &base, |_| {
        panic!("incomplete review reached storage")
    });
    assert_eq!(
        result,
        Err(CommitError::Session(SessionError::Review(
            ReviewError::IncompleteReview
        )))
    );
    sessions
        .decide(owner, token, now, 0, Some(decision()))
        .unwrap();
    assert_eq!(
        sessions.commit(owner, token, now, &base, |_| Err(
            "synthetic storage failure"
        )),
        Err(CommitError::Storage("synthetic storage failure"))
    );
    assert_eq!(
        sessions
            .read(owner, token, now)
            .unwrap()
            .proposal()
            .source()
            .blocks()[0]
            .text,
        "Synthetic contribution"
    );
    let mut stale = base.clone();
    stale.revision += 1;
    assert_eq!(
        sessions.commit::<()>(owner, token, now, &stale, |_| panic!(
            "stale review reached storage"
        )),
        Err(CommitError::Session(SessionError::Review(
            ReviewError::StaleDraft
        )))
    );
    let saved = sessions
        .commit::<()>(owner, token, now, &base, |payload| {
            Ok(VersionedResumeResponse {
                revision: 2,
                document: payload.document.clone(),
            })
        })
        .unwrap();
    assert_eq!(saved.document.sections.len(), 1);
    assert!(matches!(
        sessions.commit::<()>(owner, token, now, &base, |_| panic!("replayed commit")),
        Err(CommitError::Session(SessionError::Unavailable))
    ));
}

#[test]
fn unexpected_success_receipt_retires_review_instead_of_retrying_possible_commit() {
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let base = base();
    let token = sessions
        .begin(owner, base.clone(), proposal(), now)
        .unwrap();
    sessions
        .decide(owner, token, now, 0, Some(decision()))
        .unwrap();
    assert_eq!(
        sessions.commit::<()>(owner, token, now, &base, |_| Ok(base.clone())),
        Err(CommitError::UnexpectedReceipt)
    );
    assert!(matches!(
        sessions.read(owner, token, now),
        Err(SessionError::Unavailable)
    ));
}

#[test]
fn real_encrypted_storage_cas_preserves_racing_edits_and_prevents_replay() {
    let directory = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store = EncryptedStore::open_or_initialize(directory.path(), "test", &vault).unwrap();
    let original = store.create_draft(&base().document).unwrap();
    let base = VersionedResumeResponse {
        revision: original.revision,
        document: original.document,
    };
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let token = sessions
        .begin(owner, base.clone(), proposal(), now)
        .unwrap();
    sessions
        .decide(owner, token, now, 0, Some(decision()))
        .unwrap();
    let mut edited = base.document.clone();
    edited.contact.full_name = "Racing user edit".into();
    let result = sessions.commit(owner, token, now, &base, |payload| {
        store.save_draft(base.revision, &edited).unwrap();
        store
            .save_draft(payload.expected_revision.unwrap(), &payload.document)
            .map(|saved| VersionedResumeResponse {
                revision: saved.revision,
                document: saved.document,
            })
    });
    assert!(matches!(
        result,
        Err(CommitError::Storage(StorageError::RevisionConflict))
    ));
    assert_eq!(store.load_draft().unwrap().unwrap().document, edited);
    assert!(sessions.read(owner, token, now).is_ok());
    sessions.cancel(owner, token, now).unwrap();
    let current = store.load_draft().unwrap().unwrap();
    let current = VersionedResumeResponse {
        revision: current.revision,
        document: current.document,
    };
    let next = sessions
        .begin(owner, current.clone(), proposal(), now)
        .unwrap();
    sessions
        .decide(owner, next, now, 0, Some(decision()))
        .unwrap();
    let saved = sessions
        .commit(owner, next, now, &current, |payload| {
            store
                .save_draft(payload.expected_revision.unwrap(), &payload.document)
                .map(|saved| VersionedResumeResponse {
                    revision: saved.revision,
                    document: saved.document,
                })
        })
        .unwrap();
    assert_eq!(
        store.load_draft().unwrap().unwrap().document,
        saved.document
    );
    assert!(matches!(
        sessions.read(owner, next, now),
        Err(SessionError::Unavailable)
    ));
}

#[test]
fn ambiguous_storage_failure_cannot_reapply_a_committed_candidate() {
    let directory = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store = EncryptedStore::open_or_initialize(directory.path(), "test", &vault).unwrap();
    let original = store.create_draft(&base().document).unwrap();
    let base = VersionedResumeResponse {
        revision: original.revision,
        document: original.document,
    };
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let token = sessions
        .begin(owner, base.clone(), proposal(), now)
        .unwrap();
    sessions
        .decide(owner, token, now, 0, Some(decision()))
        .unwrap();
    let result = sessions.commit(owner, token, now, &base, |payload| {
        store
            .save_draft(payload.expected_revision.unwrap(), &payload.document)
            .unwrap();
        // Simulate failure after the database transaction already committed.
        Err(StorageError::Unavailable)
    });
    assert!(matches!(
        result,
        Err(CommitError::Storage(StorageError::Unavailable))
    ));
    let saved = store.load_draft().unwrap().unwrap();
    let current = VersionedResumeResponse {
        revision: saved.revision,
        document: saved.document,
    };
    assert_eq!(
        sessions.commit::<()>(owner, token, now, &current, |_| panic!(
            "ambiguous commit was replayed"
        )),
        Err(CommitError::Session(SessionError::Review(
            ReviewError::StaleDraft
        )))
    );
    assert_eq!(
        store.load_draft().unwrap().unwrap().revision,
        base.revision + 1
    );
}

#[test]
fn visible_token_round_trip_does_not_grant_another_owner_authority() {
    use ort_application::import_session::ReviewToken;
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let token = sessions.begin(owner, base(), proposal(), now).unwrap();
    let parsed = ReviewToken::parse(&token.identifier()).unwrap();
    assert!(sessions.read(owner, parsed, now).is_ok());
    assert!(sessions.read(ReviewOwner::default(), parsed, now).is_err());
    for value in ["", "../review", "00000000-0000-4000-8000-000000000000"] {
        assert!(ReviewToken::parse(value).is_none());
    }
}

#[test]
fn empty_profile_review_defers_creation_until_explicit_commit() {
    use ort_application::import_review::{ReviewDecision, SectionTarget};
    use ort_application::import_session::{ReviewOwner, ReviewSessions};
    use ort_documents::import::{ImportProposal, InputFormat, ValidatedExtraction};
    use ort_domain::{ResumeDocument, VersionedResumeResponse};
    let base = VersionedResumeResponse {
        revision: 0,
        document: ResumeDocument::empty("My Resume"),
    };
    let extraction=ValidatedExtraction::decode(br#"{"version":1,"format":"docx","pageCount":1,"blocks":[{"page":1,"kind":"heading","text":"Projects"}]}"#,InputFormat::Docx).unwrap();
    let mut sessions = ReviewSessions::default();
    let owner = ReviewOwner::default();
    let now = std::time::Instant::now();
    let token = sessions
        .begin(owner, base.clone(), ImportProposal::map(extraction), now)
        .unwrap();
    sessions
        .decide(
            owner,
            token,
            now,
            0,
            Some(ReviewDecision::Section {
                heading: "Projects".into(),
                target: SectionTarget::New,
            }),
        )
        .unwrap();
    let saved = sessions
        .commit(owner, token, now, &base, |payload| {
            assert_eq!(payload.expected_revision, None);
            Ok::<_, ()>(VersionedResumeResponse {
                revision: 1,
                document: payload.document.clone(),
            })
        })
        .unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.document.sections[0].heading, "Projects");
}

#[test]
fn expired_or_already_retired_review_can_be_dismissed_without_cancelling_a_new_review() {
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let token = sessions.begin(owner, base(), proposal(), now).unwrap();
    let expired = now + REVIEW_LIFETIME;
    sessions.cancel(owner, token, expired).unwrap();
    sessions.cancel(owner, token, expired).unwrap();
    assert!(!sessions.is_active(expired));
    let next = sessions.begin(owner, base(), proposal(), expired).unwrap();
    assert_eq!(
        sessions.cancel(owner, token, expired),
        Err(SessionError::Unavailable)
    );
    assert!(sessions.read(owner, next, expired).is_ok());
    assert!(
        sessions
            .commit::<()>(owner, token, expired, &base(), |_| panic!(
                "retired review reached storage"
            ))
            .is_err()
    );
}

#[test]
fn replacement_is_separate_and_edits_replace_rather_than_merge_the_saved_document() {
    let directory = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store = EncryptedStore::open_or_initialize(directory.path(), "test", &vault).unwrap();
    let mut existing = ResumeDocument::empty("Saved resume");
    existing.contact.full_name = "Saved Person".into();
    let saved = store.create_draft(&existing).unwrap();
    let current = VersionedResumeResponse {
        revision: saved.revision,
        document: saved.document,
    };
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let token = sessions
        .begin(owner, current.clone(), proposal(), now)
        .unwrap();
    let mut imported = sessions
        .read(owner, token, now)
        .unwrap()
        .snapshot(token.identifier())
        .imported_document;
    assert_ne!(imported.document_id, current.document.document_id);
    assert_ne!(
        imported.contact.full_name,
        current.document.contact.full_name
    );
    imported.contact.full_name = "Reviewed Person".into();
    assert_eq!(
        store.load_draft().unwrap().unwrap().document,
        current.document
    );
    let mapped = sessions
        .commit_replacement(owner, token, now, &current, imported.clone(), |payload| {
            let saved = store.save_imported_draft(
                payload.expected_revision.unwrap_or(0),
                &payload.document,
                1,
                0,
            )?;
            Ok::<_, StorageError>(VersionedResumeResponse {
                revision: saved.revision,
                document: saved.document,
            })
        })
        .unwrap();
    assert_eq!(mapped.revision, current.revision + 1);
    assert_eq!(mapped.document, imported);
    assert_eq!(store.load_draft().unwrap().unwrap().document, imported);
    assert!(sessions.read(owner, token, now).is_err());
    assert!(
        sessions
            .commit_replacement::<()>(owner, token, now, &current, imported, |_| panic!(
                "retired session reached storage"
            ))
            .is_err()
    );
}

#[test]
fn replacement_refuses_foreign_expired_invalid_and_stale_candidates_without_storage() {
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let current = base();
    let token = sessions
        .begin(owner, current.clone(), proposal(), now)
        .unwrap();
    let imported = sessions
        .read(owner, token, now)
        .unwrap()
        .snapshot(token.identifier())
        .imported_document;
    let refuse = |_: &ort_domain::SaveResumePayload| -> Result<VersionedResumeResponse, ()> {
        panic!("invalid replacement reached storage")
    };
    assert!(
        sessions
            .commit_replacement(
                ReviewOwner::default(),
                token,
                now,
                &current,
                imported.clone(),
                refuse
            )
            .is_err()
    );
    let mut invalid = imported.clone();
    invalid.document_id = current.document.document_id;
    assert!(
        sessions
            .commit_replacement(owner, token, now, &current, invalid, refuse)
            .is_err()
    );
    let mut invalid = imported.clone();
    invalid.title.clear();
    assert!(
        sessions
            .commit_replacement(owner, token, now, &current, invalid, refuse)
            .is_err()
    );
    let mut stale = current.clone();
    stale.revision += 1;
    assert!(
        sessions
            .commit_replacement(owner, token, now, &stale, imported.clone(), refuse)
            .is_err()
    );
    let mut changed = current.clone();
    changed.document.title = "changed without revision".into();
    assert!(
        sessions
            .commit_replacement(owner, token, now, &changed, imported.clone(), refuse)
            .is_err()
    );
    assert_eq!(
        sessions
            .commit_replacement::<()>(owner, token, now, &current, imported.clone(), |_| Err(())),
        Err(CommitError::Storage(()))
    );
    assert!(sessions.read(owner, token, now).is_ok());
    assert!(
        sessions
            .commit_replacement(
                owner,
                token,
                now + REVIEW_LIFETIME,
                &current,
                imported,
                refuse
            )
            .is_err()
    );
}

#[test]
fn cancelling_an_edited_replacement_keeps_saved_data_and_allows_a_fresh_import() {
    let now = Instant::now();
    let owner = ReviewOwner::default();
    let mut sessions = ReviewSessions::default();
    let current = base();
    let token = sessions
        .begin(owner, current.clone(), proposal(), now)
        .unwrap();
    let imported = sessions
        .read(owner, token, now)
        .unwrap()
        .snapshot(token.identifier())
        .imported_document;
    sessions.cancel(owner, token, now).unwrap();
    assert!(
        sessions
            .commit_replacement::<()>(owner, token, now, &current, imported, |_| panic!(
                "cancelled import reached storage"
            ))
            .is_err()
    );
    assert!(sessions.begin(owner, current, proposal(), now).is_ok());
}
