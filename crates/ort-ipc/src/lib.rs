//! Bounded browser capture contract and authentication primitives. Transport
//! remains disabled until per-platform vault identity and installation gates pass.

use std::collections::HashMap;
use std::io::{Read, Write};

use hmac::{Hmac, KeyInit, Mac};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_ENVELOPE_BYTES: usize = 256 * 1024;
pub const MAX_TEXT_BYTES: usize = 128 * 1024;
pub const MAX_URL_BYTES: usize = 4 * 1024;
pub const CAPTURE_TTL_MS: i64 = 60_000;
pub const ENABLED: bool = false;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BridgeError {
    #[error("the message is malformed")]
    Malformed,
    #[error("the message exceeds its size limit")]
    Oversized,
    #[error("the protocol version is incompatible")]
    Incompatible,
    #[error("the browser origin is not allowed")]
    WrongOrigin,
    #[error("the message is stale or from the future")]
    Expired,
    #[error("the message was already accepted")]
    Replay,
    #[error("the authentication tag is invalid")]
    Authentication,
    #[error("the bridge is unavailable")]
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureEnvelope {
    pub protocol_version: u16,
    pub request_id: Uuid,
    pub sent_at: String,
    pub kind: String,
    pub payload: CapturePayload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturePayload {
    pub text: String,
    pub url: String,
    pub title: String,
    pub browser: String,
    pub target: String,
}

/// Reads exactly one browser native-messaging frame with a pre-allocation cap.
/// # Errors
/// Rejects incomplete or oversized frames without allocating the stated size.
pub fn read_native_frame(input: &mut impl Read) -> Result<Vec<u8>, BridgeError> {
    let mut length = [0_u8; 4];
    input
        .read_exact(&mut length)
        .map_err(|_| BridgeError::Malformed)?;
    let length = usize::try_from(u32::from_le_bytes(length)).map_err(|_| BridgeError::Oversized)?;
    if length == 0 || length > MAX_ENVELOPE_BYTES {
        return Err(BridgeError::Oversized);
    }
    let mut bytes = vec![0_u8; length];
    input
        .read_exact(&mut bytes)
        .map_err(|_| BridgeError::Malformed)?;
    Ok(bytes)
}

/// Writes one response frame. Callers must provide a content-free response.
/// # Errors
/// Refuses oversized responses and failed stdout writes.
pub fn write_native_frame(output: &mut impl Write, bytes: &[u8]) -> Result<(), BridgeError> {
    let length = u32::try_from(bytes.len()).map_err(|_| BridgeError::Oversized)?;
    if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(BridgeError::Oversized);
    }
    output
        .write_all(&length.to_le_bytes())
        .map_err(|_| BridgeError::Unavailable)?;
    output
        .write_all(bytes)
        .map_err(|_| BridgeError::Unavailable)
}

/// Validates one extension message before any desktop launch or IPC activity.
/// # Errors
/// Rejects malformed, oversized, stale, unsupported, or unsafe content.
pub fn validate_capture(bytes: &[u8], now_ms: i64) -> Result<CaptureEnvelope, BridgeError> {
    if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(BridgeError::Oversized);
    }
    let envelope: CaptureEnvelope =
        serde_json::from_slice(bytes).map_err(|_| BridgeError::Malformed)?;
    if envelope.protocol_version != PROTOCOL_VERSION {
        return Err(BridgeError::Incompatible);
    }
    if envelope.kind != "capture.selection" {
        return Err(BridgeError::Malformed);
    }
    let sent = envelope
        .sent_at
        .parse::<Timestamp>()
        .map_err(|_| BridgeError::Malformed)?
        .as_millisecond();
    if sent > now_ms.saturating_add(5_000) || sent < now_ms.saturating_sub(CAPTURE_TTL_MS) {
        return Err(BridgeError::Expired);
    }
    let content = &envelope.payload;
    if content.text.trim().is_empty()
        || content.text.len() > MAX_TEXT_BYTES
        || content.url.len() > MAX_URL_BYTES
        || content.title.len() > 2_000
        || !matches!(content.browser.as_str(), "chrome" | "edge")
        || !matches!(content.target.as_str(), "job" | "question")
    {
        return Err(BridgeError::Malformed);
    }
    let url = url::Url::parse(&content.url).map_err(|_| BridgeError::Malformed)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(BridgeError::Malformed);
    }
    Ok(envelope)
}

/// Exact-origin check. Browser names or prefixes are insufficient authority.
/// # Errors
/// Rejects an origin absent from the compiled channel allowlist.
pub fn validate_origin(origin: &str, allowed: &[&str]) -> Result<(), BridgeError> {
    if allowed.iter().any(|item| *item == origin) {
        Ok(())
    } else {
        Err(BridgeError::WrongOrigin)
    }
}

type HmacSha256 = Hmac<Sha256>;

/// Signs a domain-separated, length-bounded handshake or capture transcript.
/// # Errors
/// Requires an installation-scoped 256-bit vault secret.
pub fn authentication_tag(secret: &[u8], transcript: &[u8]) -> Result<[u8; 32], BridgeError> {
    if secret.len() != 32 || transcript.len() > MAX_ENVELOPE_BYTES + 4096 {
        return Err(BridgeError::Authentication);
    }
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| BridgeError::Authentication)?;
    mac.update(b"ORT browser bridge v1\0");
    mac.update(transcript);
    Ok(mac.finalize().into_bytes().into())
}

/// Constant-time tag verification.
/// # Errors
/// Rejects missing, altered, or malformed tags.
pub fn verify_authentication(
    secret: &[u8],
    transcript: &[u8],
    tag: &[u8],
) -> Result<(), BridgeError> {
    if secret.len() != 32 || tag.len() != 32 || transcript.len() > MAX_ENVELOPE_BYTES + 4096 {
        return Err(BridgeError::Authentication);
    }
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| BridgeError::Authentication)?;
    mac.update(b"ORT browser bridge v1\0");
    mac.update(transcript);
    mac.verify_slice(tag)
        .map_err(|_| BridgeError::Authentication)
}

/// Memory-only replay window, keyed by browser request ID.
#[derive(Default)]
pub struct ReplayCache {
    accepted: HashMap<Uuid, i64>,
}

impl ReplayCache {
    /// # Errors
    /// Rejects duplicate or expired requests.
    pub fn accept(
        &mut self,
        request_id: Uuid,
        expiry_ms: i64,
        now_ms: i64,
    ) -> Result<(), BridgeError> {
        self.accepted.retain(|_, expiry| *expiry > now_ms);
        if expiry_ms <= now_ms || expiry_ms > now_ms.saturating_add(CAPTURE_TTL_MS) {
            return Err(BridgeError::Expired);
        }
        if self.accepted.contains_key(&request_id) {
            return Err(BridgeError::Replay);
        }
        if self.accepted.len() >= 4_096 {
            return Err(BridgeError::Unavailable);
        }
        self.accepted.insert(request_id, expiry_ms);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn message(now: i64) -> Vec<u8> {
        serde_json::to_vec(&json!({"protocolVersion":1,"requestId":Uuid::now_v7(),"sentAt":Timestamp::from_millisecond(now).unwrap().to_string(),"kind":"capture.selection","payload":{"text":"Synthetic selection","url":"https://example.test/job","title":"Synthetic","browser":"chrome","target":"job"}})).unwrap()
    }

    #[test]
    fn rejects_wrong_origin_replay_size_and_version() {
        let now = 1_800_000_000_000;
        let bytes = message(now);
        let parsed = validate_capture(&bytes, now).unwrap();
        assert_eq!(
            validate_origin("chrome-extension://wrong/", &["chrome-extension://right/"]),
            Err(BridgeError::WrongOrigin)
        );
        let mut cache = ReplayCache::default();
        cache.accept(parsed.request_id, now + 60_000, now).unwrap();
        assert_eq!(
            cache.accept(parsed.request_id, now + 60_000, now),
            Err(BridgeError::Replay)
        );
        assert_eq!(
            validate_capture(&bytes, now + 61_000).err(),
            Some(BridgeError::Expired)
        );
        let mut version: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        version["protocolVersion"] = json!(2);
        assert_eq!(
            validate_capture(&serde_json::to_vec(&version).unwrap(), now).err(),
            Some(BridgeError::Incompatible)
        );
        let mut input = std::io::Cursor::new(u32::MAX.to_le_bytes().to_vec());
        assert_eq!(read_native_frame(&mut input), Err(BridgeError::Oversized));
    }

    #[test]
    fn transcript_tag_detects_change() {
        let secret = [7_u8; 32];
        let tag = authentication_tag(&secret, b"nonce/request").unwrap();
        verify_authentication(&secret, b"nonce/request", &tag).unwrap();
        assert_eq!(
            verify_authentication(&secret, b"nonce/changed", &tag),
            Err(BridgeError::Authentication)
        );
    }
}
