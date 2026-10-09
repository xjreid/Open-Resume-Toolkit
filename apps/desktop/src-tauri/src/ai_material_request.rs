//! One material stage loop for both providers, with one logical operation owner.
use super::material_provider::{CompletedPass, MaterialProvider, PassContext};
use super::{AiRequestGate, CancelSignal, MaterialFailure, execution, now_unix_ms};
use crate::DesktopState;
use ort_ai::OperationType;
use ort_storage::ai_activity::AiTerminalStatus;
use serde_json::{Value, json};
use std::sync::Arc;
use tauri::{Emitter, Manager, WebviewWindow};
use uuid::Uuid;

pub(crate) async fn execute_material_sequence<T>(
    window: &WebviewWindow,
    operation: OperationType,
    system: &str,
    input: Value,
    max_calls: u8,
    expected_profile: Option<Uuid>,
    mut advance: impl FnMut(&str, u8, &dyn Fn() -> bool) -> Result<MaterialDecision<T>, &'static str>,
) -> Result<T, MaterialFailure> {
    if window.label() != "overlay" {
        return Err("WINDOW_NOT_AUTHORIZED".into());
    }
    let app = window.app_handle().clone();
    let state = app.state::<DesktopState>();
    let gate = app.state::<AiRequestGate>();
    let operation_id = Uuid::now_v7();
    let lease = gate.begin_overlay(operation_id).ok_or("AI_BUSY")?;
    let _ui_operation = OperationUiGuard(window);
    let _ = window.app_handle().emit("ort:ai-operation-state", true);
    let (provider, profile) = MaterialProvider::load(&state, operation)?;
    if expected_profile.is_some_and(|expected| expected != profile) {
        return Err("REVISION_CONFLICT".into());
    }
    let maximum = max_calls.min(4);
    let mut logical = MaterialOperation {
        state: &state,
        profile,
        operation_id,
        signal: Arc::clone(&lease.signal),
        status: AiTerminalStatus::Failed,
    };
    let cancelled = || lease.signal.is_cancelled();
    let provider = &provider;
    let state = &*state;
    let signal = &lease.signal;
    let result = run_stages(
        maximum,
        input,
        &cancelled,
        |input, call, previous| async move {
            state
                .with_store(|store| {
                    ort_application::application_workspace::ensure_profile(store, profile)
                })
                .map_err(|_| "REVISION_CONFLICT")?;
            if maximum > 1 {
                let phase = match call {
                    1 => "Drafting",
                    2 => "Checking sources and editing",
                    4 => "Final revision",
                    _ => "Correcting",
                };
                let _ = window.emit(
                    "ort:tailoring-progress",
                    json!({
                        "phase": phase,
                        "call": call,
                        "maximum": maximum,
                        "connectionSource": provider.source(),
                        "unit": if provider.source() == "chatgpt_plan" { "pass" } else { "call" },
                    }),
                );
            }
            provider
                .execute(PassContext {
                    window,
                    state,
                    profile,
                    operation_id,
                    operation,
                    system,
                    input,
                    call,
                    maximum,
                    previous,
                    signal: Arc::clone(signal),
                })
                .await
        },
        |completed, call| {
            advance(&completed.text, call, &cancelled).map_err(|code| {
                let details = execution::failure_details(code);
                if code != "AI_CANCELLED" {
                    let _ = state.with_store(|store| {
                        ort_application::application_workspace::ensure_profile(store, profile)?;
                        store.record_ai_failure_details(completed.attempt_id, &details)
                    });
                }
                completed.failure(code, &details, operation_id, operation, call, maximum)
            })
        },
    )
    .await;
    if result.is_ok() {
        logical.status = AiTerminalStatus::Succeeded;
    }
    result
}

async fn run_stages<T, F>(
    maximum: u8,
    mut input: Value,
    cancelled: &(dyn Fn() -> bool + Sync),
    mut dispatch: impl FnMut(Value, u8, Option<Uuid>) -> F,
    mut advance: impl FnMut(&CompletedPass, u8) -> Result<MaterialDecision<T>, MaterialFailure>,
) -> Result<T, MaterialFailure>
where
    F: std::future::Future<Output = Result<CompletedPass, MaterialFailure>> + Send,
{
    let mut previous = None;
    for call in 1..=maximum.min(4) {
        if cancelled() {
            return Err("AI_CANCELLED".into());
        }
        let completed = dispatch(input, call, previous).await?;
        previous = Some(completed.attempt_id);
        if cancelled() {
            return Err("AI_CANCELLED".into());
        }
        match advance(&completed, call)? {
            MaterialDecision::Complete(value) => return Ok(value),
            MaterialDecision::Continue(next) => input = next,
        }
    }
    Err("AI_TAILORING_FAILED".into())
}

struct OperationUiGuard<'a>(&'a WebviewWindow);
impl Drop for OperationUiGuard<'_> {
    fn drop(&mut self) {
        let _ = self.0.app_handle().emit("ort:ai-operation-state", false);
    }
}

// Keep the ledger's logical operation open between paid semantic stages, and
// close it on every exit path (including cap failure and local validation).
struct MaterialOperation<'a> {
    state: &'a DesktopState,
    profile: Uuid,
    operation_id: Uuid,
    signal: Arc<CancelSignal>,
    status: AiTerminalStatus,
}
impl Drop for MaterialOperation<'_> {
    fn drop(&mut self) {
        let status = if self.status != AiTerminalStatus::Succeeded && self.signal.is_cancelled() {
            AiTerminalStatus::Cancelled
        } else {
            self.status
        };
        let _ = self.state.with_store(|store| {
            ort_application::application_workspace::ensure_profile(store, self.profile)?;
            store.finish_ai_operation(
                self.operation_id,
                status,
                now_unix_ms().ok_or(ort_storage::StorageError::Unavailable)?,
            )
        });
    }
}

pub(crate) enum MaterialDecision<T> {
    Complete(T),
    Continue(Value),
}

pub(crate) async fn execute_material<T>(
    window: &WebviewWindow,
    operation: OperationType,
    system: &str,
    input: Value,
    validate: impl FnOnce(&str) -> Result<T, ()>,
) -> Result<T, MaterialFailure> {
    let mut validate = Some(validate);
    execute_material_sequence(window, operation, system, input, 1, None, |raw, _, _| {
        validate.take().ok_or("AI_OUTPUT_INVALID")?(raw)
            .map(MaterialDecision::Complete)
            .map_err(|()| "AI_OUTPUT_INVALID")
    })
    .await
}

#[cfg(test)]
#[path = "ai_material_request_tests.rs"]
mod tests;
