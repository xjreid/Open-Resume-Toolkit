//! Development-channel bridge; production IPC remains disabled.
use ort_domain::CommandResponse;
use serde::Serialize;
use tauri::{Manager, WebviewWindow};

#[derive(Default)]
pub(crate) struct BrowserBridgeState {
    #[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
    server: std::sync::Mutex<Option<ort_ipc::development::Server>>,
    captures: std::sync::Arc<std::sync::Mutex<ort_ipc::capture_session::CaptureSession>>,
}
impl BrowserBridgeState {
    pub(crate) fn enabled(&self) -> bool {
        #[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
        {
            self.server.lock().is_ok_and(|server| {
                server
                    .as_ref()
                    .is_some_and(ort_ipc::development::Server::is_running)
            })
        }
        #[cfg(not(all(feature = "dev-browser-bridge", target_os = "macos")))]
        {
            false
        }
    }
    pub(crate) fn connected(&self) -> bool {
        self.enabled()
            && self
                .captures
                .lock()
                .is_ok_and(|captures| captures.connected(jiff::Timestamp::now().as_millisecond()))
    }
    pub(crate) fn start(
        &self,
        target: &str,
    ) -> Result<ort_ipc::capture_session::CaptureStatus, &'static str> {
        if !self.enabled() {
            return Err("BROWSER_CAPTURE_UNAVAILABLE");
        }
        self.captures
            .lock()
            .map_err(|_| "BRIDGE_UNAVAILABLE")?
            .start(target, jiff::Timestamp::now().as_millisecond())
    }
    pub(crate) fn disconnect(&self) {
        if let Ok(mut captures) = self.captures.lock() {
            captures.disconnect(jiff::Timestamp::now().as_millisecond());
        }
        #[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
        {
            // Join outside the bridge mutex: intake may hold the storage mutex
            // while the overlay reads connection status.
            let previous = self.server.lock().ok().and_then(|mut server| server.take());
            drop(previous);
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    available: bool,
    connected: bool,
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn browser_connection_status(window: WebviewWindow) -> CommandResponse<ConnectionStatus> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    CommandResponse::success(ConnectionStatus {
        available: cfg!(all(feature = "dev-browser-bridge", target_os = "macos"))
            && crate::development_identity_allowed(&window.app_handle().config().identifier),
        connected: window.state::<BrowserBridgeState>().enabled(),
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn disconnect_development_browser(window: WebviewWindow) -> CommandResponse<bool> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    window.state::<BrowserBridgeState>().disconnect();
    CommandResponse::success(true)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn connect_development_browser(window: WebviewWindow) -> CommandResponse<bool> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    if !crate::development_identity_allowed(&window.app_handle().config().identifier) {
        return CommandResponse::failure(
            "DEV_BRIDGE_IDENTITY_INVALID",
            "errors.browserBridge",
            false,
        );
    }
    #[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
    return connect(window.app_handle());
    #[cfg(not(all(feature = "dev-browser-bridge", target_os = "macos")))]
    CommandResponse::failure("DEV_BRIDGE_NOT_BUILT", "errors.browserBridge", false)
}

/// Enable the registered development bridge on each launch. Missing or invalid
/// setup leaves it disabled; starting the listener does not imply an extension is connected.
pub(crate) fn enable_on_launch(app: &tauri::AppHandle) {
    #[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
    if crate::development_identity_allowed(&app.config().identifier) {
        let _ = connect(app);
    }
    #[cfg(not(all(feature = "dev-browser-bridge", target_os = "macos")))]
    let _ = app;
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn application_capture_status(
    window: WebviewWindow,
) -> CommandResponse<ort_ipc::capture_session::CaptureStatus> {
    if window.label() != "overlay" {
        return crate::window_not_authorized();
    }
    match window.state::<BrowserBridgeState>().captures.lock() {
        Ok(mut captures) => {
            CommandResponse::success(captures.status(jiff::Timestamp::now().as_millisecond()))
        }
        Err(_) => CommandResponse::failure("BRIDGE_UNAVAILABLE", "errors.browserBridge", false),
    }
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cancel_application_capture(
    window: WebviewWindow,
    session_id: uuid::Uuid,
) -> CommandResponse<ort_ipc::capture_session::CaptureStatus> {
    if window.label() != "overlay" {
        return crate::window_not_authorized();
    }
    match window.state::<BrowserBridgeState>().captures.lock() {
        Ok(mut captures) => CommandResponse::success(
            captures.cancel(session_id, jiff::Timestamp::now().as_millisecond()),
        ),
        Err(_) => CommandResponse::failure("BRIDGE_UNAVAILABLE", "errors.browserBridge", false),
    }
}

#[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
pub(crate) fn receive_development_request(
    state: &std::sync::Mutex<ort_ipc::capture_session::CaptureSession>,
    request: ort_ipc::NativeRequest,
    frame: &[u8],
    now: i64,
    storage_ready: bool,
    intake: impl FnOnce(&[u8]) -> Result<uuid::Uuid, ort_storage::StorageError>,
) -> serde_json::Value {
    let Ok(mut captures) = state.lock() else {
        return ort_ipc::development::response("BRIDGE_UNAVAILABLE", false);
    };
    match request {
        ort_ipc::NativeRequest::Status => {
            ort_ipc::development::response("STORAGE_UNAVAILABLE", storage_ready)
        }
        ort_ipc::NativeRequest::Poll(request) => captures.poll(&request, now, storage_ready),
        ort_ipc::NativeRequest::Event(request) => match captures.event(&request, now) {
            Ok(()) => serde_json::json!({"ok":true,"protocolVersion":1,"value":null}),
            Err(code) => ort_ipc::development::response(code, false),
        },
        ort_ipc::NativeRequest::Capture(capture) => {
            if let Err(code) = captures.authorize(capture.request_id, &capture.payload.target, now)
            {
                return ort_ipc::development::response(code, false);
            }
            match intake(frame) {
                Ok(request_id) => {
                    captures.delivered();
                    serde_json::json!({"ok":true,"protocolVersion":1,"value":{"requestId":request_id}})
                }
                Err(problem) => {
                    let code = if matches!(problem, ort_storage::StorageError::RevisionConflict) {
                        "CAPTURE_PENDING"
                    } else {
                        "BRIDGE_UNAVAILABLE"
                    };
                    captures.fail(code, now);
                    ort_ipc::development::response(code, false)
                }
            }
        }
    }
}

#[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
fn connect(handle: &tauri::AppHandle) -> CommandResponse<bool> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use tauri::{Emitter, EventTarget};
    let app = handle.clone();
    let state = handle.state::<BrowserBridgeState>();
    let Ok(mut slot) = state.server.lock() else {
        return failed();
    };
    if slot
        .as_ref()
        .is_some_and(ort_ipc::development::Server::is_running)
    {
        return CommandResponse::success(true);
    }
    *slot = None;
    let Ok(data) = app.path().app_data_dir() else {
        return failed();
    };
    let registration = data.join("browser-bridge-dev").join("registration.json");
    let Ok(metadata) = std::fs::symlink_metadata(&registration) else {
        return failed();
    };
    if !metadata.is_file()
        || metadata.len() > 1024
        || metadata.permissions().mode() & 0o777 != 0o600
        || metadata.uid() != std::fs::metadata(&data).map_or(u32::MAX, |value| value.uid())
    {
        return failed();
    }
    let Ok(bytes) = std::fs::read(registration) else {
        return failed();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return failed();
    };
    let Some(id) = value.get("extensionId").and_then(serde_json::Value::as_str) else {
        return failed();
    };
    let captures = state.captures.clone();
    let server = ort_ipc::development::Server::start(
        &ort_ipc::development::default_root(),
        id,
        move |request, frame| {
            let capture = matches!(request, ort_ipc::NativeRequest::Capture(_));
            let wake = capture || matches!(request, ort_ipc::NativeRequest::Event(_));
            let storage = app.state::<crate::DesktopState>();
            let now = jiff::Timestamp::now().as_millisecond();
            let response = receive_development_request(
                &captures,
                request,
                frame,
                now,
                storage.storage_status() == ort_domain::StorageStatus::Ready,
                |frame| {
                    storage.with_store(|store| {
                        crate::application_materials::accept_authenticated_capture(
                            store, frame, now,
                        )
                    })
                },
            );
            if wake {
                let notify = app.clone();
                let focus = capture
                    && response.get("ok").and_then(serde_json::Value::as_bool) == Some(true);
                let _ = app.run_on_main_thread(move || {
                    if let Some(overlay) = notify.get_webview_window("overlay") {
                        if focus {
                            let _ = overlay.show();
                            let _ = overlay.unminimize();
                            let _ = overlay.set_focus();
                        }
                        let _ = notify.emit_to(
                            EventTarget::webview_window("overlay"),
                            if focus {
                                "ort:browser-capture"
                            } else {
                                "ort:capture-mode"
                            },
                            (),
                        );
                    }
                });
            }
            response
        },
    );
    match server {
        Ok(server) => {
            *slot = Some(server);
            CommandResponse::success(true)
        }
        Err(_) => failed(),
    }
}
#[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
fn failed<T: Serialize>() -> CommandResponse<T> {
    CommandResponse::failure("DEV_BRIDGE_SETUP_REQUIRED", "errors.browserBridge", false)
}

#[cfg(all(test, feature = "dev-browser-bridge", target_os = "macos"))]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, mpsc},
        time::Duration,
    };

    #[test]
    fn disconnect_releases_state_lock_before_joining_intake() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let (started, receiving) = mpsc::sync_channel(1);
        let (release, waiting) = mpsc::sync_channel(1);
        let server = ort_ipc::development::Server::start(
            directory.path(),
            "abcdefghijklmnopabcdefghijklmnop",
            move |_, _| {
                started.send(()).unwrap();
                waiting.recv().unwrap();
                ort_ipc::development::response("", true)
            },
        )
        .unwrap();
        let state = Arc::new(BrowserBridgeState {
            server: std::sync::Mutex::new(Some(server)),
            captures: Arc::default(),
        });
        let path = directory.path().to_owned();
        let sender = std::thread::spawn(move || {
            ort_ipc::development::forward(
                &path,
                "chrome-extension://abcdefghijklmnopabcdefghijklmnop/",
                br#"{"protocolVersion":1,"kind":"bridge.status"}"#,
            )
        });
        receiving.recv_timeout(Duration::from_secs(1)).unwrap();
        let disconnecting = state.clone();
        let disconnect = std::thread::spawn(move || disconnecting.disconnect());
        let (checked, result) = mpsc::sync_channel(1);
        let checking = std::thread::spawn(move || {
            while state.enabled() {
                std::thread::sleep(Duration::from_millis(5));
            }
            checked.send(()).unwrap();
        });
        let responsive = result.recv_timeout(Duration::from_millis(500)).is_ok();
        release.send(()).unwrap(); // Always release before asserting to avoid a hung test.
        disconnect.join().unwrap();
        checking.join().unwrap();
        assert!(sender.join().unwrap().is_ok());
        assert!(
            responsive,
            "connection status must remain accessible during disconnect"
        );
    }
}
