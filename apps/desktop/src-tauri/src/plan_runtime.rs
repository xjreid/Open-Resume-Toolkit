//! Owns the serialized protocol and a non-secret snapshot for responsive UI reads.
use crate::{chatgpt_plan::PlanStatus, codex_runtime::Session};
use ort_ai::plan::{PlanModel, PlanSettings, QuotaSnapshot};
use serde_json::json;
use std::{
    sync::{
        Arc, Mutex, TryLockError,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone)]
struct SessionView {
    connection_id: Option<Uuid>,
    account_plan: Option<String>,
    models: Vec<PlanModel>,
    quota: Option<QuotaSnapshot>,
    login_pending: bool,
    error: Option<&'static str>,
}
impl From<&Session> for SessionView {
    fn from(session: &Session) -> Self {
        Self {
            connection_id: session.connection_id,
            account_plan: session.account_plan.clone(),
            models: session.models.clone(),
            quota: session.quota.clone(),
            login_pending: session.login_id.is_some(),
            error: session.login_error,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SessionAccess<'a> {
    pub cancel: &'a dyn Fn() -> bool,
    pub permit: &'a dyn Fn() -> Result<(), &'static str>,
    pub skip_busy: bool,
}

#[derive(Default)]
pub(crate) struct PlanRuntime {
    session: Mutex<Option<(Uuid, Session)>>,
    view: Mutex<Option<(Uuid, SessionView)>>,
    interrupt: Arc<AtomicBool>,
    shutting_down: AtomicBool,
}
impl PlanRuntime {
    pub(crate) fn interrupted(&self) -> bool {
        self.interrupt.load(Ordering::Acquire) || self.shutting_down.load(Ordering::Acquire)
    }
    pub(crate) fn login_pending(&self) -> bool {
        self.view
            .lock()
            .is_ok_and(|view| view.as_ref().is_some_and(|(_, s)| s.login_pending))
    }

    fn publish(&self, profile: Uuid, session: Option<&Session>) {
        *self
            .view
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            session.map(|session| (profile, session.into()));
    }

    pub(crate) fn status(
        &self,
        profile: Uuid,
        settings: PlanSettings,
        revision: Option<i64>,
        busy: bool,
    ) -> PlanStatus {
        let view = self
            .view
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = view
            .as_ref()
            .filter(|(id, _)| *id == profile && settings.enabled && !settings.cleanup_required)
            .map(|(_, view)| view);
        let connected = !settings.cleanup_required
            && session.is_some_and(|s| {
                s.connection_id.is_some()
                    && s.connection_id == settings.connection_id
                    && s.account_plan.is_some()
            });
        PlanStatus {
            error_code: if settings.cleanup_required {
                Some("PLAN_CREDENTIAL_CLEANUP_REQUIRED".into())
            } else {
                session
                    .and_then(|s| s.error)
                    .or_else(|| {
                        (settings.enabled
                            && !connected
                            && settings.connection_id.is_some()
                            && !session.is_some_and(|s| s.login_pending))
                        .then_some("PLAN_AUTH_REQUIRED")
                    })
                    .map(str::to_owned)
            },
            settings,
            revision,
            connected,
            operation_active: busy,
            account_plan: session.and_then(|s| s.account_plan.clone()),
            login_pending: session.is_some_and(|s| s.login_pending),
            runtime_version: session.map(|_| crate::codex_runtime::qualified_version().into()),
            models: session.map_or_else(
                || ort_ai::plan::models_from_catalog(&json!({})),
                |s| s.models.clone(),
            ),
            quota: session.and_then(|s| s.quota.clone()),
        }
    }

    pub(crate) fn is_connected(&self, profile: Uuid, connection_id: Option<Uuid>) -> bool {
        let settings = PlanSettings {
            enabled: true,
            connection_id,
            ..Default::default()
        };
        self.status(profile, settings, None, false).connected
    }

    pub(crate) fn validate_selection(
        &self,
        profile: Uuid,
        settings: &PlanSettings,
    ) -> Result<(), &'static str> {
        let status = self.status(profile, settings.clone(), None, false);
        if !status.connected {
            return Err("PLAN_AUTH_REQUIRED");
        }
        if !status.models.iter().any(|m| {
            Some(&m.id) == settings.model.as_ref()
                && m.supported
                && m.reasoning_efforts.contains(&settings.reasoning)
        }) {
            return Err("PLAN_MODEL_UNAVAILABLE");
        }
        Ok(())
    }

    /// UI polls never queue behind a quota RPC or a workflow pass. Mutations
    /// use the local snapshot; authorization/quota are revalidated before dispatch.
    pub(crate) fn with_session<T>(
        &self,
        profile: Uuid,
        end: Instant,
        access: SessionAccess<'_>,
        start: impl FnOnce(&dyn Fn() -> bool) -> Result<Session, &'static str>,
        f: impl FnOnce(&mut Session) -> Result<T, &'static str>,
    ) -> Result<Option<T>, &'static str> {
        let SessionAccess {
            cancel,
            permit,
            skip_busy,
        } = access;
        let mut current = loop {
            if cancel() || self.shutting_down.load(Ordering::Acquire) {
                return Err("AI_CANCELLED");
            }
            if Instant::now() >= end {
                return Err("PLAN_REQUEST_TIMEOUT");
            }
            match self.session.try_lock() {
                Ok(guard) => break guard,
                Err(TryLockError::Poisoned(_)) => return Err("PLAN_RUNTIME_UNAVAILABLE"),
                Err(TryLockError::WouldBlock) if skip_busy => return Ok(None),
                Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(20)),
            }
        };
        // Recheck permission while owning the process lock. A poll queued before
        // disable/sign-out cannot recreate or reuse the retired session.
        if let Err(code) = permit() {
            *current = None;
            self.publish(profile, None);
            return Err(code);
        }
        self.interrupt.store(false, Ordering::Release);
        // An approved quit permanently retires this manager. In particular, a
        // poll already queued before shutdown must not restart the process or
        // clear the cancellation signal while native termination is pending.
        if self.shutting_down.load(Ordering::Acquire) {
            self.interrupt.store(true, Ordering::Release);
            return Err("AI_CANCELLED");
        }
        if current.as_ref().is_none_or(|(id, _)| *id != profile) {
            self.publish(profile, None);
            *current = None;
            let mut session = start(&|| cancel() || self.interrupted())?;
            session.attach_interrupt(Arc::clone(&self.interrupt));
            *current = Some((profile, session));
        }
        let session = &mut current.as_mut().ok_or("PLAN_RUNTIME_UNAVAILABLE")?.1;
        let result = if session.is_running() {
            f(session)
        } else {
            Err("PLAN_RUNTIME_UNAVAILABLE")
        };
        if result
            .as_ref()
            .is_err_and(|code| crate::codex_session::requires_retirement(code))
        {
            *current = None;
        }
        self.publish(profile, current.as_ref().map(|(_, s)| s));
        result.map(Some)
    }

    pub(crate) fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::Release);
        self.stop();
    }

    pub(crate) fn stop(&self) {
        self.interrupt.store(true, Ordering::Release);
        *self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        *self
            .view
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    pub(crate) fn disconnect(&self, profile: Uuid) {
        if self
            .view
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .is_some_and(|(id, _)| *id != profile)
        {
            return;
        }
        self.interrupt.store(true, Ordering::Release);
        let mut current = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if current.as_ref().is_none_or(|(id, _)| *id != profile) {
            return;
        }
        // Dropping the process destroys memory-only credentials and login state.
        // Teardown must not dispatch further RPCs through an interrupted session.
        *current = None;
        self.publish(profile, None);
    }
}

#[cfg(test)]
#[path = "plan_runtime_tests.rs"]
mod tests;
