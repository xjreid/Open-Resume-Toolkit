//! Opt-in macOS development transport. Its temporary capability is deliberately
//! readable by the current account, not a substitute for signed production IPC.
//! No database/provider secret, captured content, or installation key goes here.
use crate::{
    BridgeError, CAPTURE_TTL_MS, MAX_ENVELOPE_BYTES, NativeRequest, ReplayCache,
    authentication_tag, validate_native_request, verify_authentication,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use uuid::Uuid;
use zeroize::Zeroize;

const HEADER: usize = 72; // session UUID, nonce UUID, expiry, extension ID
const TAG: usize = 32;
const MAX_PACKET: usize = MAX_ENVELOPE_BYTES + HEADER + TAG;
const CAPABILITY: &str = "session.json";
const SOCKET: &str = "bridge.sock";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Session {
    id: Uuid,
    extension_id: String,
    secret: [u8; 32],
}
impl Drop for Session {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

/// # Errors
/// Requires a current-user-owned private directory for this dev session.
pub fn default_root() -> PathBuf {
    // Override exists only in this opt-in dev feature, for disposable QA profiles.
    std::env::var_os("ORT_DEV_BRIDGE_DIRECTORY").map_or_else(
        || {
            PathBuf::from(format!(
                "/private/tmp/ort-dev-bridge-{}",
                rustix::process::geteuid().as_raw()
            ))
        },
        PathBuf::from,
    )
}

fn now() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}
fn unavailable<T>(_: T) -> BridgeError {
    BridgeError::Unavailable
}
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|byte| (b'a'..=b'p').contains(&byte))
}
fn private_metadata(path: &Path, directory: bool) -> Result<(), BridgeError> {
    let metadata = fs::symlink_metadata(path).map_err(unavailable)?;
    if metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o777 != if directory { 0o700 } else { 0o600 }
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err(BridgeError::Authentication);
    }
    Ok(())
}
fn prepare_root(root: &Path) -> Result<(), BridgeError> {
    if !root.is_absolute() {
        return Err(BridgeError::Authentication);
    }
    match fs::DirBuilder::new().mode(0o700).create(root) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(unavailable(error)),
    }
    private_metadata(root, true)
}
fn session(root: &Path) -> Result<Session, BridgeError> {
    private_metadata(root, true)?;
    let path = root.join(CAPABILITY);
    private_metadata(&path, false)?;
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits()).map_err(unavailable)?)
        .open(path)
        .map_err(unavailable)?;
    let metadata = file.metadata().map_err(unavailable)?;
    if metadata.len() > 2048
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o777 != 0o600
    {
        return Err(BridgeError::Authentication);
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(2049)
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
            .mode(0o600)
            .open(&temporary)
            .map_err(unavailable)?;
        file.write_all(&bytes).map_err(unavailable)?;
        file.sync_all().map_err(unavailable)?;
        fs::rename(&temporary, root.join(CAPABILITY)).map_err(unavailable)
    })();
    bytes.zeroize();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
fn packet(
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
fn authenticate<'a>(
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
#[cfg(test)]
fn read_packet(input: &mut impl Read) -> Result<Vec<u8>, BridgeError> {
    let mut length = [0; 4];
    input.read_exact(&mut length).map_err(unavailable)?;
    let size = packet_size(length)?;
    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes).map_err(unavailable)?;
    Ok(bytes)
}
fn packet_size(length: [u8; 4]) -> Result<usize, BridgeError> {
    let size = usize::try_from(u32::from_le_bytes(length)).map_err(|_| BridgeError::Oversized)?;
    if !(HEADER + TAG..=MAX_PACKET).contains(&size) {
        return Err(BridgeError::Oversized);
    }
    Ok(size)
}
// One deadline covers the whole frame, even if a peer trickles individual bytes.
// Nonblocking reads avoid macOS SO_RCVTIMEO fractional-duration differences.
fn read_stream_packet(stream: &mut UnixStream) -> Result<Vec<u8>, BridgeError> {
    fn exact(
        stream: &mut UnixStream,
        mut bytes: &mut [u8],
        deadline: Instant,
    ) -> Result<(), BridgeError> {
        while !bytes.is_empty() {
            if Instant::now() >= deadline {
                return Err(BridgeError::Unavailable);
            }
            match stream.read(bytes) {
                Ok(0) => return Err(BridgeError::Unavailable),
                Ok(size) => {
                    bytes = &mut bytes[size..];
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(unavailable(error)),
            }
        }
        Ok(())
    }
    stream.set_nonblocking(true).map_err(unavailable)?;
    let result = (|| {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut length = [0; 4];
        exact(stream, &mut length, deadline)?;
        let mut bytes = vec![0; packet_size(length)?];
        exact(stream, &mut bytes, deadline)?;
        Ok(bytes)
    })();
    stream.set_nonblocking(false).map_err(unavailable)?;
    result
}
fn write_packet(output: &mut impl Write, bytes: &[u8]) -> Result<(), BridgeError> {
    if bytes.len() > MAX_PACKET {
        return Err(BridgeError::Oversized);
    }
    output
        .write_all(
            &u32::try_from(bytes.len())
                .map_err(|_| BridgeError::Oversized)?
                .to_le_bytes(),
        )
        .map_err(unavailable)?;
    output.write_all(bytes).map_err(unavailable)
}
fn bound_stream(stream: &UnixStream) -> Result<(), BridgeError> {
    // Darwin can inherit O_NONBLOCK from the listening socket on accept.
    stream.set_nonblocking(false).map_err(unavailable)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(unavailable)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(unavailable)
}

/// # Errors
/// Rejects missing/private-invalid capabilities, stale sessions and invalid tags.
/// The supplied origin must already pass the native host's compiled allowlist.
pub fn forward(root: &Path, origin: &str, body: &[u8]) -> Result<Value, BridgeError> {
    let value = session(root)?;
    if origin != format!("chrome-extension://{}/", value.extension_id) {
        return Err(BridgeError::WrongOrigin);
    }
    let _request = validate_native_request(body, now())?;
    let mut stream = UnixStream::connect(root.join(SOCKET)).map_err(unavailable)?;
    bound_stream(&stream)?;
    let nonce = Uuid::now_v7();
    let expiry = now().saturating_add(30_000);
    write_packet(
        &mut stream,
        &packet(&value, nonce, expiry, body, b"dev/request\0")?,
    )?;
    let response = read_stream_packet(&mut stream)?;
    let (response_nonce, response_expiry, bytes) =
        authenticate(&value, &response, b"dev/response\0", now())?;
    if response_nonce != nonce || response_expiry != expiry {
        return Err(BridgeError::Authentication);
    }
    serde_json::from_slice(bytes).map_err(|_| BridgeError::Malformed)
}

pub struct Server {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    root: PathBuf,
    session_id: Uuid,
}
impl Server {
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
            && !self.stop.load(Ordering::Acquire)
    }
    /// Starts only after explicit desktop activation in the isolated dev identity.
    /// # Errors
    /// Refuses unsafe directories, invalid IDs, or another live socket.
    pub fn start(
        root: &Path,
        extension_id: &str,
        callback: impl Fn(NativeRequest, &[u8]) -> Value + Send + 'static,
    ) -> Result<Self, BridgeError> {
        if !valid_id(extension_id) {
            return Err(BridgeError::WrongOrigin);
        }
        prepare_root(root)?;
        let socket = root.join(SOCKET);
        if let Ok(metadata) = fs::symlink_metadata(&socket) {
            if !metadata.file_type().is_socket()
                || metadata.uid() != rustix::process::geteuid().as_raw()
            {
                return Err(BridgeError::Authentication);
            }
            match UnixStream::connect(&socket) {
                Ok(_) => return Err(BridgeError::Unavailable),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                    ) =>
                {
                    fs::remove_file(&socket).map_err(unavailable)?;
                }
                Err(error) => return Err(unavailable(error)),
            }
        }
        let listener = UnixListener::bind(&socket).map_err(unavailable)?;
        let setup = (|| {
            fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).map_err(unavailable)?;
            listener.set_nonblocking(true).map_err(unavailable)?;
            let mut value = Session {
                id: Uuid::now_v7(),
                extension_id: extension_id.to_owned(),
                secret: [0; 32],
            };
            getrandom::fill(&mut value.secret).map_err(unavailable)?;
            write_capability(root, &value)?;
            Ok(value)
        })();
        let value = match setup {
            Ok(value) => value,
            Err(error) => {
                let _ = fs::remove_file(socket);
                return Err(error);
            }
        };
        let session_id = value.id;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let handle = thread::spawn(move || {
            let mut nonces = ReplayCache::default();
            while !stopping.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let response = (|| {
                            bound_stream(&stream)?;
                            let bytes = read_stream_packet(&mut stream)?;
                            let (nonce, expiry, body) =
                                authenticate(&value, &bytes, b"dev/request\0", now())?;
                            nonces.accept(nonce, expiry, now())?;
                            let request = validate_native_request(body, now())?;
                            let response = serde_json::to_vec(&callback(request, body))
                                .map_err(unavailable)?;
                            packet(&value, nonce, expiry, &response, b"dev/response\0")
                        })();
                        if let Ok(bytes) = response {
                            let _ = write_packet(&mut stream, &bytes);
                        }
                        // Authentication failures deliberately receive no detailed oracle.
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            stop,
            thread: Some(handle),
            root: root.to_owned(),
            session_id,
        })
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
            let _ = fs::remove_file(self.root.join(SOCKET));
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
    const ID: &str = "abcdefghijklmnopabcdefghijklmnop";
    #[test]
    fn real_socket_roundtrip_and_cleanup() {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let server = Server::start(directory.path(), ID, |_, _| response("", true)).unwrap();
        let value = forward(
            directory.path(),
            &format!("chrome-extension://{ID}/"),
            br#"{"protocolVersion":1,"kind":"bridge.status"}"#,
        )
        .unwrap();
        assert_eq!(value["value"]["ready"], true);
        assert!(Server::start(directory.path(), ID, |_, _| Value::Null).is_err());
        assert!(
            forward(
                directory.path(),
                "chrome-extension://wrong/",
                br#"{"protocolVersion":1,"kind":"bridge.status"}"#
            )
            .is_err()
        );
        drop(server);
        assert!(!directory.path().join(CAPABILITY).exists());
        assert!(!directory.path().join(SOCKET).exists());
    }
    #[test]
    fn live_server_rejects_replayed_and_invalid_packets_without_intake() {
        use std::sync::atomic::AtomicUsize;
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let accepted = calls.clone();
        let _server = Server::start(directory.path(), ID, move |_, _| {
            accepted.fetch_add(1, Ordering::SeqCst);
            response("", true)
        })
        .unwrap();
        let value = session(directory.path()).unwrap();
        let bytes = packet(
            &value,
            Uuid::now_v7(),
            now() + 30_000,
            br#"{"protocolVersion":1,"kind":"bridge.status"}"#,
            b"dev/request\0",
        )
        .unwrap();
        let send = |bytes: &[u8]| {
            let mut stream = UnixStream::connect(directory.path().join(SOCKET)).unwrap();
            bound_stream(&stream).unwrap();
            write_packet(&mut stream, bytes).unwrap();
            read_stream_packet(&mut stream)
        };
        assert!(send(&bytes).is_ok());
        assert!(send(&bytes).is_err());
        let mut changed = bytes;
        changed[HEADER] ^= 1;
        assert!(send(&changed).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn slow_incomplete_peer_does_not_hold_disconnect_indefinitely() {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let server = Server::start(directory.path(), ID, |_, _| Value::Null).unwrap();
        let mut peer = UnixStream::connect(directory.path().join(SOCKET)).unwrap();
        peer.write_all(&[1]).unwrap();
        thread::sleep(Duration::from_millis(50));
        let start = Instant::now();
        drop(server);
        assert!(start.elapsed() < Duration::from_secs(3));
        assert!(!directory.path().join(CAPABILITY).exists());
    }
    #[test]
    fn rejects_tampering_wrong_session_direction_expiry_and_replay() {
        let value = Session {
            id: Uuid::now_v7(),
            extension_id: ID.into(),
            secret: [7; 32],
        };
        let nonce = Uuid::now_v7();
        let bytes = packet(&value, nonce, 1000, b"{}", b"dev/request\0").unwrap();
        assert!(authenticate(&value, &bytes, b"dev/response\0", 0).is_err());
        assert!(authenticate(&value, &bytes, b"dev/request\0", 1000).is_err());
        let mut changed = bytes.clone();
        changed[HEADER] ^= 1;
        assert!(authenticate(&value, &changed, b"dev/request\0", 0).is_err());
        let other = Session {
            id: Uuid::now_v7(),
            extension_id: ID.into(),
            secret: [7; 32],
        };
        assert!(authenticate(&other, &bytes, b"dev/request\0", 0).is_err());
        let (id, expiry, _) = authenticate(&value, &bytes, b"dev/request\0", 0).unwrap();
        let mut replay = ReplayCache::default();
        replay.accept(id, expiry, 0).unwrap();
        assert_eq!(replay.accept(id, expiry, 0), Err(BridgeError::Replay));
        assert!(read_packet(&mut std::io::Cursor::new(u32::MAX.to_le_bytes())).is_err());
    }
    #[test]
    fn rejects_public_directory_and_symlink_capability() {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Server::start(directory.path(), ID, |_, _| Value::Null).is_err());
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink("/does-not-exist", directory.path().join(CAPABILITY)).unwrap();
        assert!(session(directory.path()).is_err());
    }
}
