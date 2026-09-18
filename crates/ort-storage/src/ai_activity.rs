//! Content-free AI accounting inside the encrypted profile. Every pre-dispatch
//! reservation and cap update commits in one `SQLCipher` transaction.

use std::collections::BTreeMap;

use jiff::{Timestamp, ToSpan, civil::Weekday};
use ort_ai::{OperationType, Price, Provider, Usage};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{EncryptedStore, StorageError};

// Durable, content-free lifetime totals use encrypted settings and survive retention and cap resets.
fn lifetime_cost(
    connection: &rusqlite::Connection,
    profile: &str,
    credential: &str,
    currency: &str,
) -> Result<i64, StorageError> {
    lifetime_counter(connection, profile, credential, currency, false)
}
fn lifetime_counter(
    connection: &rusqlite::Connection,
    profile: &str,
    credential: &str,
    currency: &str,
    unknown: bool,
) -> Result<i64, StorageError> {
    let namespace = if unknown {
        "ai.lifetime_unknown.v1"
    } else {
        "ai.lifetime.v1"
    };
    let bytes: Option<Vec<u8>> = connection
        .query_row(
            "SELECT value_json FROM settings WHERE profile_id = ?1 AND setting_key = ?2",
            params![profile, format!("{namespace}.{credential}.{currency}")],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| StorageError::Unavailable)?;
    if let Some(bytes) = bytes {
        let total: u64 = serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidData)?;
        return i64::try_from(total).map_err(|_| StorageError::InvalidData);
    }
    let sql = if unknown {
        "SELECT COALESCE(SUM(reserved_cost_micros), 0) FROM ai_attempts WHERE profile_id = ?1 AND credential_id = ?2 AND currency = ?3 AND estimate_completeness != 'complete' AND status NOT IN ('reserved', 'dispatching', 'streaming')"
    } else {
        "SELECT COALESCE(SUM(settled_cost_micros), 0) FROM ai_attempts WHERE profile_id = ?1 AND credential_id = ?2 AND currency = ?3"
    };
    connection
        .query_row(sql, params![profile, credential, currency], |row| {
            row.get(0)
        })
        .map_err(|_| StorageError::Unavailable)
}
fn record_lifetime_counter(
    connection: &rusqlite::Connection,
    profile: &str,
    credential: &str,
    currency: &str,
    amount: i64,
    unknown: bool,
) -> Result<(), StorageError> {
    let total = lifetime_counter(connection, profile, credential, currency, unknown)?
        .checked_add(amount)
        .ok_or(StorageError::InvalidData)?;
    let namespace = if unknown {
        "ai.lifetime_unknown.v1"
    } else {
        "ai.lifetime.v1"
    };
    connection.execute("INSERT INTO settings (profile_id, setting_key, revision, value_json, updated_at) VALUES (?1, ?2, 1, ?3, CURRENT_TIMESTAMP) ON CONFLICT (profile_id, setting_key) DO UPDATE SET value_json = excluded.value_json, revision = settings.revision + 1, updated_at = excluded.updated_at", params![profile, format!("{namespace}.{credential}.{currency}"), serde_json::to_vec(&total).map_err(|_| StorageError::InvalidData)?]).map_err(|_| StorageError::Unavailable)?;
    Ok(())
}
fn preserve_lifetime_totals(
    connection: &rusqlite::Connection,
    profile: &str,
) -> Result<(), StorageError> {
    connection.execute("INSERT INTO settings (profile_id, setting_key, revision, value_json, updated_at) SELECT profile_id, 'ai.lifetime.v1.' || credential_id || '.' || currency, 1, CAST(CAST(SUM(settled_cost_micros) AS TEXT) AS BLOB), CURRENT_TIMESTAMP FROM ai_attempts WHERE profile_id = ?1 AND settled_cost_micros IS NOT NULL GROUP BY profile_id, credential_id, currency ON CONFLICT (profile_id, setting_key) DO NOTHING", [profile]).map_err(|_| StorageError::Unavailable)?;
    connection.execute("INSERT INTO settings (profile_id, setting_key, revision, value_json, updated_at) SELECT profile_id, 'ai.lifetime_unknown.v1.' || credential_id || '.' || currency, 1, CAST(CAST(SUM(reserved_cost_micros) AS TEXT) AS BLOB), CURRENT_TIMESTAMP FROM ai_attempts WHERE profile_id = ?1 AND estimate_completeness != 'complete' AND status NOT IN ('reserved', 'dispatching', 'streaming') GROUP BY profile_id, credential_id, currency ON CONFLICT (profile_id, setting_key) DO NOTHING", [profile]).map_err(|_| StorageError::Unavailable)?;
    connection.execute("INSERT INTO settings (profile_id, setting_key, revision, value_json, updated_at) SELECT DISTINCT profile_id, 'ai.lifetime_partial.v1.' || credential_id, 1, CAST('true' AS BLOB), CURRENT_TIMESTAMP FROM ai_attempts WHERE profile_id = ?1 AND estimate_completeness != 'complete' AND status NOT IN ('reserved', 'dispatching', 'streaming') ON CONFLICT (profile_id, setting_key) DO NOTHING", [profile]).map_err(|_| StorageError::Unavailable)?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct AiAttemptPreflight {
    pub operation_id: Uuid,
    pub attempt_id: Uuid,
    pub operation_type: OperationType,
    pub provider: Provider,
    pub credential_id: Uuid,
    pub requested_model: String,
    pub preset_version: String,
    pub catalog_id: String,
    pub catalog_effective_from: String,
    pub pricing_components: Vec<Price>,
    pub started_at_unix_ms: i64,
    pub estimated_input_tokens: u64,
    pub maximum_cost_micros: u64,
    pub currency: String,
    pub retry_of: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct AiCapPolicy {
    pub credential_id: Uuid,
    pub period: AiPeriod,
    pub currency: String,
    pub time_zone: String,
    pub limit_micros: u64,
    pub activated_at_unix_ms: i64,
    pub period_start_unix_ms: i64,
    pub period_end_unix_ms: Option<i64>,
    pub expected_revision: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiPeriod {
    Week,
    Month,
    Year,
    AllTime,
}
impl AiPeriod {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
            Self::AllTime => "all_time",
        }
    }

    fn from_db(value: &str) -> Option<Self> {
        match value {
            "week" => Some(Self::Week),
            "month" => Some(Self::Month),
            "year" => Some(Self::Year),
            "all_time" => Some(Self::AllTime),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCapPolicySummary {
    pub credential_id: Uuid,
    pub period: AiPeriod,
    pub currency: String,
    pub time_zone: String,
    pub limit_micros: u64,
    pub activated_at_unix_ms: i64,
    pub period_start_unix_ms: i64,
    pub period_end_unix_ms: Option<i64>,
    pub counted_micros: u64,
    pub reserved_micros: u64,
    pub unresolved_micros: u64,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiTerminalStatus {
    Succeeded,
    Failed,
    Cancelled,
    OutcomeUnknown,
}
impl AiTerminalStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::OutcomeUnknown => "outcome_unknown",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AiAttemptSettlement {
    pub attempt_id: Uuid,
    pub status: AiTerminalStatus,
    pub effective_model: Option<String>,
    pub usage: Option<Usage>,
    pub settled_cost_micros: Option<u64>,
    pub usage_complete: bool,
    pub error_category: Option<String>,
    pub ended_at_unix_ms: i64,
    pub keep_operation_active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMonitoringSummary {
    pub logical_operations: u64,
    pub attempts: u64,
    pub usage: Usage,
    pub estimated_cost_micros: u64,
    pub unresolved_reserved_micros: u64,
    pub currency: Option<String>,
    pub cost_by_currency_micros: BTreeMap<String, u64>,
    pub partial: bool,
    pub unknown_count: u64,
    pub by_provider: BTreeMap<String, u64>,
    #[serde(default)]
    pub by_credential_id: BTreeMap<String, u64>,
    pub by_status: BTreeMap<String, u64>,
    pub by_model: BTreeMap<String, u64>,
    pub by_preset: BTreeMap<String, u64>,
    pub by_operation_type: BTreeMap<String, u64>,
    pub time_buckets: Vec<AiMonitoringBucket>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMonitoringBucket {
    pub label: String,
    pub attempts: u64,
    pub usage: Usage,
    pub cost_by_currency_micros: BTreeMap<String, u64>,
    pub partial: bool,
    pub unknown_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiBucketSize {
    Day,
    Month,
}

fn add_usage(target: &mut Usage, value: Usage) -> Result<(), StorageError> {
    target.input_tokens = target
        .input_tokens
        .checked_add(value.input_tokens)
        .ok_or(StorageError::InvalidData)?;
    target.cached_input_tokens = target
        .cached_input_tokens
        .checked_add(value.cached_input_tokens)
        .ok_or(StorageError::InvalidData)?;
    target.cache_write_tokens = target
        .cache_write_tokens
        .checked_add(value.cache_write_tokens)
        .ok_or(StorageError::InvalidData)?;
    target.output_tokens = target
        .output_tokens
        .checked_add(value.output_tokens)
        .ok_or(StorageError::InvalidData)?;
    target.reasoning_tokens = target
        .reasoning_tokens
        .checked_add(value.reasoning_tokens)
        .ok_or(StorageError::InvalidData)?;
    Ok(())
}

fn monitoring_bucket_label(
    started_at_unix_ms: i64,
    time_zone: &str,
    size: AiBucketSize,
) -> Result<String, StorageError> {
    let local = Timestamp::from_millisecond(started_at_unix_ms)
        .map_err(|_| StorageError::InvalidData)?
        .in_tz(time_zone)
        .map_err(|_| StorageError::InvalidData)?;
    Ok(match size {
        AiBucketSize::Day => format!(
            "{:04}-{:02}-{:02}",
            local.year(),
            local.month(),
            local.day()
        ),
        AiBucketSize::Month => format!("{:04}-{:02}", local.year(), local.month()),
    })
}

fn operation_type(value: OperationType) -> &'static str {
    match value {
        OperationType::TailorResume => "tailor_resume",
        OperationType::RefineResume => "refine_resume",
        OperationType::CoverLetter => "cover_letter",
        OperationType::AnswerQuestion => "answer_question",
        OperationType::ImportMapping => "import_mapping",
        OperationType::CredentialTest => "credential_test",
    }
}

fn valid_preflight(value: &AiAttemptPreflight) -> bool {
    value.started_at_unix_ms > 0
        && value.maximum_cost_micros > 0
        && i64::try_from(value.maximum_cost_micros).is_ok()
        && i64::try_from(value.estimated_input_tokens).is_ok()
        && value.requested_model.len() <= 128
        && !value.requested_model.is_empty()
        && !value.requested_model.chars().any(char::is_control)
        && (1..=128).contains(&value.preset_version.len())
        && (1..=128).contains(&value.catalog_id.len())
        && (1..=64).contains(&value.catalog_effective_from.len())
        && !value.catalog_effective_from.chars().any(char::is_control)
        && !value.pricing_components.is_empty()
        && value.pricing_components.len() <= 5
        && value
            .pricing_components
            .iter()
            .all(|price| price.micros_per_million > 0)
        && !value
            .pricing_components
            .iter()
            .enumerate()
            .any(|(index, price)| {
                value.pricing_components[..index]
                    .iter()
                    .any(|other| other.category == price.category)
            })
        && value.currency.len() == 3
        && value.currency.bytes().all(|byte| byte.is_ascii_uppercase())
        && value.retry_of != Some(value.attempt_id)
}

/// Resolves one calendar guardrail period in its immutable recorded IANA zone.
/// # Errors
/// Rejects invalid timestamps, zones, or unrepresentable calendar boundaries.
pub fn ai_calendar_bounds(
    now_unix_ms: i64,
    time_zone: &str,
    period: AiPeriod,
) -> Result<(i64, Option<i64>), StorageError> {
    let now = Timestamp::from_millisecond(now_unix_ms)
        .map_err(|_| StorageError::InvalidData)?
        .in_tz(time_zone)
        .map_err(|_| StorageError::InvalidData)?;
    let start = match period {
        AiPeriod::Week => now
            .tomorrow()
            .and_then(|value| value.nth_weekday(-1, Weekday::Monday))
            .and_then(|value| value.start_of_day())
            .map_err(|_| StorageError::InvalidData)?,
        AiPeriod::Month => now
            .with()
            .day(1)
            .build()
            .and_then(|value| value.start_of_day())
            .map_err(|_| StorageError::InvalidData)?,
        AiPeriod::Year => now
            .with()
            .month(1)
            .day(1)
            .build()
            .and_then(|value| value.start_of_day())
            .map_err(|_| StorageError::InvalidData)?,
        AiPeriod::AllTime => return Ok((now_unix_ms, None)),
    };
    let end = match period {
        AiPeriod::Week => start.checked_add(7.days()),
        AiPeriod::Month => start.checked_add(1.month()),
        AiPeriod::Year => start.checked_add(1.year()),
        AiPeriod::AllTime => unreachable!(),
    }
    .map_err(|_| StorageError::InvalidData)?;
    Ok((
        start.timestamp().as_millisecond(),
        Some(end.timestamp().as_millisecond()),
    ))
}

impl EncryptedStore {
    /// Moves enabled cap configuration to a replacement credential identity,
    /// always starting the replacement at a zero baseline. The old identity's
    /// policies are removed in the same encrypted transaction.
    /// # Errors
    /// Rejects active operations, identical identities, or invalid boundaries.
    pub fn replace_ai_cap_identity(
        &self,
        old_credential_id: Uuid,
        new_credential_id: Uuid,
        copy_configuration: bool,
        now_unix_ms: i64,
    ) -> Result<(), StorageError> {
        if old_credential_id == new_credential_id || now_unix_ms <= 0 {
            return Err(StorageError::InvalidData);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let active: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM ai_operations WHERE profile_id = ?1 AND status = 'active'",
                [&profile],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::Unavailable)?;
        if active != 0 {
            return Err(StorageError::RevisionConflict);
        }
        if copy_configuration {
            let policies = {
                let mut statement = tx
                    .prepare(
                        "SELECT period, currency, time_zone, limit_micros
                        FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2",
                    )
                    .map_err(|_| StorageError::Unavailable)?;
                statement
                    .query_map(params![profile, old_credential_id.to_string()], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    })
                    .map_err(|_| StorageError::Unavailable)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| StorageError::Unavailable)?
            };
            for (period, currency, time_zone, limit) in policies {
                let period = AiPeriod::from_db(&period).ok_or(StorageError::InvalidData)?;
                let (start, end) = ai_calendar_bounds(now_unix_ms, &time_zone, period)?;
                tx.execute(
                    "INSERT INTO ai_guardrail_policies (profile_id, credential_id, period,
                    currency, time_zone, limit_micros, activated_at_unix_ms,
                    period_start_unix_ms, period_end_unix_ms, counted_micros,
                    reserved_micros, unresolved_micros, revision)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, 0, 0, 1)",
                    params![
                        profile,
                        new_credential_id.to_string(),
                        period.as_str(),
                        currency,
                        time_zone,
                        limit,
                        now_unix_ms,
                        start,
                        end
                    ],
                )
                .map_err(|_| StorageError::Unavailable)?;
            }
        }
        tx.execute(
            "DELETE FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2",
            params![profile, old_credential_id.to_string()],
        )
        .map_err(|_| StorageError::Unavailable)?;
        tx.commit().map_err(|_| StorageError::Unavailable)
    }

    /// Deletes policies tied to an explicitly removed credential identity.
    /// Historical activity and other identities remain untouched.
    /// # Errors
    /// Refuses an active operation or unavailable encrypted storage.
    pub fn remove_ai_cap_identity(&self, credential_id: Uuid) -> Result<(), StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let active: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM ai_operations WHERE profile_id = ?1 AND status = 'active'",
                [&profile],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::Unavailable)?;
        if active != 0 {
            return Err(StorageError::RevisionConflict);
        }
        tx.execute(
            "DELETE FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2",
            params![profile, credential_id.to_string()],
        )
        .map_err(|_| StorageError::Unavailable)?;
        tx.commit().map_err(|_| StorageError::Unavailable)
    }

    /// Returns content-free guardrail state for one credential.
    /// # Errors
    /// Rejects corrupt persisted values or unavailable encrypted storage.
    pub fn ai_cap_policies(
        &self,
        credential_id: Uuid,
    ) -> Result<Vec<AiCapPolicySummary>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let mut statement = connection
            .prepare(
                "SELECT period, currency, time_zone, limit_micros,
            activated_at_unix_ms, period_start_unix_ms, period_end_unix_ms, counted_micros,
            reserved_micros, unresolved_micros, revision FROM ai_guardrail_policies
            WHERE profile_id = ?1 AND credential_id = ?2 ORDER BY period",
            )
            .map_err(|_| StorageError::Unavailable)?;
        statement
            .query_map(params![profile, credential_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                ))
            })
            .map_err(|_| StorageError::Unavailable)?
            .map(|row| {
                let (
                    period,
                    currency,
                    time_zone,
                    limit,
                    activated,
                    start,
                    end,
                    counted,
                    reserved,
                    unresolved,
                    revision,
                ) = row.map_err(|_| StorageError::Unavailable)?;
                Ok(AiCapPolicySummary {
                    credential_id,
                    period: AiPeriod::from_db(&period).ok_or(StorageError::InvalidData)?,
                    currency,
                    time_zone,
                    limit_micros: u64::try_from(limit).map_err(|_| StorageError::InvalidData)?,
                    activated_at_unix_ms: activated,
                    period_start_unix_ms: start,
                    period_end_unix_ms: end,
                    counted_micros: u64::try_from(counted)
                        .map_err(|_| StorageError::InvalidData)?,
                    reserved_micros: u64::try_from(reserved)
                        .map_err(|_| StorageError::InvalidData)?,
                    unresolved_micros: u64::try_from(unresolved)
                        .map_err(|_| StorageError::InvalidData)?,
                    revision: u64::try_from(revision).map_err(|_| StorageError::InvalidData)?,
                })
            })
            .collect()
    }

    /// Disables one cap without affecting history or the other period caps.
    /// # Errors
    /// Rejects disabling while a request is active or when the cap is absent.
    pub fn disable_ai_cap(
        &self,
        credential_id: Uuid,
        period: AiPeriod,
    ) -> Result<(), StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let active: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM ai_operations WHERE profile_id = ?1 AND status = 'active'",
                [&profile],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::Unavailable)?;
        if active != 0 {
            return Err(StorageError::RevisionConflict);
        }
        let changed = tx.execute("DELETE FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2 AND period = ?3",
            params![profile, credential_id.to_string(), period.as_str()]).map_err(|_| StorageError::Unavailable)?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
        tx.commit().map_err(|_| StorageError::Unavailable)
    }

    /// Enables or revises one direct-credential cap. Counters start at zero on
    /// initial activation and are not silently reset on ordinary policy edits.
    /// # Errors
    /// Rejects invalid policy or unavailable encrypted storage.
    pub fn save_ai_cap_policy(&self, policy: &AiCapPolicy) -> Result<(), StorageError> {
        self.save_ai_cap_policy_inner(policy, false)
    }

    /// Saves the one lifetime cap, preserving reset counters on edits and retiring calendar caps.
    /// # Errors
    /// Rejects active requests, stale revisions, invalid policy or unavailable storage.
    pub fn save_ai_unified_cap(&self, policy: &AiCapPolicy) -> Result<(), StorageError> {
        if policy.period != AiPeriod::AllTime {
            return Err(StorageError::InvalidData);
        }
        self.save_ai_cap_policy_inner(policy, true)
    }

    #[allow(clippy::too_many_lines)]
    fn save_ai_cap_policy_inner(
        &self,
        policy: &AiCapPolicy,
        unified: bool,
    ) -> Result<(), StorageError> {
        if policy.limit_micros == 0
            || i64::try_from(policy.limit_micros).is_err()
            || policy.activated_at_unix_ms <= 0
            || policy.period_start_unix_ms <= 0
            || policy
                .period_end_unix_ms
                .is_some_and(|end| end <= policy.period_start_unix_ms)
            || policy.currency.len() != 3
            || !policy
                .currency
                .bytes()
                .all(|byte| byte.is_ascii_uppercase())
            || policy.time_zone.is_empty()
            || policy.time_zone.len() > 128
        {
            return Err(StorageError::InvalidData);
        }
        let limit = i64::try_from(policy.limit_micros).map_err(|_| StorageError::InvalidData)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let connection = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        if unified {
            let active: i64 = connection.query_row("SELECT COUNT(*) FROM ai_operations WHERE profile_id = ?1 AND status = 'active'", [&profile], |row| row.get(0)).map_err(|_| StorageError::Unavailable)?;
            if active != 0 {
                return Err(StorageError::RevisionConflict);
            }
            let conflicting: i64 = connection.query_row("SELECT COUNT(*) FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2 AND currency != ?3", params![profile, policy.credential_id.to_string(), policy.currency], |row| row.get(0)).map_err(|_| StorageError::Unavailable)?;
            if conflicting != 0 {
                return Err(StorageError::InvalidData);
            }
        }
        let existing: Option<(String, String, i64, Option<i64>, i64)> = connection.query_row(
            "SELECT currency, time_zone, period_start_unix_ms, period_end_unix_ms, revision FROM ai_guardrail_policies
             WHERE profile_id = ?1 AND credential_id = ?2 AND period = ?3",
            params![self.manifest.profile_id.to_string(), policy.credential_id.to_string(), policy.period.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).optional().map_err(|_| StorageError::Unavailable)?;
        let is_new = existing.is_none();
        if let Some(existing) = existing {
            if existing.0 != policy.currency
                || existing.1 != policy.time_zone
                || existing.2 != policy.period_start_unix_ms
                || existing.3 != policy.period_end_unix_ms
                || u64::try_from(existing.4).ok() != policy.expected_revision
            {
                return Err(StorageError::RevisionConflict);
            }
        } else if policy.expected_revision.is_some() {
            return Err(StorageError::RevisionConflict);
        }
        connection.execute(
            "INSERT INTO ai_guardrail_policies (profile_id, credential_id, period, currency, time_zone, limit_micros, activated_at_unix_ms, period_start_unix_ms, period_end_unix_ms, counted_micros, reserved_micros, unresolved_micros, revision)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, 0, 0, 1)
             ON CONFLICT (profile_id, credential_id, period) DO UPDATE SET
                limit_micros = excluded.limit_micros,
                revision = ai_guardrail_policies.revision + 1",
            params![self.manifest.profile_id.to_string(), policy.credential_id.to_string(), policy.period.as_str(), policy.currency, policy.time_zone, limit, policy.activated_at_unix_ms, policy.period_start_unix_ms, policy.period_end_unix_ms],
        ).map_err(|_| StorageError::Unavailable)?;
        if unified {
            if is_new {
                let total = lifetime_cost(
                    &connection,
                    &profile,
                    &policy.credential_id.to_string(),
                    &policy.currency,
                )?;
                let unknown = lifetime_counter(
                    &connection,
                    &profile,
                    &policy.credential_id.to_string(),
                    &policy.currency,
                    true,
                )?;
                connection.execute("UPDATE ai_guardrail_policies SET counted_micros = ?1, unresolved_micros = ?4 WHERE profile_id = ?2 AND credential_id = ?3 AND period = 'all_time'", params![total, profile, policy.credential_id.to_string(), unknown]).map_err(|_| StorageError::Unavailable)?;
            }
            connection.execute("UPDATE ai_guardrail_policies SET unresolved_micros = MAX(unresolved_micros, COALESCE((SELECT MAX(unresolved_micros) FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2 AND currency = ?3 AND period != 'all_time'), 0)) WHERE profile_id = ?1 AND credential_id = ?2 AND period = 'all_time'", params![profile, policy.credential_id.to_string(), policy.currency]).map_err(|_| StorageError::Unavailable)?;
            connection.execute("DELETE FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2 AND period != 'all_time'", params![profile, policy.credential_id.to_string()]).map_err(|_| StorageError::Unavailable)?;
        }
        connection.commit().map_err(|_| StorageError::Unavailable)
    }

    /// Atomically starts a logical operation, checks every active cap, reserves
    /// conservative spend, and records the attempt before any network dispatch.
    /// # Errors
    /// `RevisionConflict` means another operation is active; `InvalidData`
    /// includes cap rejection. Neither leaves a dispatchable attempt behind.
    #[allow(clippy::too_many_lines)]
    pub fn reserve_ai_attempt(&self, preflight: &AiAttemptPreflight) -> Result<(), StorageError> {
        if !valid_preflight(preflight) {
            return Err(StorageError::InvalidData);
        }
        let maximum_cost =
            i64::try_from(preflight.maximum_cost_micros).map_err(|_| StorageError::InvalidData)?;
        let input_tokens = i64::try_from(preflight.estimated_input_tokens)
            .map_err(|_| StorageError::InvalidData)?;
        let pricing_components = serde_json::to_vec(&preflight.pricing_components)
            .map_err(|_| StorageError::InvalidData)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let active: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM ai_operations WHERE profile_id = ?1 AND status = 'active'",
                [&profile],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::Unavailable)?;
        if let Some(retry_of) = preflight.retry_of {
            if active != 1 {
                return Err(StorageError::RevisionConflict);
            }
            let retryable: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM ai_attempts previous
                    JOIN ai_operations operation ON operation.operation_id = previous.operation_id
                    WHERE previous.profile_id = ?1 AND previous.attempt_id = ?2
                    AND previous.operation_id = ?3 AND previous.status = 'failed'
                    AND previous.error_category = 'transient' AND operation.status = 'active'
                    AND NOT EXISTS (SELECT 1 FROM ai_attempts retry WHERE retry.retry_of = previous.attempt_id)",
                    params![profile, retry_of.to_string(), preflight.operation_id.to_string()],
                    |row| row.get(0),
                )
                .map_err(|_| StorageError::Unavailable)?;
            if retryable != 1 {
                return Err(StorageError::RevisionConflict);
            }
        } else if active != 0 {
            return Err(StorageError::RevisionConflict);
        }
        let expired = {
            let mut statement = tx.prepare("SELECT credential_id, period, time_zone FROM ai_guardrail_policies
                WHERE profile_id = ?1 AND period_end_unix_ms IS NOT NULL AND period_end_unix_ms <= ?2")
                .map_err(|_| StorageError::Unavailable)?;
            statement
                .query_map(params![profile, preflight.started_at_unix_ms], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|_| StorageError::Unavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| StorageError::Unavailable)?
        };
        for (credential, period, time_zone) in expired {
            let period = AiPeriod::from_db(&period).ok_or(StorageError::InvalidData)?;
            let (start, end) =
                ai_calendar_bounds(preflight.started_at_unix_ms, &time_zone, period)?;
            tx.execute(
                "UPDATE ai_guardrail_policies SET period_start_unix_ms = ?1,
                period_end_unix_ms = ?2, counted_micros = 0, reserved_micros = 0,
                unresolved_micros = 0, revision = revision + 1
                WHERE profile_id = ?3 AND credential_id = ?4 AND period = ?5",
                params![start, end, profile, credential, period.as_str()],
            )
            .map_err(|_| StorageError::Unavailable)?;
        }
        let wrong_currency: i64 = tx.query_row(
            "SELECT COUNT(*) FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2
             AND currency != ?3 AND activated_at_unix_ms <= ?4 AND period_start_unix_ms <= ?4
             AND (period_end_unix_ms IS NULL OR ?4 < period_end_unix_ms)",
            params![profile, preflight.credential_id.to_string(), preflight.currency, preflight.started_at_unix_ms],
            |row| row.get(0),
        ).map_err(|_| StorageError::Unavailable)?;
        if wrong_currency != 0 {
            return Err(StorageError::InvalidData);
        }
        let mut cap_statement = tx.prepare(
            "SELECT limit_micros, counted_micros, reserved_micros, unresolved_micros
             FROM ai_guardrail_policies WHERE profile_id = ?1 AND credential_id = ?2
             AND currency = ?3 AND activated_at_unix_ms <= ?4
             AND period_start_unix_ms <= ?4 AND (period_end_unix_ms IS NULL OR ?4 < period_end_unix_ms)",
        ).map_err(|_| StorageError::Unavailable)?;
        let caps = cap_statement
            .query_map(
                params![
                    profile,
                    preflight.credential_id.to_string(),
                    preflight.currency,
                    preflight.started_at_unix_ms
                ],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .map_err(|_| StorageError::Unavailable)?;
        for cap in caps {
            let (limit, counted, reserved, unresolved) =
                cap.map_err(|_| StorageError::Unavailable)?;
            let total = counted
                .checked_add(reserved)
                .and_then(|value| value.checked_add(unresolved))
                .and_then(|value| value.checked_add(maximum_cost))
                .ok_or(StorageError::InvalidData)?;
            if total > limit {
                return Err(StorageError::InvalidData);
            }
        }
        drop(cap_statement);
        if preflight.retry_of.is_none() {
            tx.execute(
                "INSERT INTO ai_operations (operation_id, profile_id, operation_type, started_at_unix_ms, status, cancelled)
             VALUES (?1, ?2, ?3, ?4, 'active', 0)",
                params![preflight.operation_id.to_string(), profile, operation_type(preflight.operation_type), preflight.started_at_unix_ms],
            ).map_err(|_| StorageError::RevisionConflict)?;
        }
        tx.execute(
            "INSERT INTO ai_attempts (attempt_id, operation_id, profile_id, provider, credential_id, requested_model, preset_version, catalog_id, catalog_effective_from, pricing_components_json, started_at_unix_ms, status, retry_of, usage_complete, estimated_input_tokens, reserved_cost_micros, currency, estimate_completeness)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'reserved', ?12, 0, ?13, ?14, ?15, 'complete')",
            params![preflight.attempt_id.to_string(), preflight.operation_id.to_string(), profile, preflight.provider.as_str(), preflight.credential_id.to_string(), preflight.requested_model, preflight.preset_version, preflight.catalog_id, preflight.catalog_effective_from, pricing_components, preflight.started_at_unix_ms, preflight.retry_of.map(|id| id.to_string()), input_tokens, maximum_cost, preflight.currency],
        ).map_err(|_| StorageError::Unavailable)?;
        tx.execute(
            "UPDATE ai_guardrail_policies SET reserved_micros = reserved_micros + ?1, revision = revision + 1
             WHERE profile_id = ?2 AND credential_id = ?3 AND currency = ?4
             AND activated_at_unix_ms <= ?5 AND period_start_unix_ms <= ?5
             AND (period_end_unix_ms IS NULL OR ?5 < period_end_unix_ms)",
            params![maximum_cost, profile, preflight.credential_id.to_string(), preflight.currency, preflight.started_at_unix_ms],
        ).map_err(|_| StorageError::Unavailable)?;
        tx.commit().map_err(|_| StorageError::Unavailable)
    }

    /// Changes one attempt from reserved to dispatching. No network call may
    /// start unless this durable transition succeeds.
    /// # Errors
    /// Rejects stale or missing attempts.
    pub fn mark_ai_dispatching(&self, attempt_id: Uuid) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let changed = connection.execute(
            "UPDATE ai_attempts SET status = 'dispatching' WHERE attempt_id = ?1 AND profile_id = ?2 AND status = 'reserved'",
            params![attempt_id.to_string(), self.manifest.profile_id.to_string()],
        ).map_err(|_| StorageError::Unavailable)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StorageError::RevisionConflict)
        }
    }

    /// Marks that a successful provider response has begun streaming.
    /// # Errors
    /// Rejects stale, terminal, or missing attempts.
    pub fn mark_ai_streaming(&self, attempt_id: Uuid) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let changed = connection
            .execute(
                "UPDATE ai_attempts SET status = 'streaming' WHERE attempt_id = ?1
                AND profile_id = ?2 AND status = 'dispatching'",
                params![attempt_id.to_string(), self.manifest.profile_id.to_string()],
            )
            .map_err(|_| StorageError::Unavailable)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StorageError::RevisionConflict)
        }
    }

    /// Terminates an active logical operation when a classified retry cannot
    /// begin after its first attempt was settled as transient.
    /// # Errors
    /// Rejects an operation with any active attempt or an invalid status.
    pub fn finish_ai_operation(
        &self,
        operation_id: Uuid,
        status: AiTerminalStatus,
        ended_at_unix_ms: i64,
    ) -> Result<(), StorageError> {
        if ended_at_unix_ms <= 0 {
            return Err(StorageError::InvalidData);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let changed = connection
            .execute(
                "UPDATE ai_operations SET status = ?1, ended_at_unix_ms = ?2,
                cancelled = ?3 WHERE operation_id = ?4 AND profile_id = ?5
                AND status = 'active' AND started_at_unix_ms <= ?2
                AND NOT EXISTS (SELECT 1 FROM ai_attempts WHERE operation_id = ?4
                AND status IN ('reserved', 'dispatching', 'streaming'))",
                params![
                    status.as_str(),
                    ended_at_unix_ms,
                    i64::from(status == AiTerminalStatus::Cancelled),
                    operation_id.to_string(),
                    self.manifest.profile_id.to_string(),
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StorageError::RevisionConflict)
        }
    }

    /// Cancels a reserved, never-dispatched attempt without treating its
    /// preflight reservation as provider spend.
    /// # Errors
    /// Rejects absent or already-dispatched attempts.
    pub fn cancel_reserved_ai_attempt(
        &self,
        attempt_id: Uuid,
        now_unix_ms: i64,
    ) -> Result<(), StorageError> {
        if now_unix_ms <= 0 {
            return Err(StorageError::InvalidData);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let row: Option<(String, String, String, i64, i64)> = tx.query_row(
            "SELECT operation_id, credential_id, currency, reserved_cost_micros, started_at_unix_ms
             FROM ai_attempts WHERE attempt_id = ?1 AND profile_id = ?2 AND status = 'reserved'",
            params![attempt_id.to_string(), profile],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).optional().map_err(|_| StorageError::Unavailable)?;
        let Some((operation, credential, currency, reserved, started)) = row else {
            return Err(StorageError::RevisionConflict);
        };
        if now_unix_ms < started {
            return Err(StorageError::InvalidData);
        }
        tx.execute(
            "UPDATE ai_attempts SET status = 'cancelled', ended_at_unix_ms = ?1,
            settled_cost_micros = 0, usage_complete = 1, error_category = 'cancelled'
            WHERE attempt_id = ?2",
            params![now_unix_ms, attempt_id.to_string()],
        )
        .map_err(|_| StorageError::Unavailable)?;
        tx.execute(
            "UPDATE ai_operations SET status = 'cancelled', cancelled = 1, ended_at_unix_ms = ?1
            WHERE operation_id = ?2 AND status = 'active'",
            params![now_unix_ms, operation],
        )
        .map_err(|_| StorageError::Unavailable)?;
        tx.execute(
            "UPDATE ai_guardrail_policies SET reserved_micros = reserved_micros - ?1,
            revision = revision + 1 WHERE profile_id = ?2 AND credential_id = ?3 AND currency = ?4
            AND activated_at_unix_ms <= ?5 AND period_start_unix_ms <= ?5
            AND (period_end_unix_ms IS NULL OR ?5 < period_end_unix_ms)",
            params![reserved, profile, credential, currency, started],
        )
        .map_err(|_| StorageError::Unavailable)?;
        tx.commit().map_err(|_| StorageError::Unavailable)
    }

    /// Settles an attempt and every applicable guardrail in one encrypted
    /// transaction. Missing or ambiguous usage consumes the reservation as
    /// unresolved; it is never interpreted as zero spend.
    /// # Errors
    /// Rejects duplicate, invalid, or pre-dispatch settlement.
    #[allow(clippy::too_many_lines)]
    pub fn settle_ai_attempt(&self, result: &AiAttemptSettlement) -> Result<(), StorageError> {
        if result.ended_at_unix_ms <= 0
            || result
                .settled_cost_micros
                .is_some_and(|cost| i64::try_from(cost).is_err())
            || result.status == AiTerminalStatus::Succeeded
                && (result.usage.is_none() || result.effective_model.is_none())
            || result.keep_operation_active
                && (result.status != AiTerminalStatus::Failed
                    || result.error_category.as_deref() != Some("transient"))
            || result.error_category.as_deref().is_some_and(|category| {
                !matches!(
                    category,
                    "authentication"
                        | "rate_limit"
                        | "transient"
                        | "safety"
                        | "invalid_output"
                        | "timeout"
                        | "cancelled"
                        | "provider"
                )
            })
        {
            return Err(StorageError::InvalidData);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let row: Option<(String, String, i64, i64, String, String)> = tx.query_row(
            "SELECT operation_id, credential_id, reserved_cost_micros, started_at_unix_ms, currency, status
             FROM ai_attempts WHERE attempt_id = ?1 AND profile_id = ?2",
            params![result.attempt_id.to_string(), profile],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        ).optional().map_err(|_| StorageError::Unavailable)?;
        let Some((operation, credential, reserved, started, currency, status)) = row else {
            return Err(StorageError::NotFound);
        };
        if !matches!(status.as_str(), "dispatching" | "streaming")
            || result.ended_at_unix_ms < started
        {
            return Err(StorageError::RevisionConflict);
        }
        let usage_json = result
            .usage
            .map(|usage| serde_json::to_vec(&usage))
            .transpose()
            .map_err(|_| StorageError::InvalidData)?;
        let complete = result.usage_complete
            && result.usage.is_some()
            && u64::try_from(reserved).is_ok_and(|maximum| {
                result
                    .settled_cost_micros
                    .is_some_and(|cost| cost <= maximum)
            });
        let actual = if complete {
            result
                .settled_cost_micros
                .map(i64::try_from)
                .transpose()
                .map_err(|_| StorageError::InvalidData)?
        } else {
            None
        };
        record_lifetime_counter(
            &tx,
            &profile,
            &credential,
            &currency,
            actual.unwrap_or(reserved),
            actual.is_none(),
        )?;
        tx.execute(
            "UPDATE ai_attempts SET status = ?1, ended_at_unix_ms = ?2, effective_model = ?3,
             usage_json = ?4, usage_complete = ?5, settled_cost_micros = ?6,
             estimate_completeness = ?7, error_category = ?8 WHERE attempt_id = ?9 AND profile_id = ?10",
            params![result.status.as_str(), result.ended_at_unix_ms, result.effective_model, usage_json,
                    i64::from(complete), actual, if complete {"complete"} else {"partial"}, result.error_category,
                    result.attempt_id.to_string(), profile],
        ).map_err(|_| StorageError::Unavailable)?;
        tx.execute(
            "UPDATE ai_guardrail_policies SET reserved_micros = reserved_micros - ?1,
             counted_micros = counted_micros + ?2, unresolved_micros = unresolved_micros + ?3,
             revision = revision + 1 WHERE profile_id = ?4 AND credential_id = ?5 AND currency = ?6
             AND activated_at_unix_ms <= ?7 AND period_start_unix_ms <= ?7
             AND (period_end_unix_ms IS NULL OR ?7 < period_end_unix_ms)",
            params![
                reserved,
                actual.unwrap_or(0),
                if actual.is_some() { 0 } else { reserved },
                profile,
                credential,
                currency,
                started
            ],
        )
        .map_err(|_| StorageError::Unavailable)?;
        if !result.keep_operation_active {
            tx.execute(
                "UPDATE ai_operations SET status = ?1, ended_at_unix_ms = ?2, cancelled = ?3
             WHERE operation_id = ?4 AND profile_id = ?5 AND status = 'active'",
                params![
                    result.status.as_str(),
                    result.ended_at_unix_ms,
                    i64::from(result.status == AiTerminalStatus::Cancelled),
                    operation,
                    profile
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
        }
        tx.commit().map_err(|_| StorageError::Unavailable)
    }

    /// Converts crash-interrupted dispatched calls to unknown outcomes and
    /// moves their reservations to unresolved cap state.
    /// # Errors
    /// Returns `Unavailable` when the transaction cannot commit.
    pub fn recover_ai_attempts(&self, now_unix_ms: i64) -> Result<u64, StorageError> {
        if now_unix_ms <= 0 {
            return Err(StorageError::InvalidData);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let mut statement = tx.prepare(
            "SELECT attempt_id, operation_id, credential_id, reserved_cost_micros, started_at_unix_ms, currency, status
             FROM ai_attempts WHERE profile_id = ?1 AND status IN ('reserved','dispatching','streaming')",
        ).map_err(|_| StorageError::Unavailable)?;
        let rows = statement
            .query_map([&profile], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|_| StorageError::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StorageError::Unavailable)?;
        drop(statement);
        for (attempt, operation, credential, reserved, started, currency, status) in &rows {
            let dispatched = status != "reserved";
            if dispatched {
                record_lifetime_counter(&tx, &profile, credential, currency, *reserved, true)?;
            }
            tx.execute(
                "UPDATE ai_attempts SET status = ?1, ended_at_unix_ms = ?2,
                estimate_completeness = ?3, error_category = ?4,
                settled_cost_micros = ?5, usage_complete = ?6
                WHERE attempt_id = ?7",
                params![
                    if dispatched {
                        "outcome_unknown"
                    } else {
                        "cancelled"
                    },
                    now_unix_ms.max(*started),
                    if dispatched { "partial" } else { "complete" },
                    if dispatched { "provider" } else { "cancelled" },
                    if dispatched { None } else { Some(0_i64) },
                    i64::from(!dispatched),
                    attempt
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
            tx.execute(
                "UPDATE ai_operations SET status = ?1, ended_at_unix_ms = ?2, cancelled = ?3
                WHERE operation_id = ?4 AND status = 'active'",
                params![
                    if dispatched {
                        "outcome_unknown"
                    } else {
                        "cancelled"
                    },
                    now_unix_ms.max(*started),
                    i64::from(!dispatched),
                    operation
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
            tx.execute(
                "UPDATE ai_guardrail_policies SET reserved_micros = reserved_micros - ?1,
                unresolved_micros = unresolved_micros + ?2, revision = revision + 1
                WHERE profile_id = ?3 AND credential_id = ?4 AND currency = ?5
                AND activated_at_unix_ms <= ?6 AND period_start_unix_ms <= ?6
                AND (period_end_unix_ms IS NULL OR ?6 < period_end_unix_ms)",
                params![
                    reserved,
                    if dispatched { *reserved } else { 0 },
                    profile,
                    credential,
                    currency,
                    started
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
        }
        tx.commit().map_err(|_| StorageError::Unavailable)?;
        u64::try_from(rows.len()).map_err(|_| StorageError::InvalidData)
    }

    /// Returns content-free, selected-period aggregate activity. Provider and
    /// status counts are breakdowns, not a browsable individual-call feed.
    /// # Errors
    /// Rejects invalid periods, persisted usage, or overflow.
    #[allow(clippy::too_many_lines)]
    pub fn ai_monitoring_summary(
        &self,
        from_unix_ms: i64,
        to_unix_ms: i64,
        time_zone: &str,
        bucket_size: AiBucketSize,
    ) -> Result<AiMonitoringSummary, StorageError> {
        self.ai_monitoring_summary_for_key(from_unix_ms, to_unix_ms, time_zone, bucket_size, None)
    }

    /// Aggregates one credential or all credentials, including removed-key history.
    /// # Errors
    /// Rejects invalid periods, persisted usage, or overflow.
    #[allow(clippy::too_many_lines)]
    pub fn ai_monitoring_summary_for_key(
        &self,
        from_unix_ms: i64,
        to_unix_ms: i64,
        time_zone: &str,
        bucket_size: AiBucketSize,
        credential_id: Option<Uuid>,
    ) -> Result<AiMonitoringSummary, StorageError> {
        if from_unix_ms < 0 || from_unix_ms >= to_unix_ms {
            return Err(StorageError::InvalidData);
        }
        Timestamp::from_millisecond(to_unix_ms)
            .map_err(|_| StorageError::InvalidData)?
            .in_tz(time_zone)
            .map_err(|_| StorageError::InvalidData)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let mut statement = connection
            .prepare(
                "SELECT a.operation_id, a.provider, a.status, a.usage_json, a.usage_complete,
                    a.settled_cost_micros, a.reserved_cost_micros, a.currency,
                    a.started_at_unix_ms, COALESCE(a.effective_model, a.requested_model),
                    a.preset_version, o.operation_type, a.credential_id
             FROM ai_attempts a JOIN ai_operations o ON o.operation_id = a.operation_id
             WHERE a.profile_id = ?1 AND a.started_at_unix_ms >= ?2
             AND a.started_at_unix_ms < ?3 AND (?4 IS NULL OR a.credential_id = ?4)
             ORDER BY a.started_at_unix_ms, a.attempt_id",
            )
            .map_err(|_| StorageError::Unavailable)?;
        let rows = statement
            .query_map(
                params![
                    self.manifest.profile_id.to_string(),
                    from_unix_ms,
                    to_unix_ms,
                    credential_id.map(|id| id.to_string())
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<Vec<u8>>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, String>(11)?,
                        row.get::<_, String>(12)?,
                    ))
                },
            )
            .map_err(|_| StorageError::Unavailable)?;
        let mut summary = AiMonitoringSummary::default();
        let mut operations = std::collections::HashSet::new();
        let mut currencies = std::collections::HashSet::new();
        let mut buckets = BTreeMap::<String, AiMonitoringBucket>::new();
        for row in rows {
            let (
                operation,
                provider,
                status,
                usage,
                complete,
                cost,
                reserved,
                currency,
                started,
                model,
                preset,
                operation_type,
                credential,
            ) = row.map_err(|_| StorageError::Unavailable)?;
            operations.insert(operation);
            summary.attempts = summary
                .attempts
                .checked_add(1)
                .ok_or(StorageError::InvalidData)?;
            *summary.by_provider.entry(provider).or_default() += 1;
            *summary.by_credential_id.entry(credential).or_default() += 1;
            *summary.by_status.entry(status).or_default() += 1;
            *summary.by_model.entry(model).or_default() += 1;
            *summary.by_preset.entry(preset).or_default() += 1;
            *summary.by_operation_type.entry(operation_type).or_default() += 1;
            let label = monitoring_bucket_label(started, time_zone, bucket_size)?;
            let bucket = buckets
                .entry(label.clone())
                .or_insert_with(|| AiMonitoringBucket {
                    label,
                    ..AiMonitoringBucket::default()
                });
            bucket.attempts = bucket
                .attempts
                .checked_add(1)
                .ok_or(StorageError::InvalidData)?;
            if let Some(bytes) = usage {
                let value: Usage =
                    serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidData)?;
                add_usage(&mut summary.usage, value)?;
                add_usage(&mut bucket.usage, value)?;
            }
            currencies.insert(currency.clone());
            if complete == 0 || cost.is_none() {
                summary.partial = true;
                summary.unknown_count = summary
                    .unknown_count
                    .checked_add(1)
                    .ok_or(StorageError::InvalidData)?;
                summary.unresolved_reserved_micros = summary
                    .unresolved_reserved_micros
                    .checked_add(u64::try_from(reserved).map_err(|_| StorageError::InvalidData)?)
                    .ok_or(StorageError::InvalidData)?;
                bucket.partial = true;
                bucket.unknown_count = bucket
                    .unknown_count
                    .checked_add(1)
                    .ok_or(StorageError::InvalidData)?;
            }
            if let Some(cost) = cost {
                let cost = u64::try_from(cost).map_err(|_| StorageError::InvalidData)?;
                let total = summary
                    .cost_by_currency_micros
                    .entry(currency.clone())
                    .or_default();
                *total = total.checked_add(cost).ok_or(StorageError::InvalidData)?;
                let bucket_cost = bucket.cost_by_currency_micros.entry(currency).or_default();
                *bucket_cost = bucket_cost
                    .checked_add(cost)
                    .ok_or(StorageError::InvalidData)?;
            }
        }
        if summary.cost_by_currency_micros.len() <= 1 {
            summary.estimated_cost_micros = summary.cost_by_currency_micros.values().copied().sum();
        }
        summary.logical_operations =
            u64::try_from(operations.len()).map_err(|_| StorageError::InvalidData)?;
        if currencies.len() == 1 {
            summary.currency = currencies.into_iter().next();
        } else if currencies.len() > 1 {
            summary.partial = true;
        }
        summary.time_buckets = buckets.into_values().collect();
        Ok(summary)
    }

    /// Ordinary JSON export contains the same aggregate result as Monitoring.
    /// # Errors
    /// Propagates monitoring errors.
    pub fn export_ai_monitoring_json(
        &self,
        from_unix_ms: i64,
        to_unix_ms: i64,
        time_zone: &str,
        bucket_size: AiBucketSize,
    ) -> Result<Vec<u8>, StorageError> {
        let summary =
            self.ai_monitoring_summary(from_unix_ms, to_unix_ms, time_zone, bucket_size)?;
        serde_json::to_vec(&summary).map_err(|_| StorageError::InvalidData)
    }

    /// Clears only terminal logical-operation activity in the selected period.
    /// Guardrail counters are intentionally left intact.
    /// # Errors
    /// Rejects invalid periods or active operations in the selected period.
    pub fn clear_ai_activity(
        &self,
        from_unix_ms: i64,
        to_unix_ms: i64,
    ) -> Result<u64, StorageError> {
        self.clear_ai_activity_for_key(from_unix_ms, to_unix_ms, None)
    }

    /// Clears terminal operations for only the selected credential, or all keys.
    /// # Errors
    /// Rejects invalid periods or matching active operations.
    pub fn clear_ai_activity_for_key(
        &self,
        from_unix_ms: i64,
        to_unix_ms: i64,
        credential_id: Option<Uuid>,
    ) -> Result<u64, StorageError> {
        if from_unix_ms < 0 || from_unix_ms >= to_unix_ms {
            return Err(StorageError::InvalidData);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let active: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM ai_operations WHERE profile_id = ?1
            AND started_at_unix_ms >= ?2 AND started_at_unix_ms < ?3 AND status = 'active'
            AND (?4 IS NULL OR EXISTS (SELECT 1 FROM ai_attempts a WHERE a.operation_id = ai_operations.operation_id AND a.credential_id = ?4))",
                params![profile, from_unix_ms, to_unix_ms, credential_id.map(|id| id.to_string())],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::Unavailable)?;
        if active != 0 {
            return Err(StorageError::RevisionConflict);
        }
        preserve_lifetime_totals(&tx, &profile)?;
        let changed = tx
            .execute(
                "DELETE FROM ai_operations WHERE profile_id = ?1
            AND started_at_unix_ms >= ?2 AND started_at_unix_ms < ?3 AND status != 'active'
            AND (?4 IS NULL OR EXISTS (SELECT 1 FROM ai_attempts a WHERE a.operation_id = ai_operations.operation_id AND a.credential_id = ?4))",
                params![profile, from_unix_ms, to_unix_ms, credential_id.map(|id| id.to_string())],
            )
            .map_err(|_| StorageError::Unavailable)?;
        tx.commit().map_err(|_| StorageError::Unavailable)?;
        u64::try_from(changed).map_err(|_| StorageError::InvalidData)
    }

    /// Explicitly establishes a new zero baseline for one all-time cap. This
    /// is never coupled to ordinary activity clearing.
    /// # Errors
    /// Rejects non-all-time resets, active operations, invalid time, or absent policies.
    pub fn reset_ai_cap(
        &self,
        credential_id: Uuid,
        period: AiPeriod,
        now_unix_ms: i64,
    ) -> Result<(), StorageError> {
        if period != AiPeriod::AllTime || now_unix_ms <= 0 {
            return Err(StorageError::InvalidData);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let active: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM ai_operations WHERE profile_id = ?1 AND status = 'active'",
                [&profile],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::Unavailable)?;
        if active != 0 {
            return Err(StorageError::RevisionConflict);
        }
        let changed = tx
            .execute(
                "UPDATE ai_guardrail_policies SET counted_micros = 0,
            reserved_micros = 0, unresolved_micros = 0,
            activated_at_unix_ms = ?1, period_start_unix_ms = ?1,
            period_end_unix_ms = NULL, revision = revision + 1
            WHERE profile_id = ?2 AND credential_id = ?3 AND period = ?4",
                params![
                    now_unix_ms,
                    profile,
                    credential_id.to_string(),
                    period.as_str()
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
        tx.commit().map_err(|_| StorageError::Unavailable)
    }

    /// Lifetime estimated spend by currency, independent of cap baselines and activity retention.
    /// # Errors
    /// Rejects unavailable storage or corrupt totals.
    pub fn ai_lifetime_spend(
        &self,
        credential_id: Uuid,
    ) -> Result<BTreeMap<String, u64>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let profile = self.manifest.profile_id.to_string();
        let credential = credential_id.to_string();
        let prefix = format!("ai.lifetime.v1.{credential}.");
        let mut statement = connection.prepare("SELECT DISTINCT currency FROM ai_attempts WHERE profile_id = ?1 AND credential_id = ?2 UNION SELECT substr(setting_key, length(?3) + 1) FROM settings WHERE profile_id = ?1 AND substr(setting_key, 1, length(?3)) = ?3").map_err(|_| StorageError::Unavailable)?;
        let currencies = statement
            .query_map(params![profile, credential, prefix], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|_| StorageError::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StorageError::Unavailable)?;
        currencies
            .into_iter()
            .map(|currency| {
                let total = lifetime_cost(&connection, &profile, &credential, &currency)?;
                Ok((
                    currency,
                    u64::try_from(total).map_err(|_| StorageError::InvalidData)?,
                ))
            })
            .collect()
    }

    /// Whether some historical spend could not be estimated; retained after activity clearing.
    /// # Errors
    /// Rejects unavailable storage or corrupt metadata.
    pub fn ai_lifetime_spend_is_partial(&self, credential_id: Uuid) -> Result<bool, StorageError> {
        if let Some(saved) =
            self.load_setting(&format!("ai.lifetime_partial.v1.{credential_id}"))?
        {
            if saved.value != serde_json::Value::Bool(true) {
                return Err(StorageError::InvalidData);
            }
            return Ok(true);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let count: i64 = connection.query_row("SELECT COUNT(*) FROM ai_attempts WHERE profile_id = ?1 AND credential_id = ?2 AND estimate_completeness != 'complete' AND status NOT IN ('reserved', 'dispatching', 'streaming')", params![self.manifest.profile_id.to_string(), credential_id.to_string()], |row| row.get(0)).map_err(|_| StorageError::Unavailable)?;
        Ok(count != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_vault::testing::MemoryDatabaseKeyVault;

    fn preflight(cost: u64) -> AiAttemptPreflight {
        AiAttemptPreflight {
            operation_id: Uuid::now_v7(),
            attempt_id: Uuid::now_v7(),
            operation_type: OperationType::CredentialTest,
            provider: Provider::OpenAi,
            credential_id: Uuid::from_u128(42),
            requested_model: "fixture-model".into(),
            preset_version: "v1".into(),
            catalog_id: "fixture-catalog".into(),
            catalog_effective_from: "2026-09-13T00:00:00Z".into(),
            pricing_components: vec![Price {
                category: ort_ai::PriceCategory::Input,
                micros_per_million: 1,
            }],
            started_at_unix_ms: 1_000,
            estimated_input_tokens: 100,
            maximum_cost_micros: cost,
            currency: "USD".into(),
            retry_of: None,
        }
    }

    #[test]
    fn unified_caps_preserve_lifetime_spend_and_history_across_edits_resets_and_clearing() {
        let temp = tempfile::tempdir().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        let id = Uuid::from_u128(42);
        let first = preflight(80);
        store.reserve_ai_attempt(&first).unwrap();
        store.mark_ai_dispatching(first.attempt_id).unwrap();
        store
            .settle_ai_attempt(&AiAttemptSettlement {
                attempt_id: first.attempt_id,
                status: AiTerminalStatus::Succeeded,
                effective_model: Some("fixture-model".into()),
                usage: Some(Usage {
                    input_tokens: 10,
                    ..Usage::default()
                }),
                settled_cost_micros: Some(60),
                usage_complete: true,
                error_category: None,
                ended_at_unix_ms: 2_000,
                keep_operation_active: false,
            })
            .unwrap();
        assert_eq!(store.ai_lifetime_spend(id).unwrap()["USD"], 60);
        // Emulate a pre-upgrade profile: lifetime falls back to retained attempts,
        // then activity clearing must preserve that total before deleting them.
        store
            .connection
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM settings WHERE setting_key LIKE 'ai.lifetime.v1.%'",
                [],
            )
            .unwrap();
        assert_eq!(store.ai_lifetime_spend(id).unwrap()["USD"], 60);
        assert_eq!(
            store.ai_lifetime_spend(Uuid::from_u128(43)).unwrap().len(),
            0
        );
        let mut policy = AiCapPolicy {
            credential_id: id,
            period: AiPeriod::AllTime,
            currency: "USD".into(),
            time_zone: "UTC".into(),
            limit_micros: 100,
            activated_at_unix_ms: 2_500,
            period_start_unix_ms: 2_500,
            period_end_unix_ms: None,
            expected_revision: None,
        };
        store.save_ai_unified_cap(&policy).unwrap();
        let cap = store.ai_cap_policies(id).unwrap().pop().unwrap();
        assert_eq!(cap.counted_micros, 60);
        policy.limit_micros = 200;
        policy.expected_revision = Some(cap.revision);
        store.save_ai_unified_cap(&policy).unwrap();
        assert_eq!(store.ai_cap_policies(id).unwrap()[0].counted_micros, 60);
        store.reset_ai_cap(id, AiPeriod::AllTime, 3_000).unwrap();
        assert_eq!(store.ai_cap_policies(id).unwrap()[0].counted_micros, 0);
        assert_eq!(store.ai_lifetime_spend(id).unwrap()["USD"], 60);
        assert_eq!(
            store
                .ai_monitoring_summary(0, 4_000, "UTC", AiBucketSize::Day)
                .unwrap()
                .estimated_cost_micros,
            60
        );
        assert_eq!(store.clear_ai_activity(0, 4_000).unwrap(), 1);
        assert_eq!(store.ai_lifetime_spend(id).unwrap()["USD"], 60);
        let mut second = preflight(30);
        second.started_at_unix_ms = 5_000;
        store.reserve_ai_attempt(&second).unwrap();
        assert_eq!(
            store.reset_ai_cap(id, AiPeriod::AllTime, 5_001),
            Err(StorageError::RevisionConflict)
        );
        store.mark_ai_dispatching(second.attempt_id).unwrap();
        store
            .settle_ai_attempt(&AiAttemptSettlement {
                attempt_id: second.attempt_id,
                status: AiTerminalStatus::Succeeded,
                effective_model: Some("fixture-model".into()),
                usage: Some(Usage {
                    input_tokens: 10,
                    ..Usage::default()
                }),
                settled_cost_micros: Some(20),
                usage_complete: true,
                error_category: None,
                ended_at_unix_ms: 6_000,
                keep_operation_active: false,
            })
            .unwrap();
        assert_eq!(store.ai_cap_policies(id).unwrap()[0].counted_micros, 20);
        assert_eq!(store.ai_lifetime_spend(id).unwrap()["USD"], 80);
        drop(store);
        let reopened = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        assert_eq!(reopened.ai_lifetime_spend(id).unwrap()["USD"], 80);
    }

    #[test]
    fn per_key_monitoring_and_clearing_cannot_mix_credentials() {
        let temp = tempfile::tempdir().unwrap();
        let store =
            EncryptedStore::open_or_initialize(temp.path(), "test", &MemoryDatabaseKeyVault::new())
                .unwrap();
        let first = preflight(60);
        let mut second = preflight(80);
        second.credential_id = Uuid::from_u128(43);
        for attempt in [&first, &second] {
            store.reserve_ai_attempt(attempt).unwrap();
            store.mark_ai_dispatching(attempt.attempt_id).unwrap();
            store
                .settle_ai_attempt(&AiAttemptSettlement {
                    attempt_id: attempt.attempt_id,
                    status: AiTerminalStatus::Succeeded,
                    effective_model: Some("fixture-model".into()),
                    usage: Some(Usage {
                        input_tokens: 10,
                        output_tokens: 2,
                        ..Usage::default()
                    }),
                    settled_cost_micros: Some(attempt.maximum_cost_micros),
                    usage_complete: true,
                    error_category: None,
                    ended_at_unix_ms: 2_000,
                    keep_operation_active: false,
                })
                .unwrap();
        }
        let all = store
            .ai_monitoring_summary(0, 3000, "UTC", AiBucketSize::Day)
            .unwrap();
        assert_eq!(all.attempts, 2);
        assert_eq!(all.by_credential_id.len(), 2);
        let selected = store
            .ai_monitoring_summary_for_key(
                0,
                3000,
                "UTC",
                AiBucketSize::Day,
                Some(first.credential_id),
            )
            .unwrap();
        assert_eq!(selected.attempts, 1);
        assert_eq!(selected.estimated_cost_micros, 60);
        assert_eq!(selected.usage.input_tokens, 10);
        assert_eq!(selected.by_credential_id.len(), 1);
        assert_eq!(
            store
                .clear_ai_activity_for_key(0, 3000, Some(first.credential_id))
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .ai_monitoring_summary(0, 3000, "UTC", AiBucketSize::Day)
                .unwrap()
                .estimated_cost_micros,
            80
        );
        assert_eq!(
            store
                .ai_monitoring_summary_for_key(
                    0,
                    3000,
                    "UTC",
                    AiBucketSize::Day,
                    Some(Uuid::from_u128(999))
                )
                .unwrap()
                .attempts,
            0
        );
        let active = preflight(20);
        store.reserve_ai_attempt(&active).unwrap();
        assert_eq!(
            store
                .clear_ai_activity_for_key(0, 3000, Some(second.credential_id))
                .unwrap(),
            1
        );
        assert_eq!(
            store.clear_ai_activity_for_key(0, 3000, Some(active.credential_id)),
            Err(StorageError::RevisionConflict)
        );
    }

    #[test]
    fn provider_cost_above_reservation_stays_unresolved_without_overshooting_cap() {
        let temp = tempfile::tempdir().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        store
            .save_ai_cap_policy(&AiCapPolicy {
                credential_id: Uuid::from_u128(42),
                period: AiPeriod::AllTime,
                currency: "USD".into(),
                time_zone: "UTC".into(),
                limit_micros: 100,
                activated_at_unix_ms: 1,
                period_start_unix_ms: 1,
                period_end_unix_ms: None,
                expected_revision: None,
            })
            .unwrap();
        let attempt = preflight(60);
        store.reserve_ai_attempt(&attempt).unwrap();
        store.mark_ai_dispatching(attempt.attempt_id).unwrap();
        store
            .settle_ai_attempt(&AiAttemptSettlement {
                attempt_id: attempt.attempt_id,
                status: AiTerminalStatus::Succeeded,
                effective_model: Some("fixture-model".into()),
                usage: Some(Usage {
                    input_tokens: 100,
                    ..Usage::default()
                }),
                settled_cost_micros: Some(80),
                usage_complete: true,
                error_category: None,
                ended_at_unix_ms: 2_000,
                keep_operation_active: false,
            })
            .unwrap();
        let policy = store.ai_cap_policies(Uuid::from_u128(42)).unwrap();
        assert_eq!(policy[0].counted_micros, 0);
        assert_eq!(policy[0].unresolved_micros, 60);
        let summary = store
            .ai_monitoring_summary(0, 3_000, "UTC", AiBucketSize::Day)
            .unwrap();
        assert!(summary.partial);
        assert_eq!(summary.unresolved_reserved_micros, 60);
        assert!(
            store
                .ai_lifetime_spend_is_partial(Uuid::from_u128(42))
                .unwrap()
        );
        store.clear_ai_activity(0, 3_000).unwrap();
        assert!(
            store
                .ai_lifetime_spend_is_partial(Uuid::from_u128(42))
                .unwrap()
        );
        assert_eq!(
            store.ai_cap_policies(Uuid::from_u128(42)).unwrap()[0].unresolved_micros,
            60
        );
        store
            .disable_ai_cap(Uuid::from_u128(42), AiPeriod::AllTime)
            .unwrap();
        store
            .save_ai_unified_cap(&AiCapPolicy {
                credential_id: Uuid::from_u128(42),
                period: AiPeriod::AllTime,
                currency: "USD".into(),
                time_zone: "UTC".into(),
                limit_micros: 100,
                activated_at_unix_ms: 4_000,
                period_start_unix_ms: 4_000,
                period_end_unix_ms: None,
                expected_revision: None,
            })
            .unwrap();
        assert_eq!(
            store.ai_cap_policies(Uuid::from_u128(42)).unwrap()[0].unresolved_micros,
            60
        );
        store
            .reset_ai_cap(Uuid::from_u128(42), AiPeriod::AllTime, 5_000)
            .unwrap();
        assert_eq!(
            store.ai_cap_policies(Uuid::from_u128(42)).unwrap()[0].unresolved_micros,
            0
        );
        assert!(
            store
                .ai_lifetime_spend_is_partial(Uuid::from_u128(42))
                .unwrap()
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn reservation_settlement_and_export_are_atomic_and_content_free() {
        let temp = tempfile::tempdir().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        let policy = AiCapPolicy {
            credential_id: Uuid::from_u128(42),
            period: AiPeriod::AllTime,
            currency: "USD".into(),
            time_zone: "UTC".into(),
            limit_micros: 100,
            activated_at_unix_ms: 1,
            period_start_unix_ms: 1,
            period_end_unix_ms: None,
            expected_revision: None,
        };
        store.save_ai_cap_policy(&policy).unwrap();
        let first = preflight(60);
        store.reserve_ai_attempt(&first).unwrap();
        assert_eq!(
            store.reserve_ai_attempt(&preflight(50)),
            Err(StorageError::RevisionConflict)
        );
        store.mark_ai_dispatching(first.attempt_id).unwrap();
        store
            .settle_ai_attempt(&AiAttemptSettlement {
                attempt_id: first.attempt_id,
                status: AiTerminalStatus::Succeeded,
                effective_model: Some("fixture-model".into()),
                usage: Some(Usage {
                    input_tokens: 100,
                    output_tokens: 10,
                    ..Usage::default()
                }),
                settled_cost_micros: Some(40),
                usage_complete: true,
                error_category: None,
                ended_at_unix_ms: 2_000,
                keep_operation_active: false,
            })
            .unwrap();
        let second = preflight(70);
        assert_eq!(
            store.reserve_ai_attempt(&second),
            Err(StorageError::InvalidData)
        );
        assert_eq!(
            store
                .ai_monitoring_summary(0, 3_000, "UTC", AiBucketSize::Day)
                .unwrap()
                .attempts,
            1
        );
        let summary = store
            .ai_monitoring_summary(0, 3_000, "UTC", AiBucketSize::Day)
            .unwrap();
        assert_eq!(summary.estimated_cost_micros, 40);
        assert_eq!(summary.logical_operations, 1);
        assert_eq!(summary.by_model.get("fixture-model"), Some(&1));
        assert_eq!(summary.by_preset.get("v1"), Some(&1));
        assert_eq!(summary.by_operation_type.get("credential_test"), Some(&1));
        assert_eq!(summary.time_buckets.len(), 1);
        assert_eq!(summary.time_buckets[0].label, "1970-01-01");
        assert_eq!(summary.time_buckets[0].usage.output_tokens, 10);
        assert_eq!(
            store.ai_monitoring_summary(0, 3_000, "not/a-zone", AiBucketSize::Day),
            Err(StorageError::InvalidData)
        );
        let exported: AiMonitoringSummary = serde_json::from_slice(
            &store
                .export_ai_monitoring_json(0, 3_000, "UTC", AiBucketSize::Day)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(exported, summary);
        assert_eq!(store.clear_ai_activity(0, 3_000).unwrap(), 1);
        assert_eq!(
            store
                .ai_monitoring_summary(0, 3_000, "UTC", AiBucketSize::Day)
                .unwrap()
                .attempts,
            0
        );
        assert_eq!(
            store.reserve_ai_attempt(&preflight(70)),
            Err(StorageError::InvalidData)
        );
        store
            .reset_ai_cap(Uuid::from_u128(42), AiPeriod::AllTime, 2_500)
            .unwrap();
        let reset_policy = store.ai_cap_policies(Uuid::from_u128(42)).unwrap();
        assert_eq!(reset_policy[0].activated_at_unix_ms, 2_500);
        assert_eq!(reset_policy[0].period_start_unix_ms, 2_500);
        assert_eq!(reset_policy[0].counted_micros, 0);
        let mut after_reset = preflight(70);
        after_reset.started_at_unix_ms = 2_600;
        store.reserve_ai_attempt(&after_reset).unwrap();
        assert_eq!(
            store.clear_ai_activity(0, 3_000),
            Err(StorageError::RevisionConflict)
        );
        store.recover_ai_attempts(3_000).unwrap();
        drop(store);
        let reopened = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        assert_eq!(
            reopened
                .ai_monitoring_summary(0, 3_000, "UTC", AiBucketSize::Day)
                .unwrap()
                .attempts,
            1
        );
    }

    #[test]
    fn recovery_distinguishes_undispatched_from_unknown_outcome() {
        let temp = tempfile::tempdir().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        let first = preflight(25);
        store.reserve_ai_attempt(&first).unwrap();
        assert_eq!(store.recover_ai_attempts(2_000).unwrap(), 1);
        let summary = store
            .ai_monitoring_summary(0, 3_000, "UTC", AiBucketSize::Day)
            .unwrap();
        assert_eq!(summary.by_status.get("cancelled"), Some(&1));
        let second = preflight(25);
        store.reserve_ai_attempt(&second).unwrap();
        store
            .cancel_reserved_ai_attempt(second.attempt_id, 2_500)
            .unwrap();
        assert_eq!(
            store.mark_ai_dispatching(second.attempt_id),
            Err(StorageError::RevisionConflict)
        );
        let third = preflight(25);
        store.reserve_ai_attempt(&third).unwrap();
        store.mark_ai_dispatching(third.attempt_id).unwrap();
        assert_eq!(store.recover_ai_attempts(3_000).unwrap(), 1);
        let summary = store
            .ai_monitoring_summary(0, 4_000, "UTC", AiBucketSize::Day)
            .unwrap();
        assert_eq!(summary.by_status.get("outcome_unknown"), Some(&1));
        assert!(summary.partial);
        assert_eq!(store.recover_ai_attempts(4_000).unwrap(), 0);
    }

    #[test]
    fn one_transient_retry_stays_in_the_logical_operation() {
        let temp = tempfile::tempdir().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        let first = preflight(25);
        store.reserve_ai_attempt(&first).unwrap();
        store.mark_ai_dispatching(first.attempt_id).unwrap();
        store
            .settle_ai_attempt(&AiAttemptSettlement {
                attempt_id: first.attempt_id,
                status: AiTerminalStatus::Failed,
                effective_model: None,
                usage: None,
                settled_cost_micros: None,
                usage_complete: false,
                error_category: Some("transient".into()),
                ended_at_unix_ms: 1_500,
                keep_operation_active: true,
            })
            .unwrap();
        let mut retry = preflight(25);
        retry.operation_id = first.operation_id;
        retry.started_at_unix_ms = 1_600;
        retry.retry_of = Some(first.attempt_id);
        store.reserve_ai_attempt(&retry).unwrap();
        let mut duplicate = preflight(25);
        duplicate.operation_id = first.operation_id;
        duplicate.retry_of = Some(first.attempt_id);
        duplicate.started_at_unix_ms = 1_700;
        assert_eq!(
            store.reserve_ai_attempt(&duplicate),
            Err(StorageError::RevisionConflict)
        );
        store.mark_ai_dispatching(retry.attempt_id).unwrap();
        store
            .settle_ai_attempt(&AiAttemptSettlement {
                attempt_id: retry.attempt_id,
                status: AiTerminalStatus::Succeeded,
                effective_model: Some("fixture-model".into()),
                usage: Some(Usage {
                    input_tokens: 10,
                    output_tokens: 2,
                    ..Usage::default()
                }),
                settled_cost_micros: Some(10),
                usage_complete: true,
                error_category: None,
                ended_at_unix_ms: 2_000,
                keep_operation_active: false,
            })
            .unwrap();
        let summary = store
            .ai_monitoring_summary(0, 3_000, "UTC", AiBucketSize::Day)
            .unwrap();
        assert_eq!(summary.logical_operations, 1);
        assert_eq!(summary.attempts, 2);
        assert_eq!(summary.by_operation_type.get("credential_test"), Some(&2));
    }

    #[test]
    fn calendar_caps_roll_at_recorded_zone_boundaries_and_can_be_disabled() {
        let before_dst = "2026-03-07T12:00:00Z"
            .parse::<Timestamp>()
            .unwrap()
            .as_millisecond();
        let after_dst = "2026-03-09T12:00:00Z"
            .parse::<Timestamp>()
            .unwrap()
            .as_millisecond();
        let (first_start, first_end) =
            ai_calendar_bounds(before_dst, "America/New_York", AiPeriod::Week).unwrap();
        let (second_start, second_end) =
            ai_calendar_bounds(after_dst, "America/New_York", AiPeriod::Week).unwrap();
        assert_eq!(first_end, Some(second_start));
        assert_eq!(first_end.unwrap() - first_start, 167 * 60 * 60 * 1_000);
        assert_eq!(second_end.unwrap() - second_start, 168 * 60 * 60 * 1_000);

        let temp = tempfile::tempdir().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        store
            .save_ai_cap_policy(&AiCapPolicy {
                credential_id: Uuid::from_u128(42),
                period: AiPeriod::Week,
                currency: "USD".into(),
                time_zone: "America/New_York".into(),
                limit_micros: 100,
                activated_at_unix_ms: before_dst,
                period_start_unix_ms: first_start,
                period_end_unix_ms: first_end,
                expected_revision: None,
            })
            .unwrap();
        let mut attempt = preflight(25);
        attempt.started_at_unix_ms = after_dst;
        store.reserve_ai_attempt(&attempt).unwrap();
        let policy = store
            .ai_cap_policies(Uuid::from_u128(42))
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(policy.period_start_unix_ms, second_start);
        assert_eq!(policy.reserved_micros, 25);
        store
            .cancel_reserved_ai_attempt(attempt.attempt_id, after_dst + 1)
            .unwrap();
        store
            .disable_ai_cap(Uuid::from_u128(42), AiPeriod::Week)
            .unwrap();
        assert!(
            store
                .ai_cap_policies(Uuid::from_u128(42))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            ai_calendar_bounds(after_dst, "Not/A_Zone", AiPeriod::Month),
            Err(StorageError::InvalidData)
        );
    }

    #[test]
    fn replacement_credential_copies_only_cap_configuration() {
        let temp = tempfile::tempdir().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
        let old = Uuid::now_v7();
        let new = Uuid::now_v7();
        store
            .save_ai_cap_policy(&AiCapPolicy {
                credential_id: old,
                period: AiPeriod::Month,
                currency: "USD".into(),
                time_zone: "America/New_York".into(),
                limit_micros: 1_000_000,
                activated_at_unix_ms: 1_725_192_000_000,
                period_start_unix_ms: 1_725_163_200_000,
                period_end_unix_ms: Some(1_727_841_600_000),
                expected_revision: None,
            })
            .unwrap();
        store
            .replace_ai_cap_identity(old, new, true, 1_725_192_100_000)
            .unwrap();
        assert!(store.ai_cap_policies(old).unwrap().is_empty());
        let copied = store.ai_cap_policies(new).unwrap();
        assert_eq!(copied.len(), 1);
        assert_eq!(copied[0].limit_micros, 1_000_000);
        assert_eq!(copied[0].counted_micros, 0);
        assert_eq!(copied[0].reserved_micros, 0);
        assert_eq!(copied[0].unresolved_micros, 0);
        assert_eq!(copied[0].activated_at_unix_ms, 1_725_192_100_000);
    }
}
