//! Portable accounting has its own typed, bounded collections, never preferences.
use std::collections::BTreeSet;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::BackupError;

pub const MAX_AI_ACCOUNTING_RECORDS: usize = 100_000;
const MAX_TIMESTAMP: i64 = 8_640_000_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiGuardrailPeriod {
    Week,
    Month,
    Year,
    AllTime,
}
impl AiGuardrailPeriod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
            Self::AllTime => "all_time",
        }
    }
    #[must_use]
    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "week" => Some(Self::Week),
            "month" => Some(Self::Month),
            "year" => Some(Self::Year),
            "all_time" => Some(Self::AllTime),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableAiLifetimeV1 {
    pub credential_id: String,
    pub currency: String,
    pub counted_micros: u64,
    pub unresolved_micros: u64,
    pub partial: bool,
}

/// Restored policies are historical configuration until explicitly rebound.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableAiGuardrailV1 {
    pub id: String,
    pub credential_id: String,
    pub period: AiGuardrailPeriod,
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

fn valid_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
fn valid_currency(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}
fn valid_amount(value: u64) -> bool {
    i64::try_from(value).is_ok()
}

pub(crate) fn validate_accounting(
    totals: &[PortableAiLifetimeV1],
    policies: &[PortableAiGuardrailV1],
) -> Result<(), BackupError> {
    if totals.len() > MAX_AI_ACCOUNTING_RECORDS || policies.len() > MAX_AI_ACCOUNTING_RECORDS {
        return Err(BackupError::InvalidContent);
    }
    let mut total_ids = BTreeSet::new();
    for total in totals {
        if !valid_uuid(&total.credential_id)
            || !valid_currency(&total.currency)
            || !valid_amount(total.counted_micros)
            || !valid_amount(total.unresolved_micros)
            || (total.unresolved_micros > 0 && !total.partial)
            || !total_ids.insert((&total.credential_id, &total.currency))
        {
            return Err(BackupError::InvalidContent);
        }
    }
    let mut policy_ids = BTreeSet::new();
    for policy in policies {
        let total = policy
            .counted_micros
            .checked_add(policy.reserved_micros)
            .and_then(|value| value.checked_add(policy.unresolved_micros));
        if !valid_uuid(&policy.id)
            || !valid_uuid(&policy.credential_id)
            || !policy_ids.insert(&policy.id)
            || !valid_currency(&policy.currency)
            || policy.limit_micros == 0
            || !valid_amount(policy.limit_micros)
            || total.is_none_or(|value| !valid_amount(value))
            || policy.revision == 0
            || !valid_amount(policy.revision)
            || !(1..=MAX_TIMESTAMP).contains(&policy.activated_at_unix_ms)
            || !(1..=MAX_TIMESTAMP).contains(&policy.period_start_unix_ms)
            || policy
                .period_end_unix_ms
                .is_some_and(|end| end <= policy.period_start_unix_ms || end > MAX_TIMESTAMP)
            || ((policy.period == AiGuardrailPeriod::AllTime)
                != policy.period_end_unix_ms.is_none())
            || policy.time_zone.is_empty()
            || policy.time_zone.len() > 128
            || Timestamp::from_millisecond(policy.activated_at_unix_ms)
                .and_then(|time| time.in_tz(&policy.time_zone))
                .is_err()
        {
            return Err(BackupError::InvalidContent);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_accounting_has_its_own_count_boundary_and_rejects_duplicates() {
        let mut totals = (0..MAX_AI_ACCOUNTING_RECORDS)
            .map(|index| PortableAiLifetimeV1 {
                credential_id: Uuid::from_u128(index as u128).to_string(),
                currency: "USD".into(),
                counted_micros: 1,
                unresolved_micros: 0,
                partial: false,
            })
            .collect::<Vec<_>>();
        assert_eq!(validate_accounting(&totals, &[]), Ok(()));
        totals.push(PortableAiLifetimeV1 {
            credential_id: Uuid::now_v7().to_string(),
            currency: "USD".into(),
            counted_micros: 1,
            unresolved_micros: 0,
            partial: false,
        });
        assert_eq!(
            validate_accounting(&totals, &[]),
            Err(BackupError::InvalidContent)
        );
        totals.truncate(2);
        totals[1] = totals[0].clone();
        assert_eq!(
            validate_accounting(&totals, &[]),
            Err(BackupError::InvalidContent)
        );
    }
}
