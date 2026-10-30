//! Explicit Windows development transport. Its temporary capability belongs to
//! the current account; production installation identity remains a separate gate.
#[path = "development_windows_registration.rs"]
mod registration;
#[cfg(test)]
use crate::development_protocol::HEADER;
use crate::development_protocol::{MAX_PACKET, Session, authenticate, packet, packet_size};
use crate::{BridgeError, NativeRequest, ReplayCache, validate_native_request};
use ort_windows::{
    PipeConnection, PipeListener, PipeOperation, PrivateDirectory, reject_reparse_path, secure_file,
};
pub use registration::registered_extension_id;
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use uuid::Uuid;
use zeroize::Zeroize;
const CAPABILITY: &str = "session.json";
const SHARE_READ: u32 = 1;

#[must_use]
pub fn default_root() -> PathBuf {
    std::env::var_os("ORT_DEV_BRIDGE_DIRECTORY").map_or_else(
        || {
            std::env::var_os("LOCALAPPDATA").map_or_else(PathBuf::new, |path| {
                PathBuf::from(path).join("com.openresumetoolkit.dev-bridge")
            })
        },
        PathBuf::from,
    )
}
fn now() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}
/// Separate per-user development-host installation; never a production name.
#[must_use]
pub fn registration_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map_or_else(PathBuf::new, |path| {
        PathBuf::from(path).join("com.openresumetoolkit.dev-bridge-host")
    })
}
fn unavailable<T>(_: T) -> BridgeError {
    BridgeError::Unavailable
}
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|c| (b'a'..=b'p').contains(&c))
}
fn connect(identifier: Uuid, end: Instant) -> Result<PipeConnection, BridgeError> {
    loop {
        match PipeConnection::connect(&identifier.to_string()) {
            Ok(pipe) => return Ok(pipe),
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < end =>
            {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => return Err(unavailable(error)),
        }
    }
}

fn session(root: &Path) -> Result<Session, BridgeError> {
    let _directory = PrivateDirectory::open(root).map_err(unavailable)?;
    let path = root.join(CAPABILITY);
    let _guards = reject_reparse_path(&path).map_err(unavailable)?;
    secure_file(&path).map_err(unavailable)?;
    let file = OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ)
        .open(path)
        .map_err(unavailable)?;
    if file.metadata().map_err(unavailable)?.len() > 2048 {
        return Err(BridgeError::Authentication);
    }
    let mut bytes = Vec::new();
    file.take(2049)
        .read_to_end(&mut bytes)
        .map_err(unavailable)?;
    let parsed = serde_json::from_slice::<Session>(&bytes).map_err(|_| BridgeError::Authentication);
    bytes.zeroize();
    let value = parsed?;
    if !valid_id(&value.extension_id) {
        return Err(BridgeError::Authentication);
    }
    Ok(value)
}

fn write_capability(root: &Path, value: &Session) -> Result<(), BridgeError> {
    let temporary = root.join(format!("session-{}.tmp", value.id));
    let mut bytes = serde_json::to_vec(value).map_err(unavailable)?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(unavailable)?;
        secure_file(&temporary).map_err(unavailable)?;
        file.write_all(&bytes).map_err(unavailable)?;
        file.sync_all().map_err(unavailable)?;
        drop(file);
        let destination = root.join(CAPABILITY);
        if destination.try_exists().map_err(unavailable)? {
            secure_file(&destination).map_err(unavailable)?;
        }
        fs::rename(&temporary, destination).map_err(unavailable)
    })();
    bytes.zeroize();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn exclusive_session(root: &Path) -> Result<File, BridgeError> {
    let path = root.join("session.lock");
    let _guards = reject_reparse_path(&path).map_err(unavailable)?;
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => drop(file),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(unavailable(error)),
    }
    secure_file(&path).map_err(unavailable)?;
    // A process crash closes this lock; its non-secret file need not be removed.
    OpenOptions::new()
        .read(true)
        .write(true)
        // Permit read-only path guards; another session's write access is denied.
        .share_mode(SHARE_READ)
        .open(path)
        .map_err(unavailable)
}

fn wait(
    mut operation: PipeOperation,
    end: Instant,
    stop: &AtomicBool,
) -> Result<Vec<u8>, BridgeError> {
    let result = (|| loop {
        if stop.load(Ordering::Acquire) || Instant::now() >= end {
            return Err(BridgeError::Unavailable);
        }
        if operation.poll().map_err(unavailable)?.is_some() {
            return operation.bytes().map(<[u8]>::to_vec).map_err(unavailable);
        }
        thread::sleep(Duration::from_millis(2));
    })();
    operation.cancel().map_err(unavailable)?;
    result
}
fn read_exact(
    pipe: &PipeConnection,
    bytes: &mut [u8],
    end: Instant,
    stop: &AtomicBool,
) -> Result<(), BridgeError> {
    let mut offset = 0;
    while offset < bytes.len() {
        let result = wait(
            pipe.read_up_to((bytes.len() - offset).min(8192))
                .map_err(unavailable)?,
            end,
            stop,
        )?;
        if result.is_empty() {
            return Err(BridgeError::Unavailable);
        }
        bytes[offset..offset + result.len()].copy_from_slice(&result);
        offset += result.len();
    }
    Ok(())
}
fn read_packet(
    pipe: &PipeConnection,
    end: Instant,
    stop: &AtomicBool,
) -> Result<Vec<u8>, BridgeError> {
    let mut length = [0; 4];
    read_exact(pipe, &mut length, end, stop)?;
    let mut bytes = vec![0; packet_size(length)?];
    read_exact(pipe, &mut bytes, end, stop)?;
    Ok(bytes)
}
fn write_packet(
    pipe: &PipeConnection,
    bytes: &[u8],
    end: Instant,
    stop: &AtomicBool,
) -> Result<(), BridgeError> {
    if bytes.len() > MAX_PACKET {
        return Err(BridgeError::Oversized);
    }
    let mut frame = Vec::with_capacity(4 + bytes.len());
    frame.extend_from_slice(
        &u32::try_from(bytes.len())
            .map_err(unavailable)?
            .to_le_bytes(),
    );
    frame.extend_from_slice(bytes);
    write_bytes(pipe, &frame, end, stop)
}
fn write_bytes(
    pipe: &PipeConnection,
    frame: &[u8],
    end: Instant,
    stop: &AtomicBool,
) -> Result<(), BridgeError> {
    let mut offset = 0;
    while offset < frame.len() {
        let mut operation = pipe
            .write(&frame[offset..frame.len().min(offset + 8192)])
            .map_err(unavailable)?;
        let result = (|| loop {
            if stop.load(Ordering::Acquire) || Instant::now() >= end {
                return Err(BridgeError::Unavailable);
            }
            if let Some(count) = operation.poll().map_err(unavailable)? {
                return Ok(count);
            }
            thread::sleep(Duration::from_millis(2));
        })();
        operation.cancel().map_err(unavailable)?;
        let count = result?;
        if count == 0 {
            return Err(BridgeError::Unavailable);
        }
        offset += count;
    }
    Ok(())
}

/// # Errors
/// Requires a private current-user capability, matching compiled origin, valid
/// HMAC and fresh response bound to this request. One deadline covers the exchange.
pub fn forward(root: &Path, origin: &str, body: &[u8]) -> Result<Value, BridgeError> {
    let value = session(root)?;
    if origin != format!("chrome-extension://{}/", value.extension_id) {
        return Err(BridgeError::WrongOrigin);
    }
    validate_native_request(body, now())?;
    let end = Instant::now() + Duration::from_secs(2);
    let pipe = connect(value.id, end)?;
    let stop = AtomicBool::new(false);
    let nonce = Uuid::now_v7();
    let expiry = now().saturating_add(30_000);
    write_packet(
        &pipe,
        &packet(&value, nonce, expiry, body, b"dev/request\0")?,
        end,
        &stop,
    )?;
    let response = read_packet(&pipe, end, &stop)?;
    let (response_nonce, response_expiry, bytes) =
        authenticate(&value, &response, b"dev/response\0", now())?;
    if response_nonce != nonce || response_expiry != expiry {
        return Err(BridgeError::Authentication);
    }
    let response = serde_json::from_slice(bytes).map_err(|_| BridgeError::Malformed)?;
    // DisconnectNamedPipe discards unread bytes. A bounded acknowledgment after
    // verified receipt avoids FlushFileBuffers, which could block indefinitely.
    write_bytes(&pipe, &[1], end, &stop)?;
    Ok(response)
}

pub struct Server {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    root: PathBuf,
    session_id: Uuid,
    _directory: PrivateDirectory,
    _lock: File,
}
impl Server {
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
            && !self.stop.load(Ordering::Acquire)
    }
    /// # Errors
    /// Refuses public/reparse roots, another session, invalid IDs or unsafe pipes.
    pub fn start(
        root: &Path,
        extension_id: &str,
        callback: impl Fn(NativeRequest, &[u8]) -> Value + Send + 'static,
    ) -> Result<Self, BridgeError> {
        if !valid_id(extension_id) {
            return Err(BridgeError::WrongOrigin);
        }
        let directory = PrivateDirectory::create(root).map_err(unavailable)?;
        let lock = exclusive_session(root)?;
        let mut value = Session {
            id: Uuid::now_v7(),
            extension_id: extension_id.into(),
            secret: [0; 32],
        };
        getrandom::fill(&mut value.secret).map_err(unavailable)?;
        let mut listener = PipeListener::bind(&value.id.to_string()).map_err(unavailable)?;
        write_capability(root, &value)?;
        let session_id = value.id;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let handle = thread::Builder::new()
            .name("ort-dev-bridge".into())
            .spawn(move || {
                let mut nonces = ReplayCache::default();
                while !stopping.load(Ordering::Acquire) {
                    let stream = match listener.accept() {
                        Ok(Some(stream)) => stream,
                        Ok(None) => {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(_) => break,
                    };
                    let end = Instant::now() + Duration::from_secs(2);
                    let response = (|| {
                        let bytes = read_packet(&stream, end, &stopping)?;
                        let (nonce, expiry, body) =
                            authenticate(&value, &bytes, b"dev/request\0", now())?;
                        nonces.accept(nonce, expiry, now())?;
                        let request = validate_native_request(body, now())?;
                        let response =
                            serde_json::to_vec(&callback(request, body)).map_err(unavailable)?;
                        packet(&value, nonce, expiry, &response, b"dev/response\0")
                    })();
                    if let Ok(bytes) = response
                        && write_packet(&stream, &bytes, end, &stopping).is_ok()
                    {
                        let mut acknowledged = [0];
                        let _ = read_exact(&stream, &mut acknowledged, end, &stopping);
                    }
                    drop(stream);
                    if listener.disconnect().is_err() {
                        break;
                    }
                }
            })
            .map_err(unavailable);
        match handle {
            Ok(handle) => Ok(Self {
                stop,
                thread: Some(handle),
                root: root.into(),
                session_id,
                _directory: directory,
                _lock: lock,
            }),
            Err(error) => {
                let _ = fs::remove_file(root.join(CAPABILITY));
                Err(error)
            }
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        if session(&self.root).is_ok_and(|value| value.id == self.session_id) {
            let _ = fs::remove_file(self.root.join(CAPABILITY));
        }
    }
}
#[must_use]
pub fn response(code: &str, ready: bool) -> Value {
    json!({"ok":ready,"protocolVersion":1,"value":{"ready":ready},"error":if ready {Value::Null} else {json!({"code":code,"retryable":false})}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    const ID: &str = "abcdefghijklmnopabcdefghijklmnop";
    const STATUS: &[u8] = br#"{"protocolVersion":1,"kind":"bridge.status"}"#;
    fn send(root: &Path, bytes: &[u8]) -> Result<Vec<u8>, BridgeError> {
        let value = session(root)?;
        let end = Instant::now() + Duration::from_secs(2);
        let pipe = connect(value.id, end)?;
        let stop = AtomicBool::new(false);
        write_packet(&pipe, bytes, end, &stop)?;
        let response = read_packet(&pipe, end, &stop)?;
        write_bytes(&pipe, &[1], end, &stop)?;
        Ok(response)
    }
    #[test]
    fn real_private_pipe_roundtrip_exclusive_session_and_cleanup() {
        let directory = ort_windows::testing::tempdir().unwrap();
        let server = Server::start(directory.path(), ID, |_, _| response("", true)).unwrap();
        assert!(server.is_running());
        assert_eq!(
            forward(
                directory.path(),
                &format!("chrome-extension://{ID}/"),
                STATUS
            )
            .unwrap()["value"]["ready"],
            true
        );
        assert!(Server::start(directory.path(), ID, |_, _| Value::Null).is_err());
        assert_eq!(
            forward(directory.path(), "chrome-extension://wrong/", STATUS),
            Err(BridgeError::WrongOrigin)
        );
        drop(server);
        assert!(!directory.path().join(CAPABILITY).exists());
        let next = Server::start(directory.path(), ID, |_, _| response("", true)).unwrap();
        assert!(next.is_running());
    }
    #[test]
    fn replay_tampering_stale_session_and_wrong_direction_never_reach_intake() {
        let directory = ort_windows::testing::tempdir().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let accepted = Arc::clone(&calls);
        let server = Server::start(directory.path(), ID, move |_, _| {
            accepted.fetch_add(1, Ordering::SeqCst);
            response("", true)
        })
        .unwrap();
        let value = session(directory.path()).unwrap();
        let bytes = packet(
            &value,
            Uuid::now_v7(),
            now() + 30_000,
            STATUS,
            b"dev/request\0",
        )
        .unwrap();
        assert!(send(directory.path(), &bytes).is_ok());
        // The server disconnects the single pipe between messages.
        thread::sleep(Duration::from_millis(20));
        assert!(send(directory.path(), &bytes).is_err());
        thread::sleep(Duration::from_millis(20));
        let mut tampered = bytes;
        tampered[HEADER] ^= 1;
        assert!(send(directory.path(), &tampered).is_err());
        thread::sleep(Duration::from_millis(20));
        let reversed = packet(
            &value,
            Uuid::now_v7(),
            now() + 30_000,
            STATUS,
            b"dev/response\0",
        )
        .unwrap();
        assert!(send(directory.path(), &reversed).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        drop(server);
        let _next = Server::start(directory.path(), ID, |_, _| Value::Null).unwrap();
        assert!(send(directory.path(), &tampered).is_err());
    }
    #[test]
    fn incomplete_peer_is_cancelled_on_shutdown_and_public_roots_are_refused() {
        let directory = ort_windows::testing::tempdir().unwrap();
        let server = Server::start(directory.path(), ID, |_, _| Value::Null).unwrap();
        let value = session(directory.path()).unwrap();
        let pipe = PipeConnection::connect(&value.id.to_string()).unwrap();
        let mut operation = pipe.write(&[1]).unwrap();
        let end = Instant::now() + Duration::from_secs(1);
        while operation.poll().unwrap().is_none() && Instant::now() < end {
            thread::sleep(Duration::from_millis(2));
        }
        operation.cancel().unwrap();
        let start = Instant::now();
        drop(server);
        assert!(start.elapsed() < Duration::from_secs(2));
        let public = tempfile::tempdir().unwrap();
        assert!(Server::start(public.path(), ID, |_, _| Value::Null).is_err());
    }
}
