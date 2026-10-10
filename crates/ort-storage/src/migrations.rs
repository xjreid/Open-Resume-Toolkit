//! Ordered, checksum-verified schema migrations. Data conversion commits with its receipt.
use crate::{StorageError, now_string, verify_integrity};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
pub(super) const SCHEMA_VERSION: i64 = 8;

pub(super) const MIGRATION_V1_SQL: &str = "CREATE TABLE schema_migrations (
         version INTEGER PRIMARY KEY,
         checksum_sha256 TEXT NOT NULL,
         minimum_app_version TEXT NOT NULL,
         estimated_disk_bytes INTEGER NOT NULL CHECK (estimated_disk_bytes >= 0),
         requires_safety_copy INTEGER NOT NULL CHECK (requires_safety_copy IN (0, 1)),
         applied_at TEXT NOT NULL
     ) STRICT;
     CREATE TABLE app_metadata (
         metadata_key TEXT PRIMARY KEY,
         metadata_value TEXT NOT NULL
     ) STRICT;
     CREATE TABLE profiles (
         profile_id TEXT PRIMARY KEY,
         revision INTEGER NOT NULL CHECK (revision >= 1),
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
     ) STRICT;
     CREATE TABLE resume_drafts (
         profile_id TEXT PRIMARY KEY REFERENCES profiles(profile_id) ON DELETE CASCADE,
         revision INTEGER NOT NULL CHECK (revision >= 1),
         schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
         document_json BLOB NOT NULL,
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
     ) STRICT;
     CREATE TABLE published_resumes (
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         published_revision INTEGER NOT NULL CHECK (published_revision >= 1),
         draft_revision INTEGER NOT NULL CHECK (draft_revision >= 1),
         schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
         document_json BLOB NOT NULL,
         published_at TEXT NOT NULL,
         PRIMARY KEY (profile_id, published_revision)
     ) STRICT;
     CREATE TABLE settings (
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         setting_key TEXT NOT NULL,
         revision INTEGER NOT NULL CHECK (revision >= 1),
         value_json BLOB NOT NULL,
         updated_at TEXT NOT NULL,
         PRIMARY KEY (profile_id, setting_key)
     ) STRICT;
     CREATE TABLE diagnostic_events (
         event_id TEXT PRIMARY KEY,
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         event_code TEXT NOT NULL,
         severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error')),
         safe_context_json BLOB NOT NULL,
         created_at TEXT NOT NULL
     ) STRICT;";
pub(super) const MIGRATION_V2_SQL: &str = "CREATE TABLE render_manifests (
         manifest_id TEXT PRIMARY KEY,
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         source TEXT NOT NULL CHECK (source IN ('saved_draft', 'published_snapshot')),
         source_revision INTEGER NOT NULL CHECK (source_revision >= 1),
         generated_at_unix_ms INTEGER NOT NULL CHECK (generated_at_unix_ms >= 1),
         last_generated_at_unix_ms INTEGER NOT NULL CHECK (last_generated_at_unix_ms >= generated_at_unix_ms),
         render_count INTEGER NOT NULL CHECK (render_count >= 1),
         document_sha256 TEXT NOT NULL,
         document_schema_version INTEGER NOT NULL CHECK (document_schema_version >= 1),
         pdf_sha256 TEXT NOT NULL,
         renderer_version TEXT NOT NULL,
         template_id TEXT NOT NULL,
         template_sha256 TEXT NOT NULL,
         font_bundle_id TEXT NOT NULL,
         font_bundle_sha256 TEXT NOT NULL,
         page_count INTEGER NOT NULL CHECK (page_count >= 1),
         byte_count INTEGER NOT NULL CHECK (byte_count >= 1),
         UNIQUE (profile_id, source, source_revision, pdf_sha256)
     ) STRICT;
     CREATE INDEX render_manifests_recent
         ON render_manifests (profile_id, last_generated_at_unix_ms DESC, manifest_id DESC);";
pub(super) const MIGRATION_V3_SQL: &str = "CREATE TABLE ai_operations (
         operation_id TEXT PRIMARY KEY,
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         operation_type TEXT NOT NULL CHECK (operation_type IN ('tailor_resume', 'refine_resume', 'cover_letter', 'answer_question', 'import_mapping', 'credential_test')),
         started_at_unix_ms INTEGER NOT NULL CHECK (started_at_unix_ms >= 1),
         ended_at_unix_ms INTEGER CHECK (ended_at_unix_ms IS NULL OR ended_at_unix_ms >= started_at_unix_ms),
         status TEXT NOT NULL CHECK (status IN ('active', 'succeeded', 'failed', 'cancelled', 'outcome_unknown')),
         cancelled INTEGER NOT NULL CHECK (cancelled IN (0, 1))
     ) STRICT;
     CREATE UNIQUE INDEX ai_one_active_operation
         ON ai_operations (profile_id) WHERE status = 'active';
     CREATE TABLE ai_attempts (
         attempt_id TEXT PRIMARY KEY,
         operation_id TEXT NOT NULL REFERENCES ai_operations(operation_id) ON DELETE CASCADE,
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         provider TEXT NOT NULL CHECK (provider IN ('openai', 'anthropic', 'gemini')),
         credential_id TEXT NOT NULL,
         requested_model TEXT NOT NULL,
         effective_model TEXT,
         preset_version TEXT NOT NULL,
         catalog_id TEXT NOT NULL,
         started_at_unix_ms INTEGER NOT NULL CHECK (started_at_unix_ms >= 1),
         ended_at_unix_ms INTEGER CHECK (ended_at_unix_ms IS NULL OR ended_at_unix_ms >= started_at_unix_ms),
         status TEXT NOT NULL CHECK (status IN ('reserved', 'dispatching', 'streaming', 'succeeded', 'failed', 'cancelled', 'outcome_unknown')),
         retry_of TEXT REFERENCES ai_attempts(attempt_id),
         usage_json BLOB,
         usage_complete INTEGER NOT NULL CHECK (usage_complete IN (0, 1)),
         estimated_input_tokens INTEGER NOT NULL CHECK (estimated_input_tokens >= 0),
         reserved_cost_micros INTEGER NOT NULL CHECK (reserved_cost_micros >= 0),
         settled_cost_micros INTEGER CHECK (settled_cost_micros IS NULL OR settled_cost_micros >= 0),
         currency TEXT NOT NULL CHECK (length(currency) = 3),
         estimate_completeness TEXT NOT NULL CHECK (estimate_completeness IN ('complete', 'partial', 'unavailable')),
         error_category TEXT CHECK (error_category IS NULL OR error_category IN ('authentication', 'rate_limit', 'transient', 'safety', 'invalid_output', 'timeout', 'cancelled', 'provider'))
     ) STRICT;
     CREATE INDEX ai_attempts_monitoring
         ON ai_attempts (profile_id, started_at_unix_ms, provider, status);
     CREATE TABLE ai_guardrail_policies (
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         credential_id TEXT NOT NULL,
         period TEXT NOT NULL CHECK (period IN ('week', 'month', 'year', 'all_time')),
         currency TEXT NOT NULL CHECK (length(currency) = 3),
         time_zone TEXT NOT NULL,
         limit_micros INTEGER NOT NULL CHECK (limit_micros > 0),
         activated_at_unix_ms INTEGER NOT NULL CHECK (activated_at_unix_ms >= 1),
         period_start_unix_ms INTEGER NOT NULL CHECK (period_start_unix_ms >= 1),
         period_end_unix_ms INTEGER CHECK (period_end_unix_ms IS NULL OR period_end_unix_ms > period_start_unix_ms),
         counted_micros INTEGER NOT NULL CHECK (counted_micros >= 0),
         reserved_micros INTEGER NOT NULL CHECK (reserved_micros >= 0),
         unresolved_micros INTEGER NOT NULL CHECK (unresolved_micros >= 0),
         revision INTEGER NOT NULL CHECK (revision >= 1),
         PRIMARY KEY (profile_id, credential_id, period)
     ) STRICT;";
pub(super) const MIGRATION_V4_SQL: &str =
    "ALTER TABLE ai_attempts ADD COLUMN catalog_effective_from TEXT NOT NULL DEFAULT 'unavailable';
     ALTER TABLE ai_attempts ADD COLUMN pricing_components_json BLOB NOT NULL DEFAULT X'5B5D';";
pub(super) const MIGRATION_V5_SQL: &str = "CREATE TABLE tracker_entries (
         entry_id TEXT PRIMARY KEY,
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         revision INTEGER NOT NULL CHECK (revision >= 1),
         entry_json BLOB NOT NULL,
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
     ) STRICT;
     CREATE INDEX tracker_entries_profile ON tracker_entries (profile_id, updated_at DESC);";

pub(super) const MIGRATION_V6_SQL: &str =
    "ALTER TABLE ai_attempts ADD COLUMN error_details_json BLOB;";
pub(super) const MIGRATION_V7_SQL: &str = "ALTER TABLE ai_attempts ADD COLUMN connection_source TEXT NOT NULL DEFAULT 'direct_api' CHECK(connection_source IN ('direct_api','chatgpt_plan'));
ALTER TABLE ai_attempts ADD COLUMN reasoning TEXT CHECK(reasoning IN ('low','medium','high','xhigh'));
ALTER TABLE ai_attempts ADD COLUMN monetary_cost_tracking TEXT NOT NULL DEFAULT 'estimated' CHECK(monetary_cost_tracking IN ('estimated','not_tracked'));
ALTER TABLE ai_attempts ADD COLUMN reported_retries INTEGER NOT NULL DEFAULT 0 CHECK(reported_retries >= 0);";

pub(super) const MIGRATION_V8_SQL: &str = "CREATE INDEX ai_attempts_operation_order ON ai_attempts (operation_id, started_at_unix_ms, attempt_id);
CREATE TABLE ai_lifetime_totals (
    profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
    credential_id TEXT NOT NULL,
    currency TEXT NOT NULL CHECK(length(currency) = 3),
    counted_micros INTEGER NOT NULL CHECK(counted_micros >= 0),
    unresolved_micros INTEGER NOT NULL CHECK(unresolved_micros >= 0),
    partial INTEGER NOT NULL CHECK(partial IN (0,1)),
    PRIMARY KEY(profile_id, credential_id, currency)
) STRICT;
CREATE TABLE ai_imported_guardrails (
    profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
    import_id TEXT NOT NULL,
    policy_json BLOB NOT NULL,
    PRIMARY KEY(profile_id, import_id)
) STRICT;";

const MIGRATIONS: &[&str] = &[
    MIGRATION_V1_SQL,
    MIGRATION_V2_SQL,
    MIGRATION_V3_SQL,
    MIGRATION_V4_SQL,
    MIGRATION_V5_SQL,
    MIGRATION_V6_SQL,
    MIGRATION_V7_SQL,
    MIGRATION_V8_SQL,
];
fn checksum(sql: &str) -> String {
    hex::encode(Sha256::digest(sql.as_bytes()))
}

pub(super) fn migrate_schema(connection: &Connection) -> Result<(), StorageError> {
    let receipts = load_migration_receipts(connection)?;
    let latest = receipts
        .last()
        .map(|(version, _)| *version)
        .ok_or(StorageError::IntegrityFailure)?;
    if latest > SCHEMA_VERSION {
        return Err(StorageError::NewerSchema);
    }
    verify_migration_receipts(&receipts)?;
    for (version, sql) in MIGRATIONS
        .iter()
        .enumerate()
        .skip(usize::try_from(latest).map_err(|_| StorageError::IntegrityFailure)?)
    {
        let version = i64::try_from(version + 1).map_err(|_| StorageError::IntegrityFailure)?;
        apply_migration(connection, version, sql, &checksum(sql))?;
    }
    Ok(())
}

pub(super) fn apply_migration(
    connection: &Connection,
    version: i64,
    sql: &str,
    checksum: &str,
) -> Result<(), StorageError> {
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(|_| StorageError::Unavailable)?;
    let result = (|| {
        connection
            .execute_batch(sql)
            .map_err(|_| StorageError::Unavailable)?;
        if version == 8 {
            crate::ai_ledger::migrate_legacy_counters(connection)?;
        }
        connection.execute(
            "INSERT INTO schema_migrations (version, checksum_sha256, minimum_app_version, estimated_disk_bytes, requires_safety_copy, applied_at) VALUES (?1, ?2, ?3, 0, 0, ?4)",
            params![version, checksum, "0.0.0-dev", now_string()],
        ).map_err(|_| StorageError::Unavailable)?;
        #[cfg(all(test, target_os = "macos"))]
        if version == 2 {
            crate::native_qualification::crash_checkpoint("migration-before-commit");
        }
        connection
            .execute_batch("COMMIT")
            .map_err(|_| StorageError::Unavailable)?;
        #[cfg(all(test, target_os = "macos"))]
        if version == 2 {
            crate::native_qualification::crash_checkpoint("migration-after-commit");
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = connection.execute_batch("ROLLBACK");
    }
    result
}

pub(super) fn verify_schema(connection: &Connection) -> Result<(), StorageError> {
    let migrations = load_migration_receipts(connection)?;
    let latest = migrations
        .last()
        .map(|(version, _)| *version)
        .ok_or(StorageError::IntegrityFailure)?;
    if latest > SCHEMA_VERSION {
        return Err(StorageError::NewerSchema);
    }
    if latest != SCHEMA_VERSION {
        return Err(StorageError::IntegrityFailure);
    }
    verify_migration_receipts(&migrations)?;
    verify_integrity(connection)
}

pub(super) fn load_migration_receipts(
    connection: &Connection,
) -> Result<Vec<(i64, String)>, StorageError> {
    let mut statement = connection
        .prepare("SELECT version, checksum_sha256 FROM schema_migrations ORDER BY version")
        .map_err(|_| StorageError::IntegrityFailure)?;
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|_| StorageError::IntegrityFailure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StorageError::IntegrityFailure)
}

pub(super) fn verify_migration_receipts(migrations: &[(i64, String)]) -> Result<(), StorageError> {
    let expected: Vec<_> = MIGRATIONS
        .iter()
        .enumerate()
        .map(|(index, sql)| {
            (
                i64::try_from(index + 1).expect("bounded migration count"),
                checksum(sql),
            )
        })
        .collect();
    if migrations.len() > expected.len()
        || migrations.iter().zip(expected).any(
            |((version, checksum), (expected_version, expected_checksum))| {
                *version != expected_version || *checksum != expected_checksum
            },
        )
    {
        return Err(StorageError::IntegrityFailure);
    }
    Ok(())
}

pub(super) fn migration_v1_checksum() -> String {
    checksum(MIGRATION_V1_SQL)
}
