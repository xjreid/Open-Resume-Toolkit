//! Bounded native-messaging entry point. Default builds require signed production
//! transport. The opt-in macOS dev feature uses a separate temporary capability.

use std::io;

use ort_ipc::{
    BridgeError, read_native_frame, validate_native_request, validate_origin, write_native_frame,
};
use serde_json::json;

fn main() {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let result = process(&mut input);
        let invalid_frame = matches!(
            &result,
            Err(BridgeError::Malformed | BridgeError::Oversized | BridgeError::WrongOrigin)
        );
        let response = result.unwrap_or_else(|problem| failure_response(&problem));
        let Ok(bytes) = serde_json::to_vec(&response) else {
            break;
        };
        if write_native_frame(&mut output, &bytes).is_err() {
            break;
        }
        if invalid_frame || !cfg!(all(feature = "dev-browser-bridge", target_os = "macos")) {
            break;
        }
    }
}

fn failure_response(problem: &BridgeError) -> serde_json::Value {
    let code = match problem {
        BridgeError::Incompatible => "PROTOCOL_INCOMPATIBLE",
        BridgeError::WrongOrigin | BridgeError::Authentication | BridgeError::Unavailable => {
            "BRIDGE_UNAVAILABLE"
        }
        BridgeError::Oversized => "CAPTURE_TOO_LARGE",
        BridgeError::Expired | BridgeError::Replay => "CAPTURE_EXPIRED",
        BridgeError::Malformed => "CAPTURE_INVALID",
    };
    json!({"ok":false,"error":{"code":code,"messageKey":"errors.browserBridge","retryable":false},"value":null,"desktopVersion":"0.0.0-dev","hostVersion":"0.0.0-dev","protocolVersion":ort_ipc::PROTOCOL_VERSION})
}

fn process(input: &mut impl io::Read) -> Result<serde_json::Value, BridgeError> {
    let origin = std::env::args().nth(1).ok_or(BridgeError::WrongOrigin)?;
    #[cfg(not(all(feature = "dev-browser-bridge", target_os = "macos")))]
    let ids = [
        option_env!("ORT_CHROME_EXTENSION_ID"),
        option_env!("ORT_DEV_CHROME_EXTENSION_ID"),
        option_env!("ORT_DEV_EDGE_EXTENSION_ID"),
    ];
    #[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
    let ids = [option_env!("ORT_DEV_CHROME_EXTENSION_ID")];
    let allowed: Vec<String> = ids
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
) -> Result<serde_json::Value, BridgeError> {
    validate_origin(origin, allowed)?;
    let bytes = read_native_frame(input)?;
    let now = jiff::Timestamp::now().as_millisecond();
    #[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
    {
        let _request = validate_native_request(&bytes, now)?;
        ort_ipc::development::forward(&ort_ipc::development::default_root(), origin, &bytes)
    }
    #[cfg(not(all(feature = "dev-browser-bridge", target_os = "macos")))]
    match validate_native_request(&bytes, now)? {
        ort_ipc::NativeRequest::Status => Ok(json!({
            "ok": false,
            "error": {"code": "SIGNED_BRIDGE_REQUIRED", "retryable": false},
            "value": {"ready": false},
            "desktopVersion": "0.0.0-dev",
            "hostVersion": "0.0.0-dev",
            "protocolVersion": ort_ipc::PROTOCOL_VERSION
        })),
        // Transport is deliberately absent until installation identity checks pass.
        ort_ipc::NativeRequest::Capture(_)
        | ort_ipc::NativeRequest::Poll(_)
        | ort_ipc::NativeRequest::Event(_) => Err(BridgeError::Unavailable),
    }
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
    #[cfg(not(all(feature = "dev-browser-bridge", target_os = "macos")))]
    fn unsigned_host_rejects_even_a_valid_capture_without_forwarding_it() {
        let mut frame = Cursor::new(framed_capture(ort_ipc::PROTOCOL_VERSION));
        assert_eq!(
            process_with_origin(&mut frame, ORIGIN, &[ORIGIN]),
            Err(BridgeError::Unavailable)
        );
        assert_eq!(
            usize::try_from(frame.position()).unwrap(),
            frame.get_ref().len()
        );
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

    #[test]
    #[cfg(not(all(feature = "dev-browser-bridge", target_os = "macos")))]
    fn reports_missing_signed_transport_without_claiming_readiness() {
        let bytes = br#"{"protocolVersion":1,"kind":"bridge.status"}"#;
        let mut frame = Vec::new();
        write_native_frame(&mut frame, bytes).unwrap();
        let status =
            process_with_origin(&mut Cursor::new(frame.clone()), ORIGIN, &[ORIGIN]).unwrap();
        assert_eq!(status["ok"], false);
        assert_eq!(status["value"]["ready"], false);
        assert_eq!(status["error"]["code"], "SIGNED_BRIDGE_REQUIRED");
        assert_eq!(
            process_with_origin(
                &mut Cursor::new(frame),
                "chrome-extension://wrong/",
                &[ORIGIN]
            ),
            Err(BridgeError::WrongOrigin)
        );
    }
}
