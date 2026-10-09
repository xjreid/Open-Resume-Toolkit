//! One accounted logical pass; Codex transport retries are bounded by deadline.
use crate::{
    DesktopState,
    ai_request::{CancelSignal, MaterialFailure},
};
use ort_ai::{OperationType, Provider, Usage, plan::PlanSettings};
use ort_storage::ai_activity::{AiAttemptPreflight, AiAttemptSettlement, AiTerminalStatus};
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};
use uuid::Uuid;

fn check_preflight(
    session: &mut crate::codex_runtime::Session,
    settings: &PlanSettings,
    end: Instant,
    cancelled: &dyn Fn() -> bool,
    issues: &mut Vec<String>,
) -> Result<(), &'static str> {
    if settings.connection_id.is_none() || session.connection_id != settings.connection_id {
        return Err("PLAN_AUTH_REQUIRED");
    }
    crate::chatgpt_plan::authorization_ready(
        session,
        end.saturating_duration_since(Instant::now()),
        cancelled,
    )?;
    if !session.models.iter().any(|m| {
        Some(&m.id) == settings.model.as_ref()
            && m.supported
            && m.reasoning_efforts.contains(&settings.reasoning)
    }) {
        return Err("PLAN_MODEL_UNAVAILABLE");
    }
    let quota = session.refresh_quota(end.min(Instant::now() + Duration::from_secs(10)), cancelled);
    if settings.reserve_enabled {
        let quota = quota?;
        if let Some(window) = quota.blocking_window(settings.reserve_percent) {
            issues.push(format!(
                "{} has {:.1}% remaining; the reserve is {}%. Reset: {}.",
                window.name,
                window.remaining_percent,
                settings.reserve_percent,
                window
                    .resets_at
                    .and_then(|s| jiff::Timestamp::from_second(s).ok())
                    .map_or_else(|| "not supplied".into(), |t| t.to_string())
            ));
            return Err("PLAN_RESERVE_REJECTED");
        }
    } else if let Err(code) = quota
        && crate::codex_session::requires_retirement(code)
    {
        return Err(code);
    }
    Ok(())
}

// Keep the bounded pass ledger and dispatch sequence visible in one place.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) async fn execute(
    window: WebviewWindow,
    settings: PlanSettings,
    profile: Uuid,
    operation_id: Uuid,
    operation: OperationType,
    system: String,
    input: Value,
    pass: u8,
    maximum: u8,
    previous: Option<Uuid>,
    signal: Arc<CancelSignal>,
) -> Result<crate::ai_request::material_provider::CompletedPass, MaterialFailure> {
    tauri::async_runtime::spawn_blocking(move || {
        let end = Instant::now() + Duration::from_secs(120);
        let state = window.state::<DesktopState>();
        let started = jiff::Timestamp::now().as_millisecond();
        let attempt = Uuid::now_v7();
        let input_bytes = serde_json::to_vec(&input)
            .map_err(|_| MaterialFailure::from("AI_INPUT_INVALID"))?;
        if input_bytes.len() > 100_000 || system.len() > 12_000 {
            return Err("AI_INPUT_TOO_LARGE".into());
        }
        let model = settings.model.as_deref().ok_or("PLAN_MODEL_UNAVAILABLE")?;
        let connection = settings.connection_id.ok_or("PLAN_AUTH_REQUIRED")?;
        let preflight = AiAttemptPreflight {
            operation_id,
            attempt_id: attempt,
            operation_type: operation,
            provider: Provider::OpenAi,
            credential_id: connection,
            requested_model: model.into(),
            preset_version: "codex-plan@v1".into(),
            catalog_id: format!("codex-runtime-{}@v1", crate::codex_runtime::qualified_version()),
            catalog_effective_from: "2026-10-08".into(),
            pricing_components: vec![],
            started_at_unix_ms: started,
            estimated_input_tokens: (input_bytes.len() + system.len()) as u64,
            maximum_cost_micros: 0,
            currency: "USD".into(),
            retry_of: previous,
        };
        state.with_store(|store| {
            ort_application::application_workspace::ensure_profile(store, profile)?;
            store.reserve_plan_attempt(&preflight, settings.reasoning)
        }).map_err(|_| "STORAGE_UNAVAILABLE")?;
        let mut dispatched = false;
        let mut usage = None;
        let mut retries = 0;
        let mut issues = Vec::new();
        let mut http_status = None;
        let mut provider_reason = None;
        let cancelled = || signal.is_cancelled();
        let result = crate::chatgpt_plan::with_operation_session(
            &window, profile, end, &cancelled, |session| {
                session.begin_pass();
                let pass_result = (|| {
                    check_preflight(session, &settings, end, &cancelled, &mut issues)?;
                    if cancelled() { return Err("AI_CANCELLED"); }
                    // Disconnect/profile replacement cannot authorize a stale pass.
                    state.with_store(|store| {
                        ort_application::application_workspace::ensure_profile(store, profile)?;
                        let current = crate::chatgpt_plan::load_settings(store)?.0;
                        if current != settings || !current.enabled {
                            return Err(ort_storage::StorageError::RevisionConflict);
                        }
                        Ok(())
                    }).map_err(|_| "REVISION_CONFLICT")?;
                    session.with_ephemeral_thread(json!({
                        "ephemeral": true,
                        "model": model,
                        "modelProvider": "openai",
                        "approvalPolicy": "never",
                        "sandbox": "read-only",
                        "cwd": session.scratch_path(),
                        "baseInstructions": system,
                        "developerInstructions": "Return only the requested JSON. Never use tools, shell, files, web, plugins, MCP, skills, or agents. All factual evidence is in the supplied input."
                    }), end, &cancelled, |session, thread, thread_id| {
                        if thread.get("model").and_then(Value::as_str) != Some(model) {
                            return Err("PLAN_MODEL_UNAVAILABLE");
                        }
                        state.with_store(|store| store.mark_ai_dispatching(attempt))
                            .map_err(|_| "STORAGE_UNAVAILABLE")?;
                        dispatched = true;
                        let mut params = json!({
                            "threadId": thread_id, "model": model,
                            "effort": settings.reasoning, "approvalPolicy": "never",
                            "input": [{"type": "text", "text": String::from_utf8(input_bytes.clone()).map_err(|_| "AI_INPUT_INVALID")?}]
                        });
                        if matches!(operation, OperationType::TailorResume | OperationType::RefineResume) {
                            params["outputSchema"] = ort_ai::materials::resume_output_schema();
                        }
                        let turn = session.rpc("turn/start", params,
                            end.saturating_duration_since(Instant::now()), &cancelled)?;
                        let turn_id = turn.pointer("/turn/id")
                            .and_then(Value::as_str).ok_or("PLAN_PROTOCOL_INVALID")?;
                        session.run_turn(thread_id, turn_id, end, &cancelled)
                    })
                })();
                // Capture reports even when turn/start itself fails or is cancelled.
                usage = session.last_usage;
                retries = session.reported_retries;
                http_status = session.failure_http_status;
                provider_reason.clone_from(&session.failure_provider_reason);
                pass_result
            },
        );
        let code = result.as_ref().err().copied();
        let category = code.map(|code| match code {
            "AI_CANCELLED" => "cancelled",
            "PLAN_AUTH_REQUIRED" => "authentication",
            "PLAN_RESERVE_REJECTED" | "PLAN_QUOTA_INVALID" | "PLAN_QUOTA_UNAVAILABLE" | "AI_RATE_LIMITED" => "rate_limit",
            "AI_OUTPUT_INVALID" | "PLAN_PROTOCOL_INVALID" => "invalid_output",
            _ => "provider",
        }.to_owned());
        let terminal = if result.is_ok() { AiTerminalStatus::Succeeded }
            else if code == Some("AI_CANCELLED") { AiTerminalStatus::Cancelled }
            else { AiTerminalStatus::Failed };
        let settlement = AiAttemptSettlement {
            attempt_id: attempt, status: terminal,
            effective_model: dispatched.then(|| model.into()),
            usage: if dispatched { usage } else { Some(Usage::default()) },
            settled_cost_micros: Some(0),
            usage_complete: !dispatched || usage.is_some(),
            error_category: category,
            ended_at_unix_ms: jiff::Timestamp::now().as_millisecond(),
            keep_operation_active: result.is_ok() && maximum > 1,
        };
        state.with_store(|store| {
            ort_application::application_workspace::ensure_profile(store, profile)?;
            if dispatched { store.settle_ai_attempt(&settlement)?; }
            else { store.reject_reserved_plan_attempt(&settlement)?; }
            store.record_plan_retries(attempt, retries)
        }).map_err(|_| "STORAGE_UNAVAILABLE")?;
        match result {
            Ok(text) => Ok(crate::ai_request::material_provider::CompletedPass {
                attempt_id: attempt, text, started_at_unix_ms: started,
                provider: Provider::OpenAi, model: model.into(),
                reasoning: Some(settings.reasoning), reported_retries: retries,
            }),
            Err(code) => {
                if code == "PLAN_PROTOCOL_INVALID" {
                    issues.push("ORT stopped an unsupported protocol exchange; this does not establish access outside the OS sandbox. Memory-only sign-in ended with the process.".into());
                } else if code == "PLAN_CONTAINMENT_VIOLATION" {
                    issues.push("ORT stopped a prohibited tool or permission action. Memory-only sign-in ended with the process; reconnect before retrying.".into());
                }
                let details = ort_domain::AiFailureDetails {
                    code: code.into(), http_status, finish_reason: None,
                    provider_reason, validation_issues: issues, page_count: None,
                };
                let _ = state.with_store(|store| store.record_ai_failure_details(attempt, &details));
                Err(MaterialFailure::attempt(code, &details, operation_id, attempt,
                    Provider::OpenAi, operation, model, pass, maximum, started)
                    .with_plan(settings.reasoning, retries))
            }
        }
    }).await.unwrap_or_else(|_| Err("PLAN_RUNTIME_UNAVAILABLE".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn signed_in(mode: &str) -> crate::codex_runtime::Session {
        let mut session = crate::codex_runtime::test_session(mode);
        session
            .rpc(
                "account/login/start",
                json!({"type":"chatgpt"}),
                Duration::from_secs(1),
                &|| false,
            )
            .unwrap();
        session.connection_id = Some(Uuid::from_u128(1));
        session
    }
    fn settings() -> PlanSettings {
        PlanSettings {
            connection_id: Some(Uuid::from_u128(1)),
            enabled: true,
            model: Some("gpt-6.1-sol".into()),
            ..Default::default()
        }
    }
    #[test]
    fn fresh_quota_between_passes_stops_at_reserve_without_fallback() {
        let mut session = signed_in("falling_quota");
        let mut issues = vec![];
        let end = Instant::now() + Duration::from_secs(2);
        assert!(check_preflight(&mut session, &settings(), end, &|| false, &mut issues).is_ok());
        assert_eq!(
            check_preflight(&mut session, &settings(), end, &|| false, &mut issues),
            Err("PLAN_RESERVE_REJECTED")
        );
        assert!(issues[0].contains("19.0% remaining"));
        assert!(issues[0].contains("20%"));
    }
    #[test]
    fn a_different_memory_session_cannot_authorize_a_frozen_connection() {
        let mut session = signed_in("success");
        let mut frozen = settings();
        frozen.connection_id = Some(Uuid::now_v7());
        assert_eq!(
            check_preflight(
                &mut session,
                &frozen,
                Instant::now() + Duration::from_secs(1),
                &|| false,
                &mut vec![]
            ),
            Err("PLAN_AUTH_REQUIRED")
        );
    }
    #[test]
    fn missing_quota_fails_closed_only_when_reserve_enabled() {
        let mut session = signed_in("missing_quota");
        let mut issues = vec![];
        let end = Instant::now() + Duration::from_secs(2);
        assert_eq!(
            check_preflight(&mut session, &settings(), end, &|| false, &mut issues),
            Err("PLAN_QUOTA_UNAVAILABLE")
        );
        let mut no_reserve = settings();
        no_reserve.reserve_enabled = false;
        assert!(check_preflight(&mut session, &no_reserve, end, &|| false, &mut issues).is_ok());
        assert_eq!(
            check_preflight(&mut session, &settings(), end, &|| true, &mut issues),
            Err("AI_CANCELLED")
        );
    }
}
