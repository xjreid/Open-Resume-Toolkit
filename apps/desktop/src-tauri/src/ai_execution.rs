//! One durable request lifecycle for credential tests and application generation.
use super::{
    CancelSignal, DesktopState, SyntheticStreamState, category, http_client_with_timeout,
    now_unix_ms, outbound, provider_failure,
};
use ort_ai::{HttpRequest, MAX_STREAM_BYTES, ProviderAdapter, StreamEvent, estimate_priced_cost};
use ort_storage::{
    StorageError,
    ai_activity::{AiAttemptPreflight, AiAttemptSettlement, AiTerminalStatus},
};
use reqwest::Url;
use std::time::Duration;
use uuid::Uuid;

pub(super) struct Policy {
    pub timeout: Duration,
    pub retry_server_error: bool,
}
pub(super) struct Completed<T> {
    pub value: T,
    pub attempt_id: Uuid,
    pub cost: Option<u64>,
}
pub(super) struct Failure {
    pub code: &'static str,
    pub retryable: bool,
}
impl Failure {
    fn new(code: &'static str) -> Self {
        Self {
            code,
            retryable: matches!(
                code,
                "STORAGE_UNAVAILABLE" | "AI_PROVIDER_UNAVAILABLE" | "AI_BUSY" | "AI_USAGE_UNKNOWN"
            ),
        }
    }
    fn storage(_: StorageError) -> Self {
        Self::new("STORAGE_UNAVAILABLE")
    }
}

struct Attempt<'a> {
    state: &'a DesktopState,
    preflight: AiAttemptPreflight,
}
impl Attempt<'_> {
    fn reserve(&self) -> Result<(), Failure> {
        self.state
            .with_store(|store| store.reserve_ai_attempt(&self.preflight))
            .map_err(|error| {
                Failure::new(match error {
                    StorageError::InvalidData => "AI_CAP_REJECTED",
                    StorageError::RevisionConflict => "AI_BUSY",
                    _ => "STORAGE_UNAVAILABLE",
                })
            })
    }
    fn cancel_reserved(&self) -> Result<(), Failure> {
        self.state
            .with_store(|store| {
                store.cancel_reserved_ai_attempt(self.preflight.attempt_id, self.ended())
            })
            .map_err(Failure::storage)
    }
    fn ended(&self) -> i64 {
        now_unix_ms().unwrap_or(self.preflight.started_at_unix_ms)
    }
    fn settle(
        &self,
        status: AiTerminalStatus,
        category: Option<&str>,
        evidence: Option<&SyntheticStreamState>,
        keep_active: bool,
    ) -> Result<Option<u64>, Failure> {
        let result = settlement(
            &self.preflight,
            status,
            category,
            evidence,
            self.ended(),
            keep_active,
        );
        let cost = result.settled_cost_micros;
        self.state
            .with_store(|store| store.settle_ai_attempt(&result))
            .map_err(Failure::storage)?;
        Ok(cost)
    }
    fn failed(&self, code: &'static str, status: AiTerminalStatus, category: &str) -> Failure {
        match self.settle(status, Some(category), None, false) {
            Ok(_) => Failure::new(code),
            Err(error) => error,
        }
    }
}

// Billing evidence belongs to the attempt, even when its content is rejected.
fn settlement(
    preflight: &AiAttemptPreflight,
    status: AiTerminalStatus,
    category: Option<&str>,
    evidence: Option<&SyntheticStreamState>,
    ended: i64,
    keep_active: bool,
) -> AiAttemptSettlement {
    let model = evidence.and_then(|output| output.effective_model.clone());
    let usage = evidence.and_then(|output| output.usage);
    let cost = (model.as_deref() == Some(preflight.requested_model.as_str()))
        .then(|| {
            usage.and_then(|usage| {
                estimate_priced_cost(preflight.provider, &preflight.pricing_components, usage).ok()
            })
        })
        .flatten()
        .filter(|cost| *cost <= preflight.maximum_cost_micros);
    AiAttemptSettlement {
        attempt_id: preflight.attempt_id,
        status,
        effective_model: model,
        usage,
        settled_cost_micros: cost,
        usage_complete: cost.is_some(),
        error_category: category.map(str::to_owned),
        ended_at_unix_ms: ended,
        keep_operation_active: keep_active,
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) async fn execute<T>(
    state: &DesktopState,
    signal: &CancelSignal,
    preflight: AiAttemptPreflight,
    request: HttpRequest,
    url: Url,
    adapter: &(dyn ProviderAdapter + Send + Sync),
    policy: Policy,
    progress: impl Fn(&'static str, String),
    validate: impl FnOnce(&SyntheticStreamState) -> Result<T, &'static str>,
) -> Result<Completed<T>, Failure> {
    let mut attempt = Attempt { state, preflight };
    attempt.reserve()?;
    let Ok(client) = http_client_with_timeout(policy.timeout) else {
        attempt.cancel_reserved()?;
        return Err(Failure::new("AI_PROVIDER_UNAVAILABLE"));
    };
    let Ok(outgoing) = outbound(&client, url, &request) else {
        attempt.cancel_reserved()?;
        return Err(Failure::new("AI_PROVIDER_INVALID"));
    };
    drop(request);
    let mut next = Some(outgoing);
    let mut response = loop {
        if signal.is_cancelled() {
            attempt.cancel_reserved()?;
            return Err(Failure::new("AI_CANCELLED"));
        }
        let outgoing = next
            .take()
            .ok_or_else(|| Failure::new("AI_PROVIDER_INVALID"))?;
        let retry = if policy.retry_server_error && attempt.preflight.retry_of.is_none() {
            outgoing.try_clone()
        } else {
            None
        };
        if let Err(error) =
            state.with_store(|store| store.mark_ai_dispatching(attempt.preflight.attempt_id))
        {
            let _ = attempt.cancel_reserved();
            return Err(Failure::storage(error));
        }
        let response = tokio::select! {
            result = outgoing.send() => result.map_err(|_| attempt.failed("AI_PROVIDER_UNAVAILABLE", AiTerminalStatus::OutcomeUnknown, "transient"))?,
            () = signal.wait() => return Err(attempt.failed("AI_CANCELLED", AiTerminalStatus::Cancelled, "cancelled")),
        };
        if response.status().is_server_error()
            && let Some(retry) = retry
        {
            attempt.settle(AiTerminalStatus::Failed, Some("transient"), None, true)?;
            tokio::select! {
                () = tokio::time::sleep(Duration::from_secs(1)) => {},
                () = signal.wait() => {
                    state.with_store(|store| store.finish_ai_operation(attempt.preflight.operation_id, AiTerminalStatus::Cancelled, attempt.ended())).map_err(Failure::storage)?;
                    return Err(Failure::new("AI_CANCELLED"));
                }
            }
            let previous = attempt.preflight.attempt_id;
            attempt.preflight.attempt_id = Uuid::now_v7();
            attempt.preflight.started_at_unix_ms = attempt.ended();
            attempt.preflight.retry_of = Some(previous);
            if attempt.reserve().is_err() {
                state
                    .with_store(|store| {
                        store.finish_ai_operation(
                            attempt.preflight.operation_id,
                            AiTerminalStatus::Failed,
                            attempt.ended(),
                        )
                    })
                    .map_err(Failure::storage)?;
                return Err(Failure::new("AI_RETRY_BLOCKED"));
            }
            progress("retry", String::new());
            next = Some(retry);
            continue;
        }
        if !response.status().is_success() {
            let (code, retryable) = provider_failure(response.status());
            let mut error =
                attempt.failed(code, AiTerminalStatus::Failed, category(response.status()));
            if error.code == code {
                error.retryable = retryable;
            }
            return Err(error);
        }
        break response;
    };
    state
        .with_store(|store| store.mark_ai_streaming(attempt.preflight.attempt_id))
        .map_err(|_| {
            attempt.failed(
                "STORAGE_UNAVAILABLE",
                AiTerminalStatus::OutcomeUnknown,
                "provider",
            )
        })?;
    let mut raw = Vec::new();
    let mut scan = 0;
    loop {
        let chunk = tokio::select! {
            result = response.chunk() => result.map_err(|_| attempt.failed("AI_PROVIDER_UNAVAILABLE", AiTerminalStatus::OutcomeUnknown, "transient"))?,
            () = signal.wait() => return Err(attempt.failed("AI_CANCELLED", AiTerminalStatus::Cancelled, "cancelled")),
        };
        let Some(chunk) = chunk else {
            break;
        };
        if raw
            .len()
            .checked_add(chunk.len())
            .is_none_or(|size| size > MAX_STREAM_BYTES)
        {
            return Err(attempt.failed(
                "AI_OUTPUT_INVALID",
                AiTerminalStatus::Failed,
                "invalid_output",
            ));
        }
        raw.extend_from_slice(&chunk);
        while let Some(relative) = raw[scan..].iter().position(|byte| *byte == b'\n') {
            let end = scan + relative + 1;
            if let Ok(events) = adapter.parse_stream(&raw[scan..end]) {
                for event in events {
                    if let StreamEvent::Text(text) = event {
                        progress("delta", text);
                    }
                }
            }
            scan = end;
        }
    }
    let events = adapter.parse_stream(&raw).map_err(|_| {
        attempt.failed(
            "AI_OUTPUT_INVALID",
            AiTerminalStatus::Failed,
            "invalid_output",
        )
    })?;
    let output = SyntheticStreamState::from_events(events);
    let result = validate(&output).and_then(|value| {
        if output.usage.is_some() {
            Ok(value)
        } else {
            Err("AI_USAGE_UNKNOWN")
        }
    });
    let status = if output.usage.is_none() {
        AiTerminalStatus::OutcomeUnknown
    } else if result.is_ok() {
        AiTerminalStatus::Succeeded
    } else {
        AiTerminalStatus::Failed
    };
    let cost = attempt.settle(
        status,
        result.as_ref().err().map(|_| "invalid_output"),
        Some(&output),
        false,
    )?;
    let value = result.map_err(Failure::new)?;
    progress("finished", String::new());
    Ok(Completed {
        value,
        attempt_id: attempt.preflight.attempt_id,
        cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_ai::{OperationType, Price, PriceCategory, Provider, Usage};
    use ort_storage::{EncryptedStore, ai_activity::AiBucketSize};
    use ort_vault::testing::MemoryDatabaseKeyVault;

    #[test]
    fn rejected_content_settles_trustworthy_billing_in_the_durable_ledger() {
        let root = tempfile::TempDir::new().unwrap();
        let store =
            EncryptedStore::open_or_initialize(root.path(), "test", &MemoryDatabaseKeyVault::new())
                .unwrap();
        let preflight = AiAttemptPreflight {
            operation_id: Uuid::now_v7(),
            attempt_id: Uuid::now_v7(),
            operation_type: OperationType::TailorResume,
            provider: Provider::Gemini,
            credential_id: Uuid::now_v7(),
            requested_model: "fixture-model".into(),
            preset_version: "v1".into(),
            catalog_id: "fixture-catalog".into(),
            catalog_effective_from: "2026-09-13T00:00:00Z".into(),
            pricing_components: vec![
                Price {
                    category: PriceCategory::Input,
                    micros_per_million: 1_000_000,
                },
                Price {
                    category: PriceCategory::Output,
                    micros_per_million: 1_000_000,
                },
            ],
            started_at_unix_ms: 1000,
            estimated_input_tokens: 100,
            maximum_cost_micros: 1000,
            currency: "USD".into(),
            retry_of: None,
        };
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 20,
            ..Usage::default()
        };
        let mut evidence = SyntheticStreamState::from_events(vec![
            StreamEvent::Model("fixture-model".into()),
            StreamEvent::Usage(usage),
            StreamEvent::ProviderFailure,
        ]);
        assert_eq!(
            evidence.material_error("fixture-model"),
            Some("AI_OUTPUT_INVALID")
        );
        let result = settlement(
            &preflight,
            AiTerminalStatus::Failed,
            Some("invalid_output"),
            Some(&evidence),
            2000,
            false,
        );
        assert_eq!(result.settled_cost_micros, Some(120));
        store.reserve_ai_attempt(&preflight).unwrap();
        store.mark_ai_dispatching(preflight.attempt_id).unwrap();
        store.settle_ai_attempt(&result).unwrap();
        let summary = store
            .ai_monitoring_summary(0, 3000, "UTC", AiBucketSize::Day)
            .unwrap();
        assert_eq!(summary.usage, usage);
        assert_eq!(summary.cost_by_currency_micros.get("USD"), Some(&120));
        assert_eq!(summary.unresolved_reserved_micros, 0);
        evidence.effective_model = Some("untrusted-model".into());
        let uncertain = settlement(
            &preflight,
            AiTerminalStatus::Failed,
            Some("invalid_output"),
            Some(&evidence),
            2000,
            false,
        );
        assert_eq!(uncertain.usage, Some(usage));
        assert_eq!(uncertain.settled_cost_micros, None);
        assert!(!uncertain.usage_complete);
    }
}
