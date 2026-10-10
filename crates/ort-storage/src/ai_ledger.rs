//! Durable AI totals and inactive imported policies, independent of preferences.
use crate::{EncryptedStore, StorageError};
use ort_backup::{MAX_AI_ACCOUNTING_RECORDS, PortableAiGuardrailV1, PortableAiLifetimeV1};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::BTreeMap;
use uuid::Uuid;

pub(crate) fn preserve_lifetime_totals(db: &Connection, profile: &str) -> Result<(), StorageError> {
    db.execute("INSERT INTO ai_lifetime_totals (profile_id, credential_id, currency, counted_micros, unresolved_micros, partial)
        SELECT profile_id, credential_id, currency, COALESCE(SUM(settled_cost_micros),0),
        SUM(CASE WHEN estimate_completeness != 'complete' AND connection_source = 'direct_api' AND status NOT IN ('reserved','dispatching','streaming') THEN reserved_cost_micros ELSE 0 END),
        MAX(CASE WHEN estimate_completeness != 'complete' AND connection_source = 'direct_api' AND status NOT IN ('reserved','dispatching','streaming') THEN 1 ELSE 0 END)
        FROM ai_attempts WHERE profile_id = ?1 GROUP BY profile_id, credential_id, currency
        ON CONFLICT(profile_id, credential_id, currency) DO NOTHING", [profile])
    .map_err(|_| StorageError::Unavailable)?;
    Ok(())
}

/// Runs inside the schema/restore transaction. Legacy totals are authoritative
/// because retained attempt rows may represent only a fraction of lifetime use.
pub(crate) fn migrate_legacy_counters(db: &Connection) -> Result<(), StorageError> {
    let profiles = db
        .prepare("SELECT profile_id FROM profiles")
        .map_err(|_| StorageError::Unavailable)?
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| StorageError::Unavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StorageError::Unavailable)?;
    for profile in &profiles {
        preserve_lifetime_totals(db, profile)?;
    }
    let rows = db.prepare("SELECT profile_id, setting_key, value_json FROM settings WHERE setting_key LIKE 'ai.lifetime.v1.%' OR setting_key LIKE 'ai.lifetime_unknown.v1.%' OR setting_key LIKE 'ai.lifetime_partial.v1.%'")
    .map_err(|_| StorageError::Unavailable)?
        .query_map([], |r| Ok((r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,Vec<u8>>(2)?)))
        .map_err(|_| StorageError::Unavailable)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| StorageError::Unavailable)?;
    for (profile, key, bytes) in &rows {
        let Some(suffix) = key
            .strip_prefix("ai.lifetime.v1.")
            .or_else(|| key.strip_prefix("ai.lifetime_unknown.v1."))
        else {
            continue;
        };
        let (credential, currency) = suffix.rsplit_once('.').ok_or(StorageError::InvalidData)?;
        validate_identity(credential, currency)?;
        let amount: u64 = serde_json::from_slice(bytes).map_err(|_| StorageError::InvalidData)?;
        let amount = i64::try_from(amount).map_err(|_| StorageError::InvalidData)?;
        let unknown = key.starts_with("ai.lifetime_unknown.");
        let column = if unknown {
            "unresolved_micros"
        } else {
            "counted_micros"
        };
        db.execute(&format!("INSERT INTO ai_lifetime_totals (profile_id,credential_id,currency,counted_micros,unresolved_micros,partial) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(profile_id,credential_id,currency) DO UPDATE SET {column} = excluded.{column}, partial = MAX(ai_lifetime_totals.partial,excluded.partial)"), params![profile, credential, currency, if unknown {0} else {amount}, if unknown {amount} else {0}, unknown])
    .map_err(|_| StorageError::Unavailable)?;
    }
    for (profile, key, bytes) in &rows {
        if let Some(credential) = key.strip_prefix("ai.lifetime_partial.v1.") {
            if !serde_json::from_slice::<bool>(bytes).map_err(|_| StorageError::InvalidData)? {
                return Err(StorageError::InvalidData);
            }
            let changed = db.execute("UPDATE ai_lifetime_totals SET partial=1 WHERE profile_id=?1 AND credential_id=?2", params![profile,credential])
    .map_err(|_| StorageError::Unavailable)?;
            if changed == 0 {
                return Err(StorageError::InvalidData);
            }
        }
        db.execute(
            "DELETE FROM settings WHERE profile_id=?1 AND setting_key=?2",
            params![profile, key],
        )
        .map_err(|_| StorageError::Unavailable)?;
    }
    Ok(())
}
fn validate_identity(credential: &str, currency: &str) -> Result<(), StorageError> {
    if !Uuid::parse_str(credential).is_ok_and(|id| id.to_string() == credential)
        || currency.len() != 3
        || !currency.bytes().all(|b| b.is_ascii_uppercase())
    {
        return Err(StorageError::InvalidData);
    }
    Ok(())
}
pub(crate) fn lifetime_counter(
    db: &Connection,
    profile: &str,
    credential: &str,
    currency: &str,
    unknown: bool,
) -> Result<i64, StorageError> {
    let column = if unknown {
        "unresolved_micros"
    } else {
        "counted_micros"
    };
    db.query_row(
        &format!("SELECT {column} FROM ai_lifetime_totals WHERE profile_id=?1 AND credential_id=?2 AND currency=?3"),
        params![profile, credential, currency], |row| row.get(0),
    ).optional().map(|value| value.unwrap_or(0))
    .map_err(|_|StorageError::Unavailable)
}
pub(crate) fn lifetime_cost(
    db: &Connection,
    profile: &str,
    credential: &str,
    currency: &str,
) -> Result<i64, StorageError> {
    lifetime_counter(db, profile, credential, currency, false)
}
pub(crate) fn all_lifetime_counter(
    db: &Connection,
    profile: &str,
    currency: &str,
    unknown: bool,
) -> Result<i64, StorageError> {
    let column = if unknown {
        "unresolved_micros"
    } else {
        "counted_micros"
    };
    db.query_row(
        &format!("SELECT COALESCE(SUM({column}),0) FROM ai_lifetime_totals WHERE profile_id=?1 AND currency=?2"),
        params![profile,currency], |row| row.get(0),
    )
    .map_err(|_|StorageError::Unavailable)
}
/// Reserve ledger space before dispatch so settlement cannot exceed export limits.
pub(crate) fn ensure_counter(
    db: &Connection,
    profile: &str,
    credential: &str,
    currency: &str,
) -> Result<(), StorageError> {
    let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM ai_lifetime_totals WHERE profile_id=?1 AND credential_id=?2 AND currency=?3)",params![profile,credential,currency],|r|r.get(0))
    .map_err(|_|StorageError::Unavailable)?;
    if exists {
        return Ok(());
    }
    let count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM ai_lifetime_totals WHERE profile_id=?1",
            [profile],
            |r| r.get(0),
        )
        .map_err(|_| StorageError::Unavailable)?;
    if usize::try_from(count).map_err(|_| StorageError::InvalidData)? >= MAX_AI_ACCOUNTING_RECORDS {
        return Err(StorageError::InvalidData);
    }
    let known = lifetime_counter(db, profile, credential, currency, false)?;
    let unknown = lifetime_counter(db, profile, credential, currency, true)?;
    db.execute(
        "INSERT INTO ai_lifetime_totals VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT DO NOTHING",
        params![profile, credential, currency, known, unknown, unknown > 0],
    )
    .map_err(|_| StorageError::Unavailable)?;
    Ok(())
}
pub(crate) fn record_lifetime_counter(
    db: &Connection,
    profile: &str,
    credential: &str,
    currency: &str,
    amount: i64,
    unknown: bool,
) -> Result<(), StorageError> {
    ensure_counter(db, profile, credential, currency)?;
    let total = lifetime_counter(db, profile, credential, currency, unknown)?
        .checked_add(amount)
        .ok_or(StorageError::InvalidData)?;
    let column = if unknown {
        "unresolved_micros"
    } else {
        "counted_micros"
    };
    db.execute(&format!("UPDATE ai_lifetime_totals SET {column}=?4,partial=MAX(partial,?5) WHERE profile_id=?1 AND credential_id=?2 AND currency=?3"),params![profile,credential,currency,total,unknown])
    .map_err(|_|StorageError::Unavailable)?;
    Ok(())
}
pub(crate) fn read_totals(
    db: &Connection,
    profile: &str,
) -> Result<Vec<PortableAiLifetimeV1>, StorageError> {
    let sql = "SELECT credential_id,currency,counted_micros,unresolved_micros,partial
        FROM ai_lifetime_totals WHERE profile_id=?1 ORDER BY credential_id,currency";
    let mut statement = db.prepare(sql).map_err(|_| StorageError::Unavailable)?;
    statement
        .query_map([profile], |row| {
            Ok(PortableAiLifetimeV1 {
                credential_id: row.get(0)?,
                currency: row.get(1)?,
                counted_micros: read_u64(row, 2)?,
                unresolved_micros: read_u64(row, 3)?,
                partial: row.get(4)?,
            })
        })
        .map_err(|_| StorageError::Unavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StorageError::Unavailable)
}
pub(crate) fn read_policies(
    db: &Connection,
    profile: &str,
) -> Result<Vec<PortableAiGuardrailV1>, StorageError> {
    let sql = "SELECT credential_id,period,currency,time_zone,limit_micros,activated_at_unix_ms,
        period_start_unix_ms,period_end_unix_ms,counted_micros,reserved_micros,unresolved_micros,revision
        FROM ai_guardrail_policies WHERE profile_id=?1 ORDER BY credential_id,period";
    let mut statement = db.prepare(sql).map_err(|_| StorageError::Unavailable)?;
    let mut policies = statement
        .query_map([profile], |row| {
            let period = row.get::<_, String>(1)?;
            Ok((
                period,
                PortableAiGuardrailV1 {
                    id: Uuid::now_v7().to_string(),
                    credential_id: row.get(0)?,
                    period: ort_backup::AiGuardrailPeriod::AllTime,
                    currency: row.get(2)?,
                    time_zone: row.get(3)?,
                    limit_micros: read_u64(row, 4)?,
                    activated_at_unix_ms: row.get(5)?,
                    period_start_unix_ms: row.get(6)?,
                    period_end_unix_ms: row.get(7)?,
                    counted_micros: read_u64(row, 8)?,
                    reserved_micros: read_u64(row, 9)?,
                    unresolved_micros: read_u64(row, 10)?,
                    revision: read_u64(row, 11)?,
                },
            ))
        })
        .map_err(|_| StorageError::Unavailable)?
        .map(|row| {
            let (period, mut policy) = row.map_err(|_| StorageError::Unavailable)?;
            policy.period =
                ort_backup::AiGuardrailPeriod::from_db(&period).ok_or(StorageError::InvalidData)?;
            Ok(policy)
        })
        .collect::<Result<Vec<_>, StorageError>>()?;
    policies.extend(read_imported_policies(db, profile)?);
    Ok(policies)
}

fn read_imported_policies(
    db: &Connection,
    profile: &str,
) -> Result<Vec<PortableAiGuardrailV1>, StorageError> {
    db.prepare(
        "SELECT policy_json FROM ai_imported_guardrails WHERE profile_id=?1 ORDER BY import_id",
    )
    .map_err(|_| StorageError::Unavailable)?
    .query_map([profile], |r| r.get::<_, Vec<u8>>(0))
    .map_err(|_| StorageError::Unavailable)?
    .map(|r| {
        serde_json::from_slice(&r.map_err(|_| StorageError::Unavailable)?)
            .map_err(|_| StorageError::InvalidData)
    })
    .collect()
}
pub(crate) fn restore_accounting(
    db: &Connection,
    profile: &str,
    totals: &[PortableAiLifetimeV1],
    policies: &[PortableAiGuardrailV1],
) -> Result<(), StorageError> {
    migrate_legacy_counters(db)?;
    for total in totals {
        db.execute("INSERT INTO ai_lifetime_totals VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(profile_id,credential_id,currency) DO UPDATE SET counted_micros=excluded.counted_micros,unresolved_micros=excluded.unresolved_micros,partial=excluded.partial",params![profile,total.credential_id,total.currency,i64::try_from(total.counted_micros)
    .map_err(|_|StorageError::InvalidData)?,i64::try_from(total.unresolved_micros)
    .map_err(|_|StorageError::InvalidData)?,total.partial])
    .map_err(|_|StorageError::Unavailable)?;
    }
    for policy in policies {
        db.execute(
            "INSERT INTO ai_imported_guardrails VALUES (?1,?2,?3)",
            params![
                profile,
                policy.id,
                serde_json::to_vec(policy).map_err(|_| StorageError::InvalidData)?
            ],
        )
        .map_err(|_| StorageError::Unavailable)?;
    }
    Ok(())
}
impl EncryptedStore {
    /// Lifetime estimated spend, independent of cap resets and history retention.
    /// # Errors
    /// Returns an error for corrupt counters or unavailable storage.
    pub fn ai_lifetime_spend(
        &self,
        credential_id: Uuid,
    ) -> Result<BTreeMap<String, u64>, StorageError> {
        self.lifetime_spend(Some(credential_id))
    }
    /// Profile-wide lifetime estimated spend.
    /// # Errors
    /// Returns an error for corrupt counters or unavailable storage.
    pub fn ai_lifetime_spend_all(&self) -> Result<BTreeMap<String, u64>, StorageError> {
        self.lifetime_spend(None)
    }
    fn lifetime_spend(
        &self,
        credential: Option<Uuid>,
    ) -> Result<BTreeMap<String, u64>, StorageError> {
        let db = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let id = credential.map(|c| c.to_string());
        let sql = "SELECT currency,SUM(counted_micros) FROM ai_lifetime_totals
            WHERE profile_id=?1 AND (?2 IS NULL OR credential_id=?2) GROUP BY currency";
        let mut statement = db.prepare(sql).map_err(|_| StorageError::Unavailable)?;
        statement
            .query_map(params![profile, id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|_| StorageError::Unavailable)?
            .map(|row| {
                let (currency, total) = row.map_err(|_| StorageError::Unavailable)?;
                Ok((
                    currency,
                    u64::try_from(total).map_err(|_| StorageError::InvalidData)?,
                ))
            })
            .collect()
    }

    /// Whether retained lifetime accounting includes unknown costs.
    /// # Errors
    /// Returns an error for unavailable storage.
    pub fn ai_lifetime_spend_is_partial(&self, credential: Uuid) -> Result<bool, StorageError> {
        self.lifetime_partial(Some(credential))
    }
    /// Whether any retained lifetime accounting includes unknown costs.
    /// # Errors
    /// Returns an error for unavailable storage.
    pub fn ai_lifetime_spend_all_is_partial(&self) -> Result<bool, StorageError> {
        self.lifetime_partial(None)
    }
    fn lifetime_partial(&self, credential: Option<Uuid>) -> Result<bool, StorageError> {
        let db = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        db.query_row("SELECT EXISTS(SELECT 1 FROM ai_lifetime_totals WHERE profile_id=?1 AND (?2 IS NULL OR credential_id=?2) AND partial=1)",params![self.manifest.profile_id.to_string(),credential.map(|c|c.to_string())],|r|r.get(0))
    .map_err(|_|StorageError::Unavailable)
    }
    /// Historical caps restored from portable backups. These never authorize dispatch.
    /// # Errors
    /// Returns an error for unavailable or corrupt storage.
    pub fn imported_ai_guardrails(&self) -> Result<Vec<PortableAiGuardrailV1>, StorageError> {
        let db = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        read_imported_policies(&db, &self.manifest.profile_id.to_string())
    }
}

fn read_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

impl EncryptedStore {
    /// Explicitly activates one imported policy as a lifetime cap for a chosen identity.
    /// Existing target policies are never overwritten and historical exposure survives.
    /// # Errors
    /// Rejects active operations, conflicting caps, invalid time or missing imports.
    pub fn bind_imported_ai_guardrail(
        &self,
        import_id: Uuid,
        credential_id: Uuid,
        now: i64,
    ) -> Result<(), StorageError> {
        if now <= 0 {
            return Err(StorageError::InvalidData);
        }
        let mut db = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let busy:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM ai_operations WHERE profile_id=?1 AND status='active') OR EXISTS(SELECT 1 FROM ai_guardrail_policies WHERE profile_id=?1 AND credential_id=?2)",params![profile,credential_id.to_string()],|r|r.get(0))
    .map_err(|_|StorageError::Unavailable)?;
        if busy {
            return Err(StorageError::RevisionConflict);
        }
        let bytes:Vec<u8>=tx.query_row("SELECT policy_json FROM ai_imported_guardrails WHERE profile_id=?1 AND import_id=?2",params![profile,import_id.to_string()],|r|r.get(0)).optional()
    .map_err(|_|StorageError::Unavailable)?.ok_or(StorageError::NotFound)?;
        let policy: PortableAiGuardrailV1 =
            serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidData)?;
        let current = |unknown| {
            if credential_id == crate::ai_activity::AI_GENERAL_CAP_CREDENTIAL_ID {
                all_lifetime_counter(&tx, &profile, &policy.currency, unknown)
            } else {
                lifetime_counter(
                    &tx,
                    &profile,
                    &credential_id.to_string(),
                    &policy.currency,
                    unknown,
                )
            }
        };
        let counted = i64::try_from(policy.counted_micros)
            .map_err(|_| StorageError::InvalidData)?
            .max(current(false)?);
        let unresolved = i64::try_from(
            policy
                .unresolved_micros
                .checked_add(policy.reserved_micros)
                .ok_or(StorageError::InvalidData)?,
        )
        .map_err(|_| StorageError::InvalidData)?
        .max(current(true)?);
        counted
            .checked_add(unresolved)
            .ok_or(StorageError::InvalidData)?;
        let start = now
            .max(policy.activated_at_unix_ms)
            .max(policy.period_start_unix_ms);
        tx.execute("INSERT INTO ai_guardrail_policies (profile_id,credential_id,period,currency,time_zone,limit_micros,activated_at_unix_ms,period_start_unix_ms,period_end_unix_ms,counted_micros,reserved_micros,unresolved_micros,revision) VALUES (?1,?2,'all_time',?3,?4,?5,?6,?6,NULL,?7,0,?8,1)",params![profile,credential_id.to_string(),policy.currency,policy.time_zone,i64::try_from(policy.limit_micros)
    .map_err(|_|StorageError::InvalidData)?,start,counted,unresolved])
    .map_err(|_|StorageError::Unavailable)?;
        tx.execute(
            "DELETE FROM ai_imported_guardrails WHERE profile_id=?1 AND import_id=?2",
            params![profile, import_id.to_string()],
        )
        .map_err(|_| StorageError::Unavailable)?;
        tx.commit().map_err(|_| StorageError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_seven_totals_migrate_atomically_and_keep_partial_exposure_after_retention() {
        let temp = tempfile::tempdir().unwrap();
        let vault = ort_vault::testing::MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        let id = Uuid::now_v7();
        store
            .save_setting(
                &format!("ai.lifetime.v1.{id}.USD"),
                None,
                &serde_json::json!(250),
            )
            .unwrap();
        store
            .save_setting(
                &format!("ai.lifetime_unknown.v1.{id}.USD"),
                None,
                &serde_json::json!(80),
            )
            .unwrap();
        store
            .save_setting(
                &format!("ai.lifetime_partial.v1.{id}"),
                None,
                &serde_json::json!(true),
            )
            .unwrap();
        {
            let db = store.connection.lock().unwrap();
            db.execute_batch("DROP TABLE ai_lifetime_totals; DROP TABLE ai_imported_guardrails; DROP INDEX ai_attempts_operation_order; DELETE FROM schema_migrations WHERE version=8;").unwrap();
            crate::migrations::migrate_schema(&db).unwrap();
            crate::migrations::verify_schema(&db).unwrap();
            let plan=db.prepare("EXPLAIN QUERY PLAN SELECT COUNT(*) FROM ai_attempts previous WHERE previous.operation_id = 'fixture' AND previous.started_at_unix_ms <= 1").unwrap().query_map([],|r|r.get::<_,String>(3)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
            assert!(
                plan.iter()
                    .any(|line| line.contains("ai_attempts_operation_order"))
            );
            assert!(plan.iter().all(|line| !line.contains("SCAN previous")));
        }
        assert_eq!(store.ai_lifetime_spend(id).unwrap()["USD"], 250);
        assert!(store.ai_lifetime_spend_is_partial(id).unwrap());
        assert!(
            store
                .load_setting(&format!("ai.lifetime.v1.{id}.USD"))
                .unwrap()
                .is_none()
        );
        store.clear_ai_activity(0, 100).unwrap();
        drop(store);
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        assert_eq!(store.ai_lifetime_spend_all().unwrap()["USD"], 250);
        assert!(store.ai_lifetime_spend_all_is_partial().unwrap());
    }
    #[test]
    fn malformed_legacy_totals_roll_back_the_schema_receipt_and_all_conversions() {
        let temp = tempfile::tempdir().unwrap();
        let vault = ort_vault::testing::MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        let id = Uuid::now_v7();
        let key = format!("ai.lifetime.v1.{id}.USD");
        store
            .save_setting(&key, None, &serde_json::json!("corrupt"))
            .unwrap();
        let db = store.connection.lock().unwrap();
        db.execute_batch("DROP TABLE ai_lifetime_totals; DROP TABLE ai_imported_guardrails; DROP INDEX ai_attempts_operation_order; DELETE FROM schema_migrations WHERE version=8;").unwrap();
        assert_eq!(
            crate::migrations::migrate_schema(&db),
            Err(StorageError::InvalidData)
        );
        let tables: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE name='ai_lifetime_totals'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
        let version: i64 = db
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, 7);
        let bytes: Vec<u8> = db
            .query_row(
                "SELECT value_json FROM settings WHERE setting_key=?1",
                [key],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            serde_json::json!("corrupt")
        );
    }
}
