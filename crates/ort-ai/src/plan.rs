//! Non-secret ChatGPT-plan contracts and strict provider telemetry normalization.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const MODELS: [(&str, &str); 6] = [
    ("gpt-5.6-luna", "GPT-5.6 Luna"),
    ("gpt-5.6-terra", "GPT-5.6 Terra"),
    ("gpt-5.6-sol", "GPT-5.6 Sol"),
    ("gpt-6-luna", "GPT-6 Luna"),
    ("gpt-6-sol", "GPT-6 Sol"),
    ("gpt-6.1-sol", "GPT-6.1 Sol"),
];

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionSource {
    #[default]
    DirectApi,
    ChatgptPlan,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MonetaryCostTracking {
    #[default]
    Estimated,
    NotTracked,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanSettings {
    #[serde(default)]
    pub cleanup_required: bool,
    pub connection_id: Option<Uuid>,
    pub enabled: bool,
    pub model: Option<String>,
    pub reasoning: ReasoningEffort,
    pub reserve_enabled: bool,
    pub reserve_percent: u8,
}
impl Default for PlanSettings {
    fn default() -> Self {
        Self {
            cleanup_required: false,
            connection_id: None,
            enabled: false,
            model: None,
            reasoning: ReasoningEffort::Medium,
            reserve_enabled: true,
            reserve_percent: 20,
        }
    }
}
impl PlanSettings {
    /// Enabling is a saved permission independent of memory-only sign-in.
    /// # Errors
    /// A disabled or retiring session cannot start or reuse the runtime.
    pub fn runtime_permission(&self) -> Result<(), &'static str> {
        if !self.enabled {
            Err("PLAN_DISABLED")
        } else if self.cleanup_required {
            Err("PLAN_CREDENTIAL_CLEANUP_REQUIRED")
        } else {
            Ok(())
        }
    }

    #[must_use]
    pub fn valid(&self) -> bool {
        (1..=100).contains(&self.reserve_percent)
            && self
                .model
                .as_ref()
                .is_none_or(|m| MODELS.iter().any(|(id, _)| id == m))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanModel {
    pub id: String,
    pub name: String,
    pub supported: bool,
    pub explanation: Option<String>,
    pub reasoning_efforts: Vec<ReasoningEffort>,
}
pub fn models_from_catalog(value: &Value) -> Vec<PlanModel> {
    let data = value.get("data").and_then(Value::as_array);
    MODELS.iter().map(|(id, name)| {
        let found = data.and_then(|data| data.iter().find(|m| m.get("model").and_then(Value::as_str) == Some(id) && m.get("hidden").and_then(Value::as_bool) != Some(true)));
        let efforts = found.and_then(|m| m.get("supportedReasoningEfforts")).and_then(Value::as_array).into_iter().flatten().filter_map(|e| serde_json::from_value(e.get("reasoningEffort")?.clone()).ok()).collect::<Vec<_>>();
        let supported = found.is_some() && !efforts.is_empty();
        PlanModel { id: (*id).into(), name: (*name).into(), supported, explanation: (!supported).then(|| "This installed Codex runtime does not offer this model and reasoning controls.".into()), reasoning_efforts: efforts }
    }).collect()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub limit_id: String,
    pub name: String,
    pub window: String,
    pub remaining_percent: f64,
    pub window_duration_minutes: Option<u64>,
    pub resets_at: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSnapshot {
    pub fetched_at_unix_ms: i64,
    pub windows: Vec<QuotaWindow>,
}
/// Reads every supplied Codex window without inferring missing percentages.
/// # Errors
/// Rejects missing, malformed, out-of-range, or empty quota reports.
pub fn quota_from_value(value: &Value, now: i64) -> Result<QuotaSnapshot, &'static str> {
    if value
        .get("rateLimitsByLimitId")
        .is_some_and(|v| !v.is_null() && !v.is_object())
    {
        return Err("PLAN_QUOTA_INVALID");
    }
    let buckets: Vec<(&str, &Value)> = if let Some(map) = value
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
        .filter(|m| !m.is_empty())
    {
        map.iter()
            .map(|(id, bucket)| (id.as_str(), bucket))
            .collect()
    } else {
        vec![(
            "codex",
            value.get("rateLimits").ok_or("PLAN_QUOTA_UNAVAILABLE")?,
        )]
    };
    let mut windows = Vec::new();
    for (id, bucket) in buckets {
        if !bucket.is_object() || id.len() > 200 {
            return Err("PLAN_QUOTA_INVALID");
        }
        for window in ["primary", "secondary"] {
            let Some(data) = bucket.get(window).filter(|v| !v.is_null()) else {
                continue;
            };
            let used = data
                .get("usedPercent")
                .and_then(Value::as_f64)
                .filter(|n| n.is_finite() && (0.0..=100.0).contains(n))
                .ok_or("PLAN_QUOTA_INVALID")?;
            let duration = data
                .get("windowDurationMins")
                .filter(|v| !v.is_null())
                .map(|v| v.as_u64().filter(|n| *n > 0).ok_or("PLAN_QUOTA_INVALID"))
                .transpose()?;
            let reset = data
                .get("resetsAt")
                .filter(|v| !v.is_null())
                .map(|v| v.as_i64().filter(|n| *n >= 0).ok_or("PLAN_QUOTA_INVALID"))
                .transpose()?;
            // Do not persist arbitrary provider strings in monitoring diagnostics.
            windows.push(QuotaWindow {
                limit_id: id.into(),
                name: format!("Codex {window}"),
                window: window.into(),
                remaining_percent: 100.0 - used,
                window_duration_minutes: duration,
                resets_at: reset,
            });
        }
    }
    if windows.is_empty() {
        return Err("PLAN_QUOTA_UNAVAILABLE");
    }
    Ok(QuotaSnapshot {
        fetched_at_unix_ms: now,
        windows,
    })
}
impl QuotaSnapshot {
    #[must_use]
    pub fn blocking_window(&self, reserve: u8) -> Option<&QuotaWindow> {
        self.windows
            .iter()
            .filter(|w| w.remaining_percent < f64::from(reserve))
            .min_by(|a, b| a.remaining_percent.total_cmp(&b.remaining_percent))
    }
}
/// Normalize the latest cumulative report from a fresh, single-pass ephemeral
/// thread. Replace repeated events; never sum them or accept account totals.
/// Cached input and reasoning output are subsets of input and output.
#[must_use]
pub fn turn_usage(value: &Value) -> Option<crate::Usage> {
    let report = value
        .get("tokenUsage")?
        .get("total")
        .or_else(|| value.get("tokenUsage")?.get("last"))?;
    let input = report.get("inputTokens")?.as_u64()?;
    let output = report.get("outputTokens")?.as_u64()?;
    let cached = report.get("cachedInputTokens")?.as_u64()?;
    let reasoning = report.get("reasoningOutputTokens")?.as_u64()?;
    let cache_write = report
        .get("cacheWriteInputTokens")
        .map_or(Some(0), Value::as_u64)?;
    if cached.checked_add(cache_write).is_none_or(|v| v > input)
        || reasoning > output
        || input.checked_add(output).is_none()
    {
        return None;
    }
    Some(crate::Usage {
        input_tokens: input - cached - cache_write,
        cache_write_tokens: cache_write,
        output_tokens: output,
        cached_input_tokens: cached,
        reasoning_tokens: reasoning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn quotas_check_every_window_and_exact_boundary() {
        let quota = quota_from_value(&json!({"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":80},"secondary":{"usedPercent":81}},"other":{"primary":{"usedPercent":99}}}}),1).unwrap();
        assert!((quota.blocking_window(20).unwrap().remaining_percent - 1.0).abs() < f64::EPSILON);
        assert!(
            quota_from_value(&json!({"rateLimits":{"primary":{"usedPercent":80}}}), 1)
                .unwrap()
                .blocking_window(20)
                .is_none()
        );
        for bad in [
            json!({}),
            json!({"rateLimits":null}),
            json!({"rateLimits":{"primary":{"usedPercent":101}}}),
            json!({"rateLimits":{"primary":{"usedPercent":20,"resetsAt":"soon"}}}),
        ] {
            assert!(quota_from_value(&bad, 1).is_err());
        }
    }
    #[test]
    fn fresh_thread_usage_replaces_cumulative_reports_and_preserves_subsets() {
        let usage=turn_usage(&json!({"tokenUsage":{"total":{"inputTokens":100,"outputTokens":20,"cachedInputTokens":50,"reasoningOutputTokens":15},"last":{"inputTokens":20,"outputTokens":2,"cachedInputTokens":0,"reasoningOutputTokens":1}}})).unwrap();
        assert_eq!(
            usage.input_tokens + usage.cached_input_tokens + usage.output_tokens,
            120
        );
        assert!(turn_usage(&json!({"tokenUsage":{"total":{"inputTokens":100}}})).is_none());
    }
    #[test]
    fn models_never_substitute() {
        let models = models_from_catalog(
            &json!({"data":[{"model":"gpt-6.1-sol","supportedReasoningEfforts":[{"reasoningEffort":"medium"}]}]}),
        );
        assert_eq!(models.len(), 6);
        assert_eq!(models.iter().filter(|m| m.supported).count(), 1);
        assert!(
            !PlanSettings {
                reserve_percent: 0,
                ..PlanSettings::default()
            }
            .valid()
        );
    }
}
