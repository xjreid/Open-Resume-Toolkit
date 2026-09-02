use std::sync::Mutex;

use ort_domain::{CloseDecision, CloseStatusResponse, EntityId};

#[derive(Default)]
struct CloseState {
    pending: Option<String>,
    approved: bool,
}

/// Native-owned, single-use close attempts. No dirty-state cache can race edits.
#[derive(Default)]
pub(crate) struct CloseGuard(Mutex<CloseState>);

impl CloseGuard {
    pub(crate) fn request(&self) -> Result<(), &'static str> {
        let mut state = self.0.lock().map_err(|_| "CLOSE_UNAVAILABLE")?;
        if !state.approved && state.pending.is_none() {
            state.pending = Some(EntityId::new().as_uuid().to_string());
        }
        Ok(())
    }

    pub(crate) fn status(&self, window: &str) -> Result<CloseStatusResponse, &'static str> {
        authorize_main(window)?;
        let state = self.0.lock().map_err(|_| "CLOSE_UNAVAILABLE")?;
        Ok(CloseStatusResponse {
            pending_attempt: state.pending.clone(),
        })
    }

    pub(crate) fn resolve(
        &self,
        window: &str,
        attempt: &str,
        decision: CloseDecision,
    ) -> Result<(), &'static str> {
        authorize_main(window)?;
        let mut state = self.0.lock().map_err(|_| "CLOSE_UNAVAILABLE")?;
        if state.pending.as_deref() != Some(attempt) || state.approved {
            return Err("STALE_CLOSE_ATTEMPT");
        }
        state.pending = None;
        state.approved = decision == CloseDecision::Quit;
        Ok(())
    }

    pub(crate) fn approved(&self) -> bool {
        self.0.lock().is_ok_and(|state| state.approved)
    }
}

fn authorize_main(window: &str) -> Result<(), &'static str> {
    if window == "main" {
        Ok(())
    } else {
        Err("WINDOW_NOT_AUTHORIZED")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(guard: &CloseGuard) -> String {
        guard
            .status("main")
            .expect("status")
            .pending_attempt
            .expect("pending")
    }

    #[test]
    fn unsolicited_or_overlay_responses_cannot_authorize_quit() {
        let guard = CloseGuard::default();
        assert!(!guard.approved());
        assert!(
            guard
                .resolve("main", "invented", CloseDecision::Quit)
                .is_err()
        );
        guard.request().expect("request");
        let id = attempt(&guard);
        assert!(guard.status("overlay").is_err());
        assert!(guard.resolve("overlay", &id, CloseDecision::Quit).is_err());
        assert!(!guard.approved());
    }

    #[test]
    fn duplicate_close_requests_coalesce_and_cancel_invalidates_the_attempt() {
        let guard = CloseGuard::default();
        guard.request().expect("first");
        let first = attempt(&guard);
        guard.request().expect("repeat");
        assert_eq!(attempt(&guard), first);
        guard
            .resolve("main", &first, CloseDecision::Cancel)
            .expect("cancel");
        assert!(!guard.approved());
        guard.request().expect("new attempt");
        let second = attempt(&guard);
        assert_ne!(first, second);
        assert!(guard.resolve("main", &first, CloseDecision::Quit).is_err());
        guard
            .resolve("main", &second, CloseDecision::Quit)
            .expect("approve");
        assert!(guard.approved());
        assert!(guard.resolve("main", &second, CloseDecision::Quit).is_err());
    }
}
