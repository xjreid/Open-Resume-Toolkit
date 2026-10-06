//! In-memory desktop authority for overlay-initiated capture. No page content is
//! kept here. Cancelling revokes intake before Chrome receives the cancellation.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

pub const CAPTURE_MODE_TTL_MS: i64 = 120_000;
const HEARTBEAT_TTL_MS: i64 = 3_000;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PollRequest {
    pub protocol_version: u16,
    pub kind: String,
    pub client_id: Uuid,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventRequest {
    pub protocol_version: u16,
    pub kind: String,
    pub client_id: Uuid,
    pub session_id: Uuid,
    pub phase: EventPhase,
    pub code: Option<FailureCode>,
}
#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventPhase {
    Started,
    Selecting,
    Cancelled,
    Failed,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureCode {
    PageUnavailable,
    PageChanged,
    EmptySelection,
    CaptureTooLarge,
    CaptureExpired,
    CaptureInvalid,
    BridgeUnavailable,
    DeliveryUnconfirmed,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub phase: &'static str,
    pub session_id: Option<Uuid>,
    pub error: Option<&'static str>,
}
impl Default for CaptureStatus {
    fn default() -> Self {
        Self {
            phase: "idle",
            session_id: None,
            error: None,
        }
    }
}
struct Active {
    id: Uuid,
    target: String,
    expires_at: i64,
}
#[derive(Default)]
pub struct CaptureSession {
    active: Option<Active>,
    status: CaptureStatus,
    client: Option<(Uuid, i64)>,
    cancelled: Option<(Uuid, i64)>,
}
impl CaptureSession {
    fn expire(&mut self, now: i64) {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.expires_at <= now)
        {
            self.end(Some("CAPTURE_EXPIRED"), now);
        } else if self.active.is_some() && !self.connected(now) {
            self.end(Some("BROWSER_CAPTURE_UNAVAILABLE"), now);
        }
        if self.cancelled.is_some_and(|(_, expiry)| expiry <= now) {
            self.cancelled = None;
        }
    }
    #[must_use]
    pub fn connected(&self, now: i64) -> bool {
        self.client
            .is_some_and(|(_, last)| now.saturating_sub(last) < HEARTBEAT_TTL_MS)
    }
    pub fn status(&mut self, now: i64) -> CaptureStatus {
        self.expire(now);
        self.status.clone()
    }
    /// # Errors
    /// Requires a live browser client, a supported target and no active mode.
    pub fn start(&mut self, target: &str, now: i64) -> Result<CaptureStatus, &'static str> {
        self.expire(now);
        if !matches!(target, "job" | "question") {
            return Err("CAPTURE_TARGET_INVALID");
        }
        if !self.connected(now) {
            return Err("BROWSER_CAPTURE_UNAVAILABLE");
        }
        if self.active.is_some() {
            return Err("CAPTURE_BUSY");
        }
        let id = Uuid::now_v7();
        self.active = Some(Active {
            id,
            target: target.into(),
            expires_at: now.saturating_add(CAPTURE_MODE_TTL_MS),
        });
        self.status = CaptureStatus {
            phase: "waiting",
            session_id: Some(id),
            error: None,
        };
        Ok(self.status.clone())
    }
    pub fn cancel(&mut self, session_id: Uuid, now: i64) -> CaptureStatus {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.id == session_id)
        {
            self.end(None, now);
        }
        self.status(now)
    }
    fn end(&mut self, error: Option<&'static str>, now: i64) {
        if let Some(active) = self.active.take() {
            self.cancelled = Some((active.id, now.saturating_add(10_000)));
        }
        self.status = CaptureStatus {
            phase: "idle",
            session_id: None,
            error,
        };
    }
    pub fn disconnect(&mut self, now: i64) {
        self.end(None, now);
        self.client = None;
    }
    pub fn poll(&mut self, request: &PollRequest, now: i64, storage_ready: bool) -> Value {
        self.expire(now);
        // A second Chrome profile cannot steal the currently live browser session.
        if self.client.is_some_and(|(id, _)| id != request.client_id) && self.connected(now) {
            return json!({"ok":false,"protocolVersion":1,"error":{"code":"BROWSER_BUSY"}});
        }
        self.client = Some((request.client_id, now));
        if !storage_ready {
            self.end(Some("STORAGE_UNAVAILABLE"), now);
        }
        let mut commands = Vec::new();
        if let Some((id, _)) = self.cancelled {
            commands.push(json!({"kind":"capture.cancel","sessionId":id}));
        }
        if let Some(active) = &self.active {
            commands.push(json!({"kind":"capture.start","sessionId":active.id,"target":active.target,"expiresAt":active.expires_at}));
        }
        json!({"ok":true,"protocolVersion":1,"value":{"ready":storage_ready,"commands":commands}})
    }
    /// # Errors
    /// Rejects events from another browser or stale capture generation.
    pub fn event(&mut self, event: &EventRequest, now: i64) -> Result<(), &'static str> {
        self.expire(now);
        if self.client.is_none_or(|(id, _)| id != event.client_id) {
            return Err("CAPTURE_INVALID");
        }
        if matches!(event.phase, EventPhase::Cancelled)
            && self.cancelled.is_some_and(|(id, _)| id == event.session_id)
        {
            self.cancelled = None;
            return Ok(());
        }
        if self
            .active
            .as_ref()
            .is_none_or(|active| active.id != event.session_id)
        {
            return Err("CAPTURE_EXPIRED");
        }
        match event.phase {
            EventPhase::Started => {}
            EventPhase::Selecting => self.status.phase = "selecting",
            EventPhase::Cancelled => self.end(None, now),
            EventPhase::Failed => self.end(
                Some(match event.code.unwrap_or(FailureCode::CaptureInvalid) {
                    FailureCode::PageUnavailable => "PAGE_UNAVAILABLE",
                    FailureCode::PageChanged => "PAGE_CHANGED",
                    FailureCode::EmptySelection => "EMPTY_SELECTION",
                    FailureCode::CaptureTooLarge => "CAPTURE_TOO_LARGE",
                    FailureCode::CaptureExpired => "CAPTURE_EXPIRED",
                    FailureCode::CaptureInvalid => "CAPTURE_INVALID",
                    FailureCode::BridgeUnavailable => "BRIDGE_UNAVAILABLE",
                    FailureCode::DeliveryUnconfirmed => "DELIVERY_UNCONFIRMED",
                }),
                now,
            ),
        }
        Ok(())
    }
    /// # Errors
    /// Intake must match the active overlay generation and its intended field.
    pub fn authorize(&mut self, id: Uuid, target: &str, now: i64) -> Result<(), &'static str> {
        self.expire(now);
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.id == id && active.target == target)
            && self.status.phase == "selecting"
        {
            Ok(())
        } else {
            Err("CAPTURE_EXPIRED")
        }
    }
    pub fn fail(&mut self, code: &'static str, now: i64) {
        self.end(Some(code), now);
    }
    pub fn delivered(&mut self) {
        self.active = None;
        self.status = CaptureStatus::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn poll(id: Uuid) -> PollRequest {
        PollRequest {
            protocol_version: 1,
            kind: "bridge.poll".into(),
            client_id: id,
        }
    }
    #[test]
    fn cancellation_revokes_late_delivery_and_cannot_cancel_a_new_capture() {
        let client = Uuid::now_v7();
        let mut state = CaptureSession::default();
        state.poll(&poll(client), 0, true);
        let old = state.start("job", 1).unwrap().session_id.unwrap();
        state.cancel(old, 2);
        assert!(state.authorize(old, "job", 2).is_err());
        let current = state.start("question", 3).unwrap().session_id.unwrap();
        state.cancel(old, 4);
        assert_eq!(state.status(4).session_id, Some(current));
        assert_eq!(
            state.poll(&poll(client), 5, true)["value"]["commands"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        state
            .event(
                &EventRequest {
                    protocol_version: 1,
                    kind: "capture.event".into(),
                    client_id: client,
                    session_id: current,
                    phase: EventPhase::Selecting,
                    code: None,
                },
                6,
            )
            .unwrap();
        assert!(state.authorize(current, "job", 7).is_err());
        state.authorize(current, "question", 7).unwrap();
        state.delivered();
        assert!(state.authorize(current, "question", 8).is_err());
    }
    #[test]
    fn live_client_is_pinned_and_lost_browser_or_timeout_ends_capture() {
        let mut state = CaptureSession::default();
        let client = Uuid::now_v7();
        assert!(state.start("job", 0).is_err());
        state.poll(&poll(client), 0, true);
        state.start("job", 1).unwrap();
        assert_eq!(state.poll(&poll(Uuid::now_v7()), 2, true)["ok"], false);
        assert_eq!(
            state.status(3_001).error,
            Some("BROWSER_CAPTURE_UNAVAILABLE")
        );
        state.poll(&poll(client), 4_000, true);
        state.start("job", 4_001).unwrap();
        assert_eq!(
            state.status(4_001 + CAPTURE_MODE_TTL_MS).error,
            Some("CAPTURE_EXPIRED")
        );
        state.poll(&poll(client), 200_000, false);
        assert_eq!(state.status(200_001).error, Some("STORAGE_UNAVAILABLE"));
    }
}
