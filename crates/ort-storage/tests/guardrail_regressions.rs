use ort_ai::{OperationType, Price, PriceCategory, Provider, Usage};
use ort_backup::BackupPassphrase;
use ort_domain::ResumeDocument;
use ort_storage::{
    EncryptedStore, StorageError,
    ai_activity::{
        AI_GENERAL_CAP_CREDENTIAL_ID, AiAttemptPreflight, AiAttemptSettlement, AiCapPolicy,
        AiPeriod, AiTerminalStatus,
    },
};
use ort_vault::testing::MemoryDatabaseKeyVault;
use uuid::Uuid;

fn attempt(credential_id: Uuid, started: i64, cost: u64) -> AiAttemptPreflight {
    AiAttemptPreflight {
        operation_id: Uuid::now_v7(),
        attempt_id: Uuid::now_v7(),
        operation_type: OperationType::CredentialTest,
        provider: Provider::OpenAi,
        credential_id,
        requested_model: "fixture-model".into(),
        preset_version: "v1".into(),
        catalog_id: "a".repeat(64),
        catalog_effective_from: "2026-09-13T00:00:00Z".into(),
        pricing_components: vec![Price {
            category: PriceCategory::Input,
            micros_per_million: 1,
        }],
        started_at_unix_ms: started,
        estimated_input_tokens: 100,
        maximum_cost_micros: cost,
        currency: "USD".into(),
        retry_of: None,
    }
}
fn cap(credential_id: Uuid) -> AiCapPolicy {
    AiCapPolicy {
        credential_id,
        period: AiPeriod::AllTime,
        currency: "USD".into(),
        time_zone: "UTC".into(),
        limit_micros: 100,
        activated_at_unix_ms: 10_000,
        period_start_unix_ms: 10_000,
        period_end_unix_ms: None,
        expected_revision: None,
    }
}
#[test]
fn clock_rollback_cannot_bypass_key_or_general_caps_and_resets_are_monotonic() {
    for identity in [Uuid::from_u128(42), AI_GENERAL_CAP_CREDENTIAL_ID] {
        let temp = tempfile::tempdir().unwrap();
        let store =
            EncryptedStore::open_or_initialize(temp.path(), "test", &MemoryDatabaseKeyVault::new())
                .unwrap();
        store.save_ai_unified_cap(&cap(identity)).unwrap();
        assert_eq!(
            store.reserve_ai_attempt(&attempt(Uuid::from_u128(42), 9_999, 1)),
            Err(StorageError::InvalidData)
        );
        assert_eq!(
            store.reserve_ai_attempt(&attempt(Uuid::from_u128(42), 10_001, 101)),
            Err(StorageError::InvalidData)
        );
        store
            .reset_ai_cap(identity, AiPeriod::AllTime, 5_000)
            .unwrap();
        assert_eq!(
            store.ai_cap_policies(identity).unwrap()[0].activated_at_unix_ms,
            10_000
        );
        assert_eq!(
            store.reserve_ai_attempt(&attempt(Uuid::from_u128(42), 9_999, 1)),
            Err(StorageError::InvalidData)
        );
    }
}
#[test]
fn backup_preserves_caps_as_inactive_and_explicit_binding_preserves_exposure() {
    let temp = tempfile::tempdir().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let source =
        EncryptedStore::open_or_initialize(&temp.path().join("source"), "test", &vault).unwrap();
    let key = Uuid::now_v7();
    source.save_ai_unified_cap(&cap(key)).unwrap();
    source
        .save_ai_unified_cap(&cap(AI_GENERAL_CAP_CREDENTIAL_ID))
        .unwrap();
    let request = attempt(key, 10_001, 70);
    source.reserve_ai_attempt(&request).unwrap();
    source.mark_ai_dispatching(request.attempt_id).unwrap();
    let pass = BackupPassphrase::new("synthetic review passphrase".into()).unwrap();
    let bytes = source.create_portable_backup(&pass, "0.0.0-dev").unwrap();
    let dest =
        EncryptedStore::open_or_initialize(&temp.path().join("dest"), "test", &vault).unwrap();
    dest.restore_portable_backup(&bytes, &pass).unwrap();
    assert!(
        dest.ai_cap_policies(AI_GENERAL_CAP_CREDENTIAL_ID)
            .unwrap()
            .is_empty()
    );
    assert!(dest.ai_cap_policies(key).unwrap().is_empty());
    let policies = dest.imported_ai_guardrails().unwrap();
    assert_eq!(policies.len(), 2);
    assert!(policies.iter().all(|policy| policy.reserved_micros == 70));
    // Recovery settles the imported in-flight attempt conservatively first.
    dest.recover_ai_attempts(20_000).unwrap();
    let general = policies
        .iter()
        .find(|policy| policy.credential_id == AI_GENERAL_CAP_CREDENTIAL_ID.to_string())
        .unwrap();
    dest.bind_imported_ai_guardrail(
        Uuid::parse_str(&general.id).unwrap(),
        AI_GENERAL_CAP_CREDENTIAL_ID,
        20_001,
    )
    .unwrap();
    let restored = dest.ai_cap_policies(AI_GENERAL_CAP_CREDENTIAL_ID).unwrap();
    assert_eq!(restored[0].reserved_micros, 0);
    assert_eq!(restored[0].unresolved_micros, 70);
    assert_eq!(
        dest.reserve_ai_attempt(&attempt(Uuid::now_v7(), 20_002, 31)),
        Err(StorageError::InvalidData)
    );
    assert_eq!(dest.imported_ai_guardrails().unwrap().len(), 1);
    let exported = dest.create_portable_backup(&pass, "0.0.0-dev").unwrap();
    let payload = ort_backup::restore_backup(&exported, &pass).unwrap();
    assert_eq!(payload.profile.ai_guardrail_policies.len(), 2);
}
#[test]
fn more_than_128_used_keys_remain_exportable_after_retention_and_restore() {
    let temp = tempfile::tempdir().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        EncryptedStore::open_or_initialize(&temp.path().join("source"), "test", &vault).unwrap();
    for _ in 0..129 {
        let p = attempt(Uuid::now_v7(), 10_000, 1);
        store.reserve_ai_attempt(&p).unwrap();
        store.mark_ai_dispatching(p.attempt_id).unwrap();
        store
            .settle_ai_attempt(&AiAttemptSettlement {
                attempt_id: p.attempt_id,
                status: AiTerminalStatus::Succeeded,
                effective_model: Some("fixture-model".into()),
                usage: Some(Usage {
                    input_tokens: 1,
                    ..Usage::default()
                }),
                settled_cost_micros: Some(1),
                usage_complete: true,
                error_category: None,
                ended_at_unix_ms: 10_001,
                keep_operation_active: false,
            })
            .unwrap();
    }
    store.clear_ai_activity(1, 20_000).unwrap();
    let pass = BackupPassphrase::new("synthetic review passphrase".into()).unwrap();
    let bytes = store.create_portable_backup(&pass, "0.0.0-dev").unwrap();
    let backup = ort_backup::restore_backup(&bytes, &pass).unwrap();
    assert!(backup.profile.settings.is_empty());
    assert_eq!(backup.profile.ai_lifetime_totals.len(), 129);
    assert!(backup.profile.ai_attempts.is_empty());
    let dest =
        EncryptedStore::open_or_initialize(&temp.path().join("dest"), "test", &vault).unwrap();
    dest.restore_portable_backup(&bytes, &pass).unwrap();
    assert_eq!(dest.ai_lifetime_spend_all().unwrap()["USD"], 129);
}
#[test]
fn retired_profile_cannot_create_save_or_publish_in_a_same_revision_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let first =
        EncryptedStore::open_or_initialize(&temp.path().join("first"), "test", &vault).unwrap();
    let retired = first
        .create_draft(&ResumeDocument::empty("Retired private resume"))
        .unwrap();
    let second =
        EncryptedStore::open_or_initialize(&temp.path().join("second"), "test", &vault).unwrap();
    let fresh = second
        .create_draft(&ResumeDocument::empty("Fresh resume"))
        .unwrap();
    let old = first.manifest().profile_id;
    assert_eq!(
        second.save_resume_for_profile(old, Some(retired.revision), &retired.document),
        Err(StorageError::RevisionConflict)
    );
    assert_eq!(
        second.save_resume_for_profile(old, None, &retired.document),
        Err(StorageError::RevisionConflict)
    );
    assert_eq!(
        second.publish_resume_for_profile(old, retired.revision),
        Err(StorageError::RevisionConflict)
    );
    assert_eq!(second.load_draft().unwrap().unwrap(), fresh);
    assert!(second.load_latest_published().unwrap().is_none());
}

#[test]
fn calendar_rollover_remains_fail_closed_after_restart_and_settlement_handles_rollback() {
    let temp = tempfile::tempdir().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let id = Uuid::now_v7();
    let jan = "2026-01-31T23:59:00Z"
        .parse::<jiff::Timestamp>()
        .unwrap()
        .as_millisecond();
    let feb = jan + 120_000;
    let (start, end) =
        ort_storage::ai_activity::ai_calendar_bounds(jan, "UTC", AiPeriod::Month).unwrap();
    store
        .save_ai_cap_policy(&AiCapPolicy {
            credential_id: id,
            period: AiPeriod::Month,
            activated_at_unix_ms: jan,
            period_start_unix_ms: start,
            period_end_unix_ms: end,
            ..cap(id)
        })
        .unwrap();
    let request = attempt(id, feb, 20);
    store.reserve_ai_attempt(&request).unwrap();
    store.mark_ai_dispatching(request.attempt_id).unwrap();
    store
        .settle_ai_attempt(&AiAttemptSettlement {
            attempt_id: request.attempt_id,
            status: AiTerminalStatus::Succeeded,
            effective_model: Some("fixture-model".into()),
            usage: Some(Usage {
                input_tokens: 1,
                ..Usage::default()
            }),
            settled_cost_micros: Some(10),
            usage_complete: true,
            error_category: None,
            ended_at_unix_ms: jan,
            keep_operation_active: false,
        })
        .unwrap();
    assert_eq!(store.ai_cap_policies(id).unwrap()[0].counted_micros, 10);
    drop(store);
    let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    assert_eq!(
        store.reserve_ai_attempt(&attempt(id, jan, 1)),
        Err(StorageError::InvalidData)
    );
    assert_eq!(store.ai_cap_policies(id).unwrap()[0].counted_micros, 10);
}

#[test]
fn retry_cannot_escape_a_cap_by_using_an_earlier_clock() {
    let temp = tempfile::tempdir().unwrap();
    let store =
        EncryptedStore::open_or_initialize(temp.path(), "test", &MemoryDatabaseKeyVault::new())
            .unwrap();
    let id = Uuid::now_v7();
    store.save_ai_unified_cap(&cap(id)).unwrap();
    let first = attempt(id, 10_001, 10);
    store.reserve_ai_attempt(&first).unwrap();
    store.mark_ai_dispatching(first.attempt_id).unwrap();
    store
        .settle_ai_attempt(&AiAttemptSettlement {
            attempt_id: first.attempt_id,
            status: AiTerminalStatus::Failed,
            effective_model: None,
            usage: Some(Usage::default()),
            settled_cost_micros: Some(1),
            usage_complete: true,
            error_category: Some("transient".into()),
            ended_at_unix_ms: 10_002,
            keep_operation_active: true,
        })
        .unwrap();
    let mut retry = attempt(id, 9_999, 10);
    retry.operation_id = first.operation_id;
    retry.retry_of = Some(first.attempt_id);
    assert_eq!(
        store.reserve_ai_attempt(&retry),
        Err(StorageError::InvalidData)
    );
    assert_eq!(store.ai_cap_policies(id).unwrap()[0].reserved_micros, 0);
    retry.started_at_unix_ms = 10_003;
    store.reserve_ai_attempt(&retry).unwrap();
    store.mark_ai_dispatching(retry.attempt_id).unwrap();
    store.recover_ai_attempts(10_004).unwrap();
    let summary = store
        .ai_monitoring_summary(
            0,
            20_000,
            "UTC",
            ort_storage::ai_activity::AiBucketSize::Day,
        )
        .unwrap();
    assert_eq!(summary.recent_failures[0].call_number, 2);
    assert_eq!(store.ai_cap_policies(id).unwrap()[0].counted_micros, 1);
}
