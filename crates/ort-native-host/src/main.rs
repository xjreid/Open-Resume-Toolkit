//! Bounded native-messaging entry point. It remains fail-closed until a signed
//! desktop/host pair can share an identity-scoped vault secret and IPC endpoint.

use std::io;

use ort_ipc::{
    BridgeError, read_native_frame, validate_capture, validate_origin, write_native_frame,
};
use serde_json::json;

fn main() {
    let result = process(&mut io::stdin().lock());
    let code = match result {
        Ok(()) => "BRIDGE_UNAVAILABLE",
        Err(BridgeError::Incompatible) => "PROTOCOL_INCOMPATIBLE",
        Err(BridgeError::WrongOrigin | BridgeError::Authentication) => "BRIDGE_UNAVAILABLE",
        Err(BridgeError::Oversized) => "CAPTURE_TOO_LARGE",
        Err(BridgeError::Expired | BridgeError::Replay) => "CAPTURE_EXPIRED",
        Err(_) => "CAPTURE_INVALID",
    };
    // No capture text, URL, local path, or system error is written to stdout.
    let response = json!({"ok":false,"error":{"code":code,"messageKey":"errors.browserBridge","retryable":false},"value":null,"desktopVersion":"0.0.0-dev","hostVersion":"0.0.0-dev","protocolVersion":ort_ipc::PROTOCOL_VERSION});
    if let Ok(bytes) = serde_json::to_vec(&response) {
        let _ = write_native_frame(&mut io::stdout().lock(), &bytes);
    }
}

fn process(input: &mut impl io::Read) -> Result<(), BridgeError> {
    let origin = std::env::args().nth(1).ok_or(BridgeError::WrongOrigin)?;
    let allowed: Vec<String> = [
        option_env!("ORT_DEV_CHROME_EXTENSION_ID"),
        option_env!("ORT_DEV_EDGE_EXTENSION_ID"),
    ]
    .into_iter()
    .flatten()
    .filter(|id| id.len() == 32 && id.bytes().all(|byte| (b'a'..=b'p').contains(&byte)))
    .map(|id| format!("chrome-extension://{id}/"))
    .collect();
    process_with_origin(
        input,
        &origin,
        &allowed.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

fn process_with_origin(
    input: &mut impl io::Read,
    origin: &str,
    allowed: &[&str],
) -> Result<(), BridgeError> {
    validate_origin(origin, allowed)?;
    let bytes = read_native_frame(input)?;
    let now = jiff::Timestamp::now().as_millisecond();
    let _capture = validate_capture(&bytes, now)?;
    // The transport is deliberately absent until installation identity checks pass.
    Err(BridgeError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const ORIGIN: &str = "chrome-extension://abcdefghijklmnopabcdefghijklmnop/";

    fn framed_capture(version: u16) -> Vec<u8> {
        let payload = json!({
            "protocolVersion": version,
            "requestId": "018bd20e-8d6e-7a0b-8000-000000000001",
            "sentAt": jiff::Timestamp::now().to_string(),
            "kind": "capture.selection",
            "payload": {
                "text": "Synthetic selected job text",
                "url": "https://example.test/job",
                "title": "Synthetic job",
                "browser": "chrome",
                "target": "job"
            }
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
        let mut frame = Vec::with_capacity(4 + bytes.len());
        frame.extend_from_slice(&u32::try_from(bytes.len()).unwrap().to_le_bytes());
        frame.extend_from_slice(&bytes);
        frame
    }

    #[test]
    fn unsigned_host_rejects_even_a_valid_capture_without_forwarding_it() {
        let mut frame = Cursor::new(framed_capture(ort_ipc::PROTOCOL_VERSION));
        assert_eq!(
            process_with_origin(&mut frame, ORIGIN, &[ORIGIN]),
            Err(BridgeError::Unavailable)
        );
        assert_eq!(frame.position() as usize, frame.get_ref().len());
    }

    #[test]
    fn rejects_wrong_origin_before_reading_capture() {
        let mut frame = Cursor::new(framed_capture(ort_ipc::PROTOCOL_VERSION));
        assert_eq!(
            process_with_origin(&mut frame, "chrome-extension://wrong/", &[ORIGIN]),
            Err(BridgeError::WrongOrigin)
        );
        assert_eq!(frame.position(), 0);
    }

    #[test]
    fn rejects_version_mismatch_and_oversized_frame() {
        let mut incompatible = Cursor::new(framed_capture(ort_ipc::PROTOCOL_VERSION + 1));
        assert_eq!(
            process_with_origin(&mut incompatible, ORIGIN, &[ORIGIN]),
            Err(BridgeError::Incompatible)
        );
        let mut oversized = Cursor::new(u32::MAX.to_le_bytes().to_vec());
        assert_eq!(
            process_with_origin(&mut oversized, ORIGIN, &[ORIGIN]),
            Err(BridgeError::Oversized)
        );
    }
}
