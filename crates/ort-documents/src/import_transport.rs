//! Bounded parent-side worker transport policy, NOT an OS sandbox or pipe driver.
//!
//! A future native adapter must drain both pipes with nonblocking/cancellable I/O,
//! poll the deadline even when a worker is silent, and terminate/reap the entire
//! contained job on every result. Never use `Command::output`/`read_to_end` for
//! hostile output. These types grant no file, process, IPC, or storage authority.

use std::fmt;
use std::time::{Duration, Instant};

use crate::import::{ImportError, InputFormat, MAX_EXTRACTION_BYTES, ValidatedExtraction};

pub const MAX_PIPE_CHUNK_BYTES: usize = 8 * 1024;
pub const MAX_STDERR_BYTES: usize = 16 * 1024;
pub const WORKER_WALL_LIMIT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    #[error("document import was cancelled")]
    Cancelled,
    #[error("document worker exceeded its wall-time limit")]
    TimedOut,
    #[error("document worker output exceeded its limit")]
    OutputLimit,
    #[error("document worker protocol failed")]
    Protocol,
    #[error("document worker did not exit successfully")]
    WorkerFailed,
    #[error("document worker transport failed")]
    Io,
    #[error("document worker output allocation failed")]
    Allocation,
    #[error(transparent)]
    Extraction(#[from] ImportError),
}

/// Events come from the trusted native adapter, never from JSON or a webview.
/// Exit status must be observed from the OS; a worker cannot report its own
/// success. Exit can precede the remaining buffered pipe data and EOFs.
#[derive(Clone, Copy)]
pub enum WorkerEvent<'a> {
    Stdout(&'a [u8]),
    Stderr(&'a [u8]),
    StdoutEof,
    StderrEof,
    Exited { code: Option<i32> },
    IoFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportProgress {
    Pending,
    /// Data/exit evidence is complete. The adapter must still terminate/reap
    /// the contained job and verify cleanup before handing anything to review.
    ReadyForCleanup,
}

/// One extraction, no reusable/resettable session and no content in Debug.
/// Errors are sticky, immediately discard buffered output, and require the
/// adapter to stop the worker. Valid-looking partial output is never returned.
pub struct ExtractionTransport {
    expected_format: InputFormat,
    last_observed: Instant,
    started_at: Instant,
    stdout: Vec<u8>,
    stderr_bytes: usize,
    stdout_eof: bool,
    stderr_eof: bool,
    exited: bool,
    failure: Option<TransportError>,
}

impl ExtractionTransport {
    /// `now` is a parent-owned monotonic timestamp taken BEFORE worker launch.
    #[must_use]
    pub const fn new(expected_format: InputFormat, now: Instant) -> Self {
        Self {
            expected_format,
            last_observed: now,
            started_at: now,
            stdout: Vec::new(),
            stderr_bytes: 0,
            stdout_eof: false,
            stderr_eof: false,
            exited: false,
            failure: None,
        }
    }

    /// Poll independently of pipe activity, including while waiting for exit.
    /// Cancellation takes precedence over success, even after the final bytes.
    ///
    /// # Errors
    /// Any error requires termination/reaping; subsequent calls retain it.
    pub fn poll(
        &mut self,
        now: Instant,
        cancelled: bool,
    ) -> Result<TransportProgress, TransportError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if cancelled {
            return self.fail(TransportError::Cancelled);
        }
        if now < self.last_observed {
            return self.fail(TransportError::Protocol);
        }
        self.last_observed = now;
        if now.duration_since(self.started_at) >= WORKER_WALL_LIMIT {
            return self.fail(TransportError::TimedOut);
        }
        Ok(self.progress())
    }

    /// Accepts bounded chunks; never grows storage before checking the limit.
    /// Stderr is counted and discarded, not decoded, logged, or sent to review.
    ///
    /// # Errors
    /// Rejects excess output, bad event order, clock regression and failed exit.
    pub fn observe(
        &mut self,
        event: WorkerEvent<'_>,
        now: Instant,
    ) -> Result<TransportProgress, TransportError> {
        self.poll(now, false)?;
        match event {
            WorkerEvent::Stdout(bytes) => {
                if self.stdout_eof || bytes.is_empty() {
                    return self.fail(TransportError::Protocol);
                }
                if bytes.len() > MAX_PIPE_CHUNK_BYTES
                    || bytes.len() > MAX_EXTRACTION_BYTES - self.stdout.len()
                {
                    return self.fail(TransportError::OutputLimit);
                }
                // Reserve the full bounded payload once, on the heap, avoiding
                // geometric growth beyond the cap and large stack temporaries.
                if self.stdout.is_empty()
                    && self.stdout.try_reserve_exact(MAX_EXTRACTION_BYTES).is_err()
                {
                    return self.fail(TransportError::Allocation);
                }
                self.stdout.extend_from_slice(bytes);
            }
            WorkerEvent::Stderr(bytes) => {
                if self.stderr_eof || bytes.is_empty() {
                    return self.fail(TransportError::Protocol);
                }
                if bytes.len() > MAX_PIPE_CHUNK_BYTES
                    || bytes.len() > MAX_STDERR_BYTES - self.stderr_bytes
                {
                    return self.fail(TransportError::OutputLimit);
                }
                self.stderr_bytes += bytes.len();
            }
            WorkerEvent::StdoutEof => {
                if self.stdout_eof {
                    return self.fail(TransportError::Protocol);
                }
                self.stdout_eof = true;
            }
            WorkerEvent::StderrEof => {
                if self.stderr_eof {
                    return self.fail(TransportError::Protocol);
                }
                self.stderr_eof = true;
            }
            WorkerEvent::Exited { code } => {
                if self.exited {
                    return self.fail(TransportError::Protocol);
                }
                if code != Some(0) {
                    return self.fail(TransportError::WorkerFailed);
                }
                self.exited = true;
            }
            WorkerEvent::IoFailure => return self.fail(TransportError::Io),
        }
        Ok(self.progress())
    }

    /// Consumes the transport and validates exactly one complete JSON message.
    /// Call ONLY after native job cleanup succeeded; discard on cleanup failure.
    /// This method cannot verify OS containment, cleanup, input type, or truthful
    /// pagination. The future native adapter must supply that independent proof.
    ///
    /// # Errors
    /// Rejects cancellation, timeouts, incomplete transport or invalid extraction.
    pub fn finish(
        mut self,
        now: Instant,
        cancelled: bool,
    ) -> Result<ValidatedExtraction, TransportError> {
        if self.poll(now, cancelled)? != TransportProgress::ReadyForCleanup {
            return self.fail(TransportError::Protocol);
        }
        Ok(ValidatedExtraction::decode(
            &self.stdout,
            self.expected_format,
        )?)
    }

    fn progress(&self) -> TransportProgress {
        if self.stdout_eof && self.stderr_eof && self.exited {
            TransportProgress::ReadyForCleanup
        } else {
            TransportProgress::Pending
        }
    }

    fn fail<T>(&mut self, error: TransportError) -> Result<T, TransportError> {
        self.stdout = Vec::new();
        self.failure = Some(error);
        Err(error)
    }
}

impl fmt::Debug for ExtractionTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExtractionTransport")
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr_bytes)
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}
