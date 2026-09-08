//! Direct-child supervision for the capability-restricted Wasm helper.
//! This uses a distinct policy from the superseded native-parser rlimit receipt.
use crate::MacosWorkerOutput;
use ort_documents::{
    import::{InputFormat, ValidatedExtraction},
    import_source::inspect_source,
    worker_supervisor::NativeWorkerEvent,
};
use rustix::{
    fs::{OFlags, fcntl_getfl, fcntl_setfl},
    io::{Errno, write},
};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
const DEADLINE: Duration = Duration::from_secs(60);
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParserProcessError {
    #[error("the bundled parser helper is unavailable or unverified")]
    Identity,
    #[error("the document could not be imported")]
    Failed,
    #[error("document import was cancelled or exceeded its deadline")]
    Cancelled,
    #[error("parser cleanup could not be confirmed")]
    Cleanup,
}
/// Only native packaging code may supply the expected executable hash. The
/// renderer must never provide the helper path, digest or parser module bytes.
pub struct ParserHelper {
    executable: PathBuf,
    sha256: [u8; 32],
    cdhash: String,
}
impl ParserHelper {
    /// Binds a trusted bundle-relative executable to its packaging manifest.
    ///
    /// # Errors
    /// Rejects relative paths and directory traversal. Full verification occurs
    /// again before each job, inside the operation's absolute deadline.
    pub fn new(
        executable: PathBuf,
        sha256: [u8; 32],
        cdhash: String,
    ) -> Result<Self, ParserProcessError> {
        if cdhash.len() != 40
            || !cdhash.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !executable.is_absolute()
            || executable
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(ParserProcessError::Identity);
        }
        Ok(Self {
            executable,
            sha256,
            cdhash,
        })
    }
    /// Runs one bounded job and accepts output only after success, EOF on both
    /// private pipes and OS-observed reaping. No document path crosses the pipe.
    ///
    /// # Errors
    /// Refuses unverified helpers, malformed input, cancellation, deadlines,
    /// protocol/exit failures and unconfirmed cleanup. Never returns partial text.
    pub fn extract(
        &self,
        format: InputFormat,
        input: &[u8],
        cancel: &AtomicBool,
    ) -> Result<ValidatedExtraction, ParserProcessError> {
        let start = Instant::now();
        inspect_source(input, format).map_err(|_| ParserProcessError::Failed)?;
        verify(&self.executable, &self.sha256, start, cancel)?;
        check(start, cancel)?;
        let child = Command::new(&self.executable)
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ParserProcessError::Failed)?;
        let mut child = OwnedChild(child);
        // The child remains unreaped until identity validation completes, so
        // its PID cannot be reused. No source bytes are sent before this check.
        let result = verify_running(&child.0, &self.cdhash)
            .and_then(|()| check(start, cancel))
            .and_then(|()| exchange(&mut child.0, format, input, start, cancel));
        if result.is_err() {
            child.stop()?;
        }
        result
    }
}
fn check(start: Instant, cancel: &AtomicBool) -> Result<(), ParserProcessError> {
    if cancel.load(Ordering::Acquire) || start.elapsed() >= DEADLINE {
        Err(ParserProcessError::Cancelled)
    } else {
        Ok(())
    }
}
fn verify(
    path: &Path,
    expected: &[u8; 32],
    start: Instant,
    cancel: &AtomicBool,
) -> Result<(), ParserProcessError> {
    let metadata = path
        .symlink_metadata()
        .map_err(|_| ParserProcessError::Identity)?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
        return Err(ParserProcessError::Identity);
    }
    let mut file = std::fs::File::open(path).map_err(|_| ParserProcessError::Identity)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    let mut total = 0usize;
    loop {
        check(start, cancel)?;
        let count = file
            .read(&mut buffer)
            .map_err(|_| ParserProcessError::Identity)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count)
            .filter(|n| *n <= 64 * 1024 * 1024)
            .ok_or(ParserProcessError::Identity)?;
        hasher.update(&buffer[..count]);
    }
    if hasher.finalize().as_slice() != expected {
        return Err(ParserProcessError::Identity);
    }
    // Verification is bounded and never displays signing prompts. App Sandbox
    // entitlements are bound by the expected signed executable's exact digest.
    let mut verification = OwnedChild(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--strict"])
            .arg(path)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| ParserProcessError::Identity)?,
    );
    loop {
        if let Err(error) = check(start, cancel) {
            verification.stop()?;
            return Err(error);
        }
        if let Some(status) = verification
            .0
            .try_wait()
            .map_err(|_| ParserProcessError::Identity)?
        {
            return if status.success() {
                Ok(())
            } else {
                Err(ParserProcessError::Identity)
            };
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
struct OwnedChild(Child);
impl OwnedChild {
    fn stop(&mut self) -> Result<(), ParserProcessError> {
        let _ = self.0.kill();
        let start = Instant::now();
        loop {
            if self
                .0
                .try_wait()
                .map_err(|_| ParserProcessError::Cleanup)?
                .is_some()
            {
                return Ok(());
            }
            if start.elapsed() >= Duration::from_secs(1) {
                return Err(ParserProcessError::Cleanup);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.stop();
        }
    }
}
fn exchange(
    child: &mut Child,
    format: InputFormat,
    input: &[u8],
    start: Instant,
    cancel: &AtomicBool,
) -> Result<ValidatedExtraction, ParserProcessError> {
    let mut stdin = Some(child.stdin.take().ok_or(ParserProcessError::Failed)?);
    let writer = stdin.as_ref().ok_or(ParserProcessError::Failed)?;
    fcntl_setfl(
        writer,
        fcntl_getfl(writer).map_err(|_| ParserProcessError::Failed)? | OFlags::NONBLOCK,
    )
    .map_err(|_| ParserProcessError::Failed)?;
    let mut output = MacosWorkerOutput::new(
        child
            .stdout
            .take()
            .ok_or(ParserProcessError::Failed)?
            .into(),
        child
            .stderr
            .take()
            .ok_or(ParserProcessError::Failed)?
            .into(),
    )
    .map_err(|_| ParserProcessError::Failed)?;
    let mut header = *b"ORTW\0\0\0\0\0";
    header[4] = match format {
        InputFormat::Docx => 1,
        InputFormat::Pdf => 2,
    };
    header[5..].copy_from_slice(
        &u32::try_from(input.len())
            .map_err(|_| ParserProcessError::Failed)?
            .to_le_bytes(),
    );
    let mut sent = 0;
    let mut bytes = vec![];
    let mut eof = [false; 2];
    loop {
        check(start, cancel)?;
        if let Some(writer) = stdin.as_ref() {
            let part = if sent < header.len() {
                &header[sent..]
            } else {
                &input[sent - header.len()..]
            };
            let length = part.len().min(8192);
            match write(writer, &part[..length]) {
                Ok(0) => return Err(ParserProcessError::Failed),
                Ok(count) => sent += count,
                Err(Errno::INTR | Errno::AGAIN) => {}
                Err(_) => return Err(ParserProcessError::Failed),
            }
            if sent == header.len() + input.len() {
                stdin = None;
            }
        }
        let wait = if stdin.is_some() {
            Duration::ZERO
        } else {
            Duration::from_millis(25)
        };
        match output
            .receive(wait)
            .map_err(|_| ParserProcessError::Failed)?
        {
            Some(NativeWorkerEvent::Stdout(chunk)) => {
                bytes
                    .try_reserve(chunk.len())
                    .map_err(|_| ParserProcessError::Failed)?;
                bytes.extend(chunk);
            }
            Some(NativeWorkerEvent::Stderr(_)) | None => {}
            Some(NativeWorkerEvent::StdoutEof) => eof[0] = true,
            Some(NativeWorkerEvent::StderrEof) => eof[1] = true,
            _ => return Err(ParserProcessError::Failed),
        }
        if let Some(status) = child.try_wait().map_err(|_| ParserProcessError::Failed)? {
            if !status.success() || stdin.is_some() {
                return Err(ParserProcessError::Failed);
            }
            if eof == [true, true] {
                check(start, cancel)?;
                return ValidatedExtraction::decode(&bytes, format)
                    .map_err(|_| ParserProcessError::Failed);
            }
        }
        if stdin.is_some() || eof == [true, true] {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

fn verify_running(child: &Child, cdhash: &str) -> Result<(), ParserProcessError> {
    use security_framework::os::macos::code_signing::{
        Flags, GuestAttributes, SecCode, SecRequirement,
    };
    let mut attributes = GuestAttributes::new();
    attributes.set_pid(i32::try_from(child.id()).map_err(|_| ParserProcessError::Identity)?);
    let flags = Flags::NO_NETWORK_ACCESS;
    let code = SecCode::copy_guest_with_attribues(None, &attributes, Flags::NONE)
        .map_err(|_| ParserProcessError::Identity)?;
    let requirement: SecRequirement =
        format!("cdhash H\"{cdhash}\" and entitlement[\"com.apple.security.app-sandbox\"] exists")
            .parse()
            .map_err(|_| ParserProcessError::Identity)?;
    code.check_validity(flags, &requirement)
        .map_err(|_| ParserProcessError::Identity)
}
