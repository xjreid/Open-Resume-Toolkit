//! Semantic session lifetimes above the stdio protocol: one thread per pass,
//! and quota availability that never silently retains a failed refresh.
use crate::codex_runtime::Session;
use ort_ai::plan::{QuotaSnapshot, quota_from_value};
use serde_json::Value;
use std::time::Instant;

pub(crate) fn requires_retirement(code: &str) -> bool {
    matches!(
        code,
        "AI_CANCELLED"
            | "PLAN_REQUEST_TIMEOUT"
            | "PLAN_CONTAINMENT_VIOLATION"
            | "PLAN_PROTOCOL_INVALID"
            | "PLAN_RUNTIME_UNAVAILABLE"
    )
}

impl Session {
    pub(crate) fn refresh_quota(
        &mut self,
        end: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<QuotaSnapshot, &'static str> {
        self.quota = None;
        let result = self
            .rpc(
                "account/rateLimits/read",
                serde_json::json!({}),
                end.saturating_duration_since(Instant::now()),
                cancel,
            )
            .and_then(|value| quota_from_value(&value, jiff::Timestamp::now().as_millisecond()));
        self.quota = result.as_ref().ok().cloned();
        result
    }

    pub(crate) fn with_ephemeral_thread<T>(
        &mut self,
        params: Value,
        end: Instant,
        cancel: &dyn Fn() -> bool,
        f: impl FnOnce(&mut Self, &Value, &str) -> Result<T, &'static str>,
    ) -> Result<T, &'static str> {
        let thread = self.rpc(
            "thread/start",
            params,
            end.saturating_duration_since(Instant::now()),
            cancel,
        )?;
        let Some(id) = thread.pointer("/thread/id").and_then(Value::as_str) else {
            self.terminate();
            return Err("PLAN_PROTOCOL_INVALID");
        };
        let result = f(self, &thread, id);
        if result.as_ref().is_err_and(|code| requires_retirement(code)) {
            self.terminate();
        } else {
            // This covers model validation, accounting, and turn-start errors,
            // as well as completed turns. Failed release retires the process.
            self.release_thread(id, end, cancel)?;
        }
        result
    }
}
