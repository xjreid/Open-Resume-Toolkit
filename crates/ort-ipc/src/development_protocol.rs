//! Internal authenticated development protocol shared by the native transports.
use crate::{
    BridgeError, CAPTURE_TTL_MS, MAX_ENVELOPE_BYTES, authentication_tag, verify_authentication,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroize;
pub(super) const HEADER: usize = 72;
const TAG: usize = 32;
pub(super) const MAX_PACKET: usize = MAX_ENVELOPE_BYTES + HEADER + TAG;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Session {
    pub(super) id: Uuid,
    pub(super) extension_id: String,
    pub(super) secret: [u8; 32],
}
impl Drop for Session {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

pub(super) fn packet(
    value: &Session,
    nonce: Uuid,
    expiry: i64,
    body: &[u8],
    direction: &[u8],
) -> Result<Vec<u8>, BridgeError> {
    if body.len() > MAX_ENVELOPE_BYTES {
        return Err(BridgeError::Oversized);
    }
    let mut bytes = Vec::with_capacity(HEADER + body.len() + TAG);
    bytes.extend_from_slice(value.id.as_bytes());
    bytes.extend_from_slice(nonce.as_bytes());
    bytes.extend_from_slice(&expiry.to_le_bytes());
    bytes.extend_from_slice(value.extension_id.as_bytes());
    bytes.extend_from_slice(body);
    let mut transcript = direction.to_vec();
    transcript.extend_from_slice(&bytes);
    bytes.extend_from_slice(&authentication_tag(&value.secret, &transcript)?);
    Ok(bytes)
}
pub(super) fn authenticate<'a>(
    value: &Session,
    bytes: &'a [u8],
    direction: &[u8],
    now_ms: i64,
) -> Result<(Uuid, i64, &'a [u8]), BridgeError> {
    if bytes.len() < HEADER + TAG || bytes.len() > MAX_PACKET {
        return Err(BridgeError::Oversized);
    }
    let tag_start = bytes.len() - TAG;
    let mut transcript = direction.to_vec();
    transcript.extend_from_slice(&bytes[..tag_start]);
    verify_authentication(&value.secret, &transcript, &bytes[tag_start..])?;
    if bytes[..16] != *value.id.as_bytes() || bytes[40..HEADER] != *value.extension_id.as_bytes() {
        return Err(BridgeError::Authentication);
    }
    let nonce = Uuid::from_slice(&bytes[16..32]).map_err(|_| BridgeError::Malformed)?;
    let expiry = i64::from_le_bytes(
        bytes[32..40]
            .try_into()
            .map_err(|_| BridgeError::Malformed)?,
    );
    if expiry <= now_ms || expiry > now_ms.saturating_add(CAPTURE_TTL_MS) {
        return Err(BridgeError::Expired);
    }
    Ok((nonce, expiry, &bytes[HEADER..tag_start]))
}
pub(super) fn packet_size(length: [u8; 4]) -> Result<usize, BridgeError> {
    let size = usize::try_from(u32::from_le_bytes(length)).map_err(|_| BridgeError::Oversized)?;
    if !(HEADER + TAG..=MAX_PACKET).contains(&size) {
        return Err(BridgeError::Oversized);
    }
    Ok(size)
}
