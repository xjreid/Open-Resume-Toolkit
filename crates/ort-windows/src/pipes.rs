//! Overlapped local pipes with owned buffers and bounded cancellation cleanup.
//! No OVERLAPPED or buffer is freed while the kernel can still access it.
use crate::security::PrivateSecurity;
use std::{
    cell::UnsafeCell,
    io,
    marker::PhantomPinned,
    mem::zeroed,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    pin::Pin,
    ptr::{null, null_mut},
    sync::Arc,
    time::{Duration, Instant},
};
use windows_sys::Win32::Security::Authorization::SE_FILE_OBJECT;
use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_NO_DATA,
        ERROR_OPERATION_ABORTED, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, ERROR_PIPE_NOT_CONNECTED,
        GENERIC_READ, GENERIC_WRITE, GetLastError, INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, OPEN_EXISTING,
        PIPE_ACCESS_DUPLEX, ReadFile, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT, WriteFile,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_TYPE_BYTE,
        },
        Threading::CreateEventW,
    },
};

const CHUNK: usize = 8 * 1024;
const CLEANUP: Duration = Duration::from_secs(1);

fn unavailable() -> io::Error {
    io::Error::other("Windows pipe unavailable")
}

fn random_pipe_name() -> io::Result<Vec<u16>> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| unavailable())?;
    Ok(format!(
        r"\\.\pipe\com.openresumetoolkit.dev.parser.{}",
        hex::encode(bytes)
    )
    .encode_utf16()
    .chain(Some(0))
    .collect())
}

#[derive(Clone)]
pub struct PipeConnection(Arc<OwnedHandle>);

fn bridge_name(identifier: &str) -> io::Result<Vec<u16>> {
    if identifier.is_empty()
        || identifier.len() > 128
        || !identifier
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'-'))
    {
        return Err(unavailable());
    }
    Ok(
        format!(r"\\.\pipe\com.openresumetoolkit.dev.bridge.{identifier}")
            .encode_utf16()
            .chain(Some(0))
            .collect(),
    )
}

/// Single-instance, current-user local pipe. The server holds the same handle
/// across clients, so there is no disconnect/rebind name-takeover window.
pub struct PipeListener {
    connection: PipeConnection,
    pending: Option<PipeOperation>,
    accepted: bool,
}
impl PipeListener {
    /// # Errors
    /// Refuses a pre-existing name, invalid identifier or failed private DACL.
    pub fn bind(identifier: &str) -> io::Result<Self> {
        let name = bridge_name(identifier)?;
        let security = PrivateSecurity::new()?;
        let attributes = security.attributes();
        // SAFETY: private non-inherited pipe, one instance, local clients only.
        let raw = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                8192,
                8192,
                0,
                &raw const attributes,
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            return Err(unavailable());
        }
        let connection = PipeConnection(Arc::new(unsafe { OwnedHandle::from_raw_handle(raw) }));
        security.verify(connection.0.as_ref(), SE_FILE_OBJECT)?;
        Ok(Self {
            connection,
            pending: None,
            accepted: false,
        })
    }

    /// Nonblocking accept; stop/deadline checks belong to the caller's poll loop.
    /// # Errors
    /// Returns a bounded failure on native connection errors.
    pub fn accept(&mut self) -> io::Result<Option<PipeConnection>> {
        if self.accepted {
            return Ok(None);
        }
        if self.pending.is_none() {
            let mut operation = self.connection.operation(Vec::new())?;
            // SAFETY: owned stable overlapped/event, retained until completion.
            if unsafe { ConnectNamedPipe(self.connection.raw(), operation.overlapped()) } == 0 {
                match unsafe { GetLastError() } {
                    ERROR_PIPE_CONNECTED => operation.completed = Some(0),
                    ERROR_IO_PENDING => operation.pending = true,
                    _ => return Err(unavailable()),
                }
            } else {
                operation.completed = Some(0);
            }
            self.pending = Some(operation);
        }
        if self
            .pending
            .as_mut()
            .ok_or_else(unavailable)?
            .poll()?
            .is_none()
        {
            return Ok(None);
        }
        self.pending = None;
        self.accepted = true;
        Ok(Some(self.connection.clone()))
    }

    /// Call only after the accepted connection's I/O has completed or cancelled.
    /// # Errors
    /// Refuses unconfirmed connection cancellation or disconnect failure.
    pub fn disconnect(&mut self) -> io::Result<()> {
        if let Some(operation) = self.pending.take() {
            operation.cancel()?;
        }
        // SAFETY: caller serializes all operations on this owned server instance.
        if unsafe { DisconnectNamedPipe(self.connection.raw()) } == 0
            && unsafe { GetLastError() } != ERROR_PIPE_NOT_CONNECTED
        {
            return Err(unavailable());
        }
        self.accepted = false;
        Ok(())
    }
}

/// The synchronous child-side handle is inheritable, but only an explicit
/// `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` may transfer it to a contained helper.
pub(super) struct InheritedEnd(pub(super) OwnedHandle);

pub(super) fn inherited_pair() -> io::Result<(PipeConnection, InheritedEnd)> {
    let name = random_pipe_name()?;
    let security = PrivateSecurity::new()?;
    let attributes = security.attributes();
    // SAFETY: complete private DACL applies at creation; server is not inherited.
    let raw = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            4096,
            4096,
            0,
            &raw const attributes,
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(unavailable());
    }
    // SAFETY: CreateNamedPipeW transferred one live handle.
    let server = PipeConnection(Arc::new(unsafe { OwnedHandle::from_raw_handle(raw) }));
    let mut attributes = security.attributes();
    attributes.bInheritHandle = 1;
    // SAFETY: same-process client opens the newly created private pipe. Only
    // this synchronous end is inherited; its handle remains alive through launch.
    let raw = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            &raw const attributes,
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(unavailable());
    }
    // SAFETY: successful CreateFileW transferred one live handle.
    let client = InheritedEnd(unsafe { OwnedHandle::from_raw_handle(raw) });
    let mut connect = server.operation(Vec::new())?;
    // SAFETY: OVERLAPPED storage/event are heap-owned and remain live until completion.
    let result = unsafe { ConnectNamedPipe(server.raw(), connect.overlapped()) };
    if result == 0 {
        // SAFETY: immediately follows the failed Win32 operation.
        match unsafe { GetLastError() } {
            ERROR_PIPE_CONNECTED => {}
            ERROR_IO_PENDING => {
                connect.pending = true;
                let end = Instant::now() + CLEANUP;
                loop {
                    if connect.poll()?.is_some() {
                        break;
                    }
                    if Instant::now() >= end {
                        return Err(unavailable());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            _ => return Err(unavailable()),
        }
    }
    Ok((server, client))
}

impl PipeConnection {
    /// Connects once without blocking on a busy server; callers may retry within
    /// their own deadline. Server ACL verification precedes any capability bytes.
    /// # Errors
    /// Rejects unavailable servers, invalid names and non-private server ACLs.
    pub fn connect(identifier: &str) -> io::Result<Self> {
        let name = bridge_name(identifier)?;
        // SAFETY: no inherited handles; identification-only QoS prevents server
        // impersonation, and overlapped I/O is bounded by the operation owner.
        let raw = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                null_mut(),
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            // SAFETY: directly follows CreateFileW. Busy is retryable within the
            // caller's deadline; ACL/name failures are not retried.
            return Err(if unsafe { GetLastError() } == ERROR_PIPE_BUSY {
                io::Error::new(io::ErrorKind::WouldBlock, "Windows pipe busy")
            } else {
                unavailable()
            });
        }
        let connection = Self(Arc::new(unsafe { OwnedHandle::from_raw_handle(raw) }));
        PrivateSecurity::new()?.verify(connection.0.as_ref(), SE_FILE_OBJECT)?;
        Ok(connection)
    }
    pub(super) fn raw(&self) -> *mut std::ffi::c_void {
        self.0.as_raw_handle()
    }

    fn operation(&self, data: Vec<u8>) -> io::Result<PipeOperation> {
        // SAFETY: unnamed event has no inherited handle and no pointer arguments.
        let event = unsafe { CreateEventW(null(), 1, 0, null()) };
        if event.is_null() {
            return Err(unavailable());
        }
        // SAFETY: CreateEventW transferred one live handle.
        let event = unsafe { OwnedHandle::from_raw_handle(event) };
        // SAFETY: zero is valid initialization for OVERLAPPED; event stays owned beside it.
        let mut overlapped: OVERLAPPED = unsafe { zeroed() };
        overlapped.hEvent = event.as_raw_handle();
        Ok(PipeOperation {
            inner: Some(Box::pin(Operation {
                pipe: self.clone(),
                data: data.into_iter().map(UnsafeCell::new).collect(),
                overlapped: UnsafeCell::new(overlapped),
                _event: event,
                _pin: PhantomPinned,
            })),
            pending: false,
            completed: None,
        })
    }

    /// Starts a bounded read. Poll and inspect bytes only after completion.
    /// # Errors
    /// Returns a bounded I/O error; no untrusted bytes enter diagnostics.
    pub fn read(&self) -> io::Result<PipeOperation> {
        self.read_up_to(CHUNK)
    }

    /// # Errors
    /// Rejects reads outside the fixed per-operation bound or native I/O errors.
    pub fn read_up_to(&self, length: usize) -> io::Result<PipeOperation> {
        if !(1..=CHUNK).contains(&length) {
            return Err(unavailable());
        }
        let mut operation = self.operation(vec![0; length])?;
        let inner = operation.inner.as_ref().ok_or_else(unavailable)?;
        let mut count = 0;
        // SAFETY: the boxed buffer and OVERLAPPED retain stable addresses until
        // poll/cancel confirms completion, including after moving PipeOperation.
        let result = unsafe {
            ReadFile(
                self.raw(),
                inner.data.as_ptr().cast::<u8>().cast_mut(),
                u32::try_from(length).map_err(|_| unavailable())?,
                &raw mut count,
                inner.overlapped.get(),
            )
        };
        operation.started(result, count)?;
        Ok(operation)
    }

    /// Starts one write of at most 8 KiB. The operation owns a copy of its data.
    /// # Errors
    /// Rejects empty/oversized chunks or unavailable pipes.
    pub fn write(&self, bytes: &[u8]) -> io::Result<PipeOperation> {
        if bytes.is_empty() || bytes.len() > CHUNK {
            return Err(unavailable());
        }
        let mut operation = self.operation(bytes.to_vec())?;
        let inner = operation.inner.as_ref().ok_or_else(unavailable)?;
        let mut count = 0;
        // SAFETY: operation owns all memory borrowed by the asynchronous write.
        let result = unsafe {
            WriteFile(
                self.raw(),
                inner.data.as_ptr().cast::<u8>(),
                u32::try_from(inner.data.len()).map_err(|_| unavailable())?,
                &raw mut count,
                inner.overlapped.get(),
            )
        };
        operation.started(result, count)?;
        Ok(operation)
    }
}

struct Operation {
    pipe: PipeConnection,
    // Windows may mutate these while Rust polls other fields. Use shared access
    // to the pinned Operation and explicit interior mutability for kernel memory.
    data: Box<[UnsafeCell<u8>]>,
    overlapped: UnsafeCell<OVERLAPPED>,
    _event: OwnedHandle,
    _pin: PhantomPinned,
}
// SAFETY: OVERLAPPED references only this boxed allocation, its owned event and
// Arc-owned pipe. Pinning prevents moving the operation; its buffer also has a
// stable allocation. CancelIoEx and
// GetOverlappedResult permit cross-thread completion; no thread-affine state is
// borrowed, and Sync is deliberately not implemented.
unsafe impl Send for Operation {}

/// An operation must be drained before buffers/events can be freed. If Windows
/// cannot confirm cancellation, retain the allocation/handle and report cleanup
/// failure. Leaking a bounded chunk is preferable to kernel use-after-free.
pub struct PipeOperation {
    inner: Option<Pin<Box<Operation>>>,
    pending: bool,
    completed: Option<usize>,
}

impl PipeOperation {
    fn overlapped(&self) -> *mut OVERLAPPED {
        self.inner
            .as_ref()
            .expect("owned operation")
            .overlapped
            .get()
    }

    fn started(&mut self, result: i32, count: u32) -> io::Result<()> {
        if result != 0 {
            self.completed = Some(count as usize);
            return Ok(());
        }
        // SAFETY: GetLastError follows ReadFile/WriteFile without another OS call.
        match unsafe { GetLastError() } {
            ERROR_IO_PENDING => {
                self.pending = true;
                Ok(())
            }
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_NO_DATA => {
                self.completed = Some(0);
                Ok(())
            }
            _ => Err(unavailable()),
        }
    }

    /// # Errors
    /// Returns a bounded error when I/O failed. None means it is still pending.
    pub fn poll(&mut self) -> io::Result<Option<usize>> {
        if !self.pending {
            return Ok(self.completed);
        }
        let inner = self.inner.as_ref().ok_or_else(unavailable)?;
        let mut count = 0;
        // SAFETY: handle/OVERLAPPED remain owned; nonblocking status query.
        let result = unsafe {
            GetOverlappedResult(inner.pipe.raw(), inner.overlapped.get(), &raw mut count, 0)
        };
        if result != 0 {
            self.pending = false;
            self.completed = Some(count as usize);
            return Ok(self.completed);
        }
        // SAFETY: immediately follows GetOverlappedResult.
        match unsafe { GetLastError() } {
            ERROR_IO_INCOMPLETE => Ok(None),
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_NO_DATA => {
                self.pending = false;
                self.completed = Some(0);
                Ok(self.completed)
            }
            code => {
                self.pending = false;
                Err(if code == ERROR_OPERATION_ABORTED {
                    io::Error::new(io::ErrorKind::Interrupted, "Windows pipe cancelled")
                } else {
                    unavailable()
                })
            }
        }
    }

    /// # Errors
    /// Bytes cannot be borrowed while Windows may still be writing the buffer.
    pub fn bytes(&self) -> io::Result<&[u8]> {
        if self.pending {
            return Err(unavailable());
        }
        let inner = self.inner.as_ref().ok_or_else(unavailable)?;
        let length = self.completed.ok_or_else(unavailable)?;
        if length > inner.data.len() {
            return Err(unavailable());
        }
        // SAFETY: completion proves Windows no longer accesses this operation's
        // buffer. UnsafeCell<u8> has u8's layout; the checked slice remains
        // borrowed from self, so it cannot be dropped or restarted while borrowed.
        Ok(unsafe { std::slice::from_raw_parts(inner.data.as_ptr().cast::<u8>(), length) })
    }

    fn drain(&mut self) -> io::Result<()> {
        if !self.pending {
            return Ok(());
        }
        let inner = self.inner.as_ref().ok_or_else(unavailable)?;
        // SAFETY: cancellation only marks this operation. Keep every referenced
        // allocation and handle alive until the completion query proves it ended.
        unsafe { CancelIoEx(inner.pipe.raw(), inner.overlapped.get()) };
        let end = Instant::now() + CLEANUP;
        while self.pending {
            let _ = self.poll();
            if !self.pending {
                return Ok(());
            }
            if Instant::now() >= end {
                return Err(unavailable());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    /// Explicit cancellation receipt for supervisors; do not accept worker
    /// output if any operation's cleanup cannot be confirmed.
    /// # Errors
    /// Returns an error after one second if cancellation remains unconfirmed.
    pub fn cancel(mut self) -> io::Result<()> {
        let result = self.drain();
        if result.is_err() {
            self.retain_pending();
        }
        result
    }

    fn retain_pending(&mut self) {
        if let Some(inner) = self.inner.take() {
            // Preserve the pin and all owned allocations/handles indefinitely
            // if Windows has not acknowledged completion. Never move its pointee.
            std::mem::forget(inner);
        }
    }
}

impl Drop for PipeOperation {
    fn drop(&mut self) {
        if self.drain().is_err() {
            self.retain_pending();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{Read, Write},
    };

    fn wait(operation: &mut PipeOperation) -> usize {
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(count) = operation.poll().unwrap() {
                return count;
            }
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn read_buffers_are_inaccessible_until_io_completes_and_eof_is_observed() {
        let (parent, child) = inherited_pair().unwrap();
        let mut read = parent.read().unwrap();
        assert!(read.poll().unwrap().is_none());
        assert!(read.bytes().is_err());
        let mut child = File::from(child.0);
        child.write_all(b"SYNTHETIC").unwrap();
        assert_eq!(wait(&mut read), 9);
        assert_eq!(read.bytes().unwrap(), b"SYNTHETIC");
        drop(read);
        drop(child);
        let mut eof = parent.read().unwrap();
        assert_eq!(wait(&mut eof), 0);
    }

    #[test]
    fn stalled_reads_and_backpressured_writes_cancel_without_blocking_the_owner() {
        let (parent, _child) = inherited_pair().unwrap();
        let read = parent.read().unwrap();
        let start = Instant::now();
        read.cancel().unwrap();
        let write = parent.write(&[0x42; CHUNK]).unwrap();
        write.cancel().unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(parent.write(&[]).is_err());
        assert!(parent.write(&[0; CHUNK + 1]).is_err());
    }

    #[test]
    fn parent_writes_reach_a_synchronous_child_handle() {
        let (parent, child) = inherited_pair().unwrap();
        let mut write = parent.write(b"SYNTHETIC").unwrap();
        let mut child = File::from(child.0);
        let mut buffer = [0; 9];
        child.read_exact(&mut buffer).unwrap();
        assert_eq!(wait(&mut write), 9);
        assert_eq!(&buffer, b"SYNTHETIC");
    }
}
