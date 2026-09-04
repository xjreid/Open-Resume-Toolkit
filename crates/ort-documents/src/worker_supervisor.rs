//! Fail-closed orchestration between a trusted native containment adapter and
//! the bounded extraction transport.
//!
//! This module does not create an App Sandbox, `AppContainer`, XPC service, Job
//! Object, pipe, or parser. A platform adapter must establish those boundaries
//! before parser code runs and report only parent-observed facts here. The
//! supervisor always requests whole-job termination and verified cleanup,
//! including after apparently successful worker exit. No extraction is returned
//! until that cleanup succeeds.

use std::cmp::min;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::import::{InputFormat, ValidatedExtraction};
use crate::import_transport::{
    ExtractionTransport, TransportError, TransportProgress, WORKER_WALL_LIMIT, WorkerEvent,
};

pub const WORKER_MEMORY_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
pub const WORKER_CPU_LIMIT: Duration = Duration::from_secs(30);
pub const WORKER_HANDLE_LIMIT: u32 = 64;
pub const SUPERVISOR_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub const CLEANUP_LIMIT: Duration = Duration::from_secs(5);
pub const OPERATION_NONCE_BYTES: usize = 16;

const COMMON_CONTROLS: u32 =
    1 << 0 | 1 << 1 | 1 << 2 | 1 << 3 | 1 << 4 | 1 << 5 | 1 << 6 | 1 << 7 | 1 << 8;
const MACOS_CONTROLS: u32 = COMMON_CONTROLS | 1 << 9 | 1 << 10 | 1 << 11 | 1 << 12;
const WINDOWS_CONTROLS: u32 = COMMON_CONTROLS | 1 << 9 | 1 << 10 | 1 << 11 | 1 << 12 | 1 << 13;

/// Native isolation mechanism selected before any untrusted bytes are parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerPlatform {
    MacOsAppSandboxXpc,
    WindowsAppContainerJob,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlEvidence {
    Observed,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonLaunchControls {
    pub fixed_verified_executable: ControlEvidence,
    pub minimal_environment: ControlEvidence,
    pub exact_descriptor_allowlist: ControlEvidence,
    pub read_only_input: ControlEvidence,
    pub private_output: ControlEvidence,
    pub network_denied: ControlEvidence,
    pub child_creation_denied: ControlEvidence,
    pub unrelated_files_denied: ControlEvidence,
    pub credentials_and_ipc_denied: ControlEvidence,
}

impl CommonLaunchControls {
    fn bits(self) -> u32 {
        controls(&[
            self.fixed_verified_executable,
            self.minimal_environment,
            self.exact_descriptor_allowlist,
            self.read_only_input,
            self.private_output,
            self.network_denied,
            self.child_creation_denied,
            self.unrelated_files_denied,
            self.credentials_and_ipc_denied,
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacOsLaunchControls {
    pub helper_signature_verified: ControlEvidence,
    pub app_sandbox_effective: ControlEvidence,
    pub sandbox_inheritance_effective: ControlEvidence,
    pub no_shared_entitlements: ControlEvidence,
}

impl MacOsLaunchControls {
    fn bits(self) -> u32 {
        controls(&[
            self.helper_signature_verified,
            self.app_sandbox_effective,
            self.sandbox_inheritance_effective,
            self.no_shared_entitlements,
        ]) << 9
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowsLaunchControls {
    pub appcontainer_token_effective: ControlEvidence,
    pub zero_capabilities: ControlEvidence,
    pub job_bound_before_execution: ControlEvidence,
    pub kill_on_job_close: ControlEvidence,
    pub no_breakaway: ControlEvidence,
}

impl WindowsLaunchControls {
    fn bits(self) -> u32 {
        controls(&[
            self.appcontainer_token_effective,
            self.zero_capabilities,
            self.job_bound_before_execution,
            self.kill_on_job_close,
            self.no_breakaway,
        ]) << 9
    }
}

/// Exact resource ceilings established before parser execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerLimits {
    pub memory_bytes: u64,
    pub cpu_time: Duration,
    pub open_handles: u32,
    pub wall_time: Duration,
    pub core_dump_bytes: u64,
}

impl WorkerLimits {
    #[must_use]
    pub const fn production() -> Self {
        Self {
            memory_bytes: WORKER_MEMORY_LIMIT_BYTES,
            cpu_time: WORKER_CPU_LIMIT,
            open_handles: WORKER_HANDLE_LIMIT,
            wall_time: WORKER_WALL_LIMIT,
            core_dump_bytes: 0,
        }
    }

    fn is_acceptable(self) -> bool {
        self.memory_bytes > 0
            && self.memory_bytes <= WORKER_MEMORY_LIMIT_BYTES
            && !self.cpu_time.is_zero()
            && self.cpu_time <= WORKER_CPU_LIMIT
            && self.open_handles > 0
            && self.open_handles <= WORKER_HANDLE_LIMIT
            && self.wall_time == WORKER_WALL_LIMIT
            && self.core_dump_bytes == 0
    }
}

/// Parent-observed launch facts supplied by the trusted native adapter.
///
/// The bit mask is deliberately not public. Callers enumerate every measured
/// control instead of accepting an open-ended "sandboxed" boolean. This receipt
/// is an orchestration precondition, not independent proof that the OS enforced
/// the claimed policy.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LaunchReceipt {
    operation_nonce: [u8; OPERATION_NONCE_BYTES],
    platform: WorkerPlatform,
    limits: WorkerLimits,
    controls: u32,
}

impl LaunchReceipt {
    /// Creates a macOS receipt after a separately signed XPC supervisor has
    /// measured every supplied condition before launching its fixed child.
    #[must_use]
    pub fn macos(
        operation_nonce: [u8; OPERATION_NONCE_BYTES],
        common: CommonLaunchControls,
        platform: MacOsLaunchControls,
        limits: WorkerLimits,
    ) -> Self {
        Self {
            operation_nonce,
            platform: WorkerPlatform::MacOsAppSandboxXpc,
            limits,
            controls: common.bits() | platform.bits(),
        }
    }

    /// Creates a Windows receipt after the child is bound to its restricted
    /// token, explicit handle list, mitigation policy, and Job Object before it
    /// can execute parser code.
    #[must_use]
    pub fn windows(
        operation_nonce: [u8; OPERATION_NONCE_BYTES],
        common: CommonLaunchControls,
        platform: WindowsLaunchControls,
        limits: WorkerLimits,
    ) -> Self {
        Self {
            operation_nonce,
            platform: WorkerPlatform::WindowsAppContainerJob,
            limits,
            controls: common.bits() | platform.bits(),
        }
    }

    fn validate(self) -> Result<(), SupervisionError> {
        let expected = match self.platform {
            WorkerPlatform::MacOsAppSandboxXpc => MACOS_CONTROLS,
            WorkerPlatform::WindowsAppContainerJob => WINDOWS_CONTROLS,
        };
        if self.operation_nonce == [0; OPERATION_NONCE_BYTES]
            || self.controls != expected
            || !self.limits.is_acceptable()
        {
            return Err(SupervisionError::ContainmentUnavailable);
        }
        Ok(())
    }

    #[must_use]
    pub const fn platform(self) -> WorkerPlatform {
        self.platform
    }

    #[must_use]
    pub const fn limits(self) -> WorkerLimits {
        self.limits
    }
}

impl fmt::Debug for LaunchReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LaunchReceipt")
            .field("platform", &self.platform)
            .field("limits", &self.limits)
            .field("all_controls_observed", &(self.validate().is_ok()))
            .finish_non_exhaustive()
    }
}

const fn controls(values: &[ControlEvidence]) -> u32 {
    let mut result = 0;
    let mut index = 0;
    while index < values.len() {
        if matches!(values[index], ControlEvidence::Observed) {
            result |= 1 << index;
        }
        index += 1;
    }
    result
}

/// Owned events produced by a native adapter's bounded, fair pipe driver.
/// Content is intentionally omitted from `Debug` output.
pub enum NativeWorkerEvent {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    StdoutEof,
    StderrEof,
    Exited { code: Option<i32> },
}

impl fmt::Debug for NativeWorkerEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdout(bytes) => formatter
                .debug_struct("Stdout")
                .field("bytes", &bytes.len())
                .finish(),
            Self::Stderr(bytes) => formatter
                .debug_struct("Stderr")
                .field("bytes", &bytes.len())
                .finish(),
            Self::StdoutEof => formatter.write_str("StdoutEof"),
            Self::StderrEof => formatter.write_str("StderrEof"),
            Self::Exited { code } => formatter
                .debug_struct("Exited")
                .field("code", code)
                .finish(),
        }
    }
}

/// Cleanup facts observed after whole-job termination was requested.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CleanupReceipt {
    pub operation_nonce: [u8; OPERATION_NONCE_BYTES],
    pub worker_reaped: ControlEvidence,
    pub process_tree_empty: ControlEvidence,
    pub stdout_closed: ControlEvidence,
    pub stderr_closed: ControlEvidence,
    pub input_closed: ControlEvidence,
    pub output_closed: ControlEvidence,
    pub containment_closed: ControlEvidence,
}

impl fmt::Debug for CleanupReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CleanupReceipt")
            .field("complete", &self.complete())
            .finish_non_exhaustive()
    }
}

impl CleanupReceipt {
    const fn complete(self) -> bool {
        matches!(self.worker_reaped, ControlEvidence::Observed)
            && matches!(self.process_tree_empty, ControlEvidence::Observed)
            && matches!(self.stdout_closed, ControlEvidence::Observed)
            && matches!(self.stderr_closed, ControlEvidence::Observed)
            && matches!(self.input_closed, ControlEvidence::Observed)
            && matches!(self.output_closed, ControlEvidence::Observed)
            && matches!(self.containment_closed, ControlEvidence::Observed)
    }
}

/// Narrow interface implemented by reviewed macOS and Windows native adapters.
/// `receive` must return within `maximum_wait` even when both pipes are silent.
/// `terminate_tree` must target the owned XPC child/job, never a reported PID.
pub trait ContainedWorker {
    fn launch_receipt(&self) -> LaunchReceipt;

    /// Returns one parent-observed event, or `None` when the bounded wait elapsed.
    ///
    /// # Errors
    /// Returns a content-free error when the pipe/event driver cannot continue.
    fn receive(
        &mut self,
        maximum_wait: Duration,
    ) -> Result<Option<NativeWorkerEvent>, NativeAdapterError>;

    /// Requests termination of the complete containment object on every path.
    ///
    /// # Errors
    /// Returns by `deadline` with a content-free error; cleanup must still be
    /// attempted afterwards.
    fn terminate_tree(&mut self, deadline: Instant) -> Result<(), NativeAdapterError>;

    /// Reaps the worker, drains/closes pipes and source/result handles, verifies
    /// the process tree is empty, and destroys the containment object.
    ///
    /// # Errors
    /// Must return by `deadline` or fail closed with a content-free error.
    fn cleanup(&mut self, deadline: Instant) -> Result<CleanupReceipt, NativeAdapterError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NativeAdapterError {
    #[error("native document-worker adapter is unavailable")]
    Unavailable,
    #[error("native document-worker event transport failed")]
    EventTransport,
    #[error("native document-worker termination failed")]
    Termination,
    #[error("native document-worker cleanup failed")]
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SupervisionError {
    #[error("required native document-worker containment is unavailable")]
    ContainmentUnavailable,
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("native document-worker supervision failed")]
    Native,
    #[error("document-worker cleanup could not be verified")]
    CleanupUnverified,
}

/// Supervises exactly one already-contained worker and always consumes it.
///
/// The adapter must have established the reported OS boundary before this call.
/// Import remains separately feature-gated; this function does not enable it.
///
/// # Errors
/// Returns no extraction on an invalid launch receipt, cancellation, timeout,
/// transport/protocol failure, native adapter failure, or incomplete cleanup.
pub fn supervise<W: ContainedWorker>(
    worker: &mut W,
    expected_format: InputFormat,
    cancelled: &AtomicBool,
) -> Result<ValidatedExtraction, SupervisionError> {
    supervise_with_clock(worker, expected_format, cancelled, &SystemClock)
}

trait Clock {
    fn now(&self) -> Instant;
}

struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

fn supervise_with_clock<W: ContainedWorker, C: Clock>(
    worker: &mut W,
    expected_format: InputFormat,
    cancelled: &AtomicBool,
    clock: &C,
) -> Result<ValidatedExtraction, SupervisionError> {
    let started_at = clock.now();
    let mut transport = ExtractionTransport::new(expected_format, started_at);
    let launch_receipt = worker.launch_receipt();
    let mut result = launch_receipt.validate();

    while result.is_ok() {
        let now = clock.now();
        match transport.poll(now, cancelled.load(Ordering::Acquire)) {
            Ok(TransportProgress::ReadyForCleanup) => break,
            Err(error) => {
                result = Err(error.into());
                break;
            }
            Ok(TransportProgress::Pending) => {}
        }
        let elapsed = now.saturating_duration_since(started_at);
        let remaining = WORKER_WALL_LIMIT.saturating_sub(elapsed);
        let wait = min(SUPERVISOR_POLL_INTERVAL, remaining);
        match worker.receive(wait) {
            Ok(Some(event)) => {
                let observed_at = clock.now();
                if let Err(error) = observe(&mut transport, event, observed_at) {
                    result = Err(error.into());
                }
            }
            Ok(None) => {}
            Err(_) => {
                let _ = transport.observe(WorkerEvent::IoFailure, clock.now());
                result = Err(SupervisionError::Native);
            }
        }
    }

    // A termination request is mandatory even if launch attestation, parsing,
    // output, or exit status already failed. Cleanup evidence, not a sent kill,
    // controls whether any otherwise-valid extraction may escape this boundary.
    let cleanup_deadline = clock.now() + CLEANUP_LIMIT;
    let termination_failed = worker.terminate_tree(cleanup_deadline).is_err();
    let cleanup = worker.cleanup(cleanup_deadline);
    let cleanup_missed_deadline = clock.now() > cleanup_deadline;
    if termination_failed
        || cleanup_missed_deadline
        || !matches!(cleanup, Ok(receipt) if receipt.complete()
            && receipt.operation_nonce == launch_receipt.operation_nonce)
    {
        return Err(SupervisionError::CleanupUnverified);
    }
    result?;
    transport
        .finish(clock.now(), cancelled.load(Ordering::Acquire))
        .map_err(Into::into)
}

fn observe(
    transport: &mut ExtractionTransport,
    event: NativeWorkerEvent,
    now: Instant,
) -> Result<TransportProgress, TransportError> {
    match event {
        NativeWorkerEvent::Stdout(bytes) => transport.observe(WorkerEvent::Stdout(&bytes), now),
        NativeWorkerEvent::Stderr(bytes) => transport.observe(WorkerEvent::Stderr(&bytes), now),
        NativeWorkerEvent::StdoutEof => transport.observe(WorkerEvent::StdoutEof, now),
        NativeWorkerEvent::StderrEof => transport.observe(WorkerEvent::StderrEof, now),
        NativeWorkerEvent::Exited { code } => transport.observe(WorkerEvent::Exited { code }, now),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicBool;

    use super::*;

    const WIRE: &[u8] = br#"{"version":1,"format":"docx","pageCount":1,"blocks":[{"page":1,"kind":"paragraph","text":"SYNTHETIC_PRIVATE_MARKER"}]}"#;
    const NONCE: [u8; OPERATION_NONCE_BYTES] = [0x5a; OPERATION_NONCE_BYTES];

    struct TestClock {
        now: Cell<Instant>,
    }

    impl TestClock {
        fn new() -> Self {
            Self {
                now: Cell::new(Instant::now()),
            }
        }

        fn advance(&self, duration: Duration) {
            self.now.set(self.now.get() + duration);
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            self.now.get()
        }
    }

    struct MockWorker<'a> {
        receipt: LaunchReceipt,
        events: VecDeque<Result<Option<NativeWorkerEvent>, NativeAdapterError>>,
        clock: &'a TestClock,
        advance_per_receive: Duration,
        advance_during_cleanup: Duration,
        terminate_result: Result<(), NativeAdapterError>,
        cleanup_result: Result<CleanupReceipt, NativeAdapterError>,
        receives: usize,
        termination_calls: usize,
        cleanup_calls: usize,
    }

    impl ContainedWorker for MockWorker<'_> {
        fn launch_receipt(&self) -> LaunchReceipt {
            self.receipt
        }

        fn receive(
            &mut self,
            maximum_wait: Duration,
        ) -> Result<Option<NativeWorkerEvent>, NativeAdapterError> {
            assert!(maximum_wait <= SUPERVISOR_POLL_INTERVAL);
            self.receives += 1;
            self.clock.advance(self.advance_per_receive);
            self.events.pop_front().unwrap_or(Ok(None))
        }

        fn terminate_tree(&mut self, deadline: Instant) -> Result<(), NativeAdapterError> {
            assert!(deadline >= self.clock.now());
            self.termination_calls += 1;
            self.terminate_result
        }

        fn cleanup(&mut self, deadline: Instant) -> Result<CleanupReceipt, NativeAdapterError> {
            assert!(deadline >= self.clock.now());
            self.cleanup_calls += 1;
            self.clock.advance(self.advance_during_cleanup);
            self.cleanup_result
        }
    }

    fn launch(platform: WorkerPlatform) -> LaunchReceipt {
        let common = CommonLaunchControls {
            fixed_verified_executable: ControlEvidence::Observed,
            minimal_environment: ControlEvidence::Observed,
            exact_descriptor_allowlist: ControlEvidence::Observed,
            read_only_input: ControlEvidence::Observed,
            private_output: ControlEvidence::Observed,
            network_denied: ControlEvidence::Observed,
            child_creation_denied: ControlEvidence::Observed,
            unrelated_files_denied: ControlEvidence::Observed,
            credentials_and_ipc_denied: ControlEvidence::Observed,
        };
        match platform {
            WorkerPlatform::MacOsAppSandboxXpc => LaunchReceipt::macos(
                NONCE,
                common,
                MacOsLaunchControls {
                    helper_signature_verified: ControlEvidence::Observed,
                    app_sandbox_effective: ControlEvidence::Observed,
                    sandbox_inheritance_effective: ControlEvidence::Observed,
                    no_shared_entitlements: ControlEvidence::Observed,
                },
                WorkerLimits::production(),
            ),
            WorkerPlatform::WindowsAppContainerJob => LaunchReceipt::windows(
                NONCE,
                common,
                WindowsLaunchControls {
                    appcontainer_token_effective: ControlEvidence::Observed,
                    zero_capabilities: ControlEvidence::Observed,
                    job_bound_before_execution: ControlEvidence::Observed,
                    kill_on_job_close: ControlEvidence::Observed,
                    no_breakaway: ControlEvidence::Observed,
                },
                WorkerLimits::production(),
            ),
        }
    }

    const fn complete_cleanup() -> CleanupReceipt {
        CleanupReceipt {
            operation_nonce: NONCE,
            worker_reaped: ControlEvidence::Observed,
            process_tree_empty: ControlEvidence::Observed,
            stdout_closed: ControlEvidence::Observed,
            stderr_closed: ControlEvidence::Observed,
            input_closed: ControlEvidence::Observed,
            output_closed: ControlEvidence::Observed,
            containment_closed: ControlEvidence::Observed,
        }
    }

    fn success_events() -> VecDeque<Result<Option<NativeWorkerEvent>, NativeAdapterError>> {
        [
            Ok(Some(NativeWorkerEvent::Stdout(WIRE.to_vec()))),
            Ok(Some(NativeWorkerEvent::Exited { code: Some(0) })),
            Ok(Some(NativeWorkerEvent::StdoutEof)),
            Ok(Some(NativeWorkerEvent::StderrEof)),
        ]
        .into()
    }

    fn worker(clock: &TestClock, platform: WorkerPlatform) -> MockWorker<'_> {
        MockWorker {
            receipt: launch(platform),
            events: success_events(),
            clock,
            advance_per_receive: Duration::ZERO,
            advance_during_cleanup: Duration::ZERO,
            terminate_result: Ok(()),
            cleanup_result: Ok(complete_cleanup()),
            receives: 0,
            termination_calls: 0,
            cleanup_calls: 0,
        }
    }

    #[test]
    fn macos_and_windows_receipts_can_reach_output_only_after_cleanup() {
        for platform in [
            WorkerPlatform::MacOsAppSandboxXpc,
            WorkerPlatform::WindowsAppContainerJob,
        ] {
            let clock = TestClock::new();
            let mut worker = worker(&clock, platform);
            let extraction = supervise_with_clock(
                &mut worker,
                InputFormat::Docx,
                &AtomicBool::new(false),
                &clock,
            )
            .unwrap();
            assert_eq!(extraction.blocks()[0].text, "SYNTHETIC_PRIVATE_MARKER");
            assert_eq!(worker.termination_calls, 1);
            assert_eq!(worker.cleanup_calls, 1);
        }
    }

    #[test]
    fn incomplete_platform_controls_fail_before_receiving_and_still_cleanup() {
        for platform in [
            WorkerPlatform::MacOsAppSandboxXpc,
            WorkerPlatform::WindowsAppContainerJob,
        ] {
            let valid = launch(platform);
            for bit in 0..u32::BITS {
                let flag = 1_u32 << bit;
                if valid.controls & flag == 0 {
                    continue;
                }
                let clock = TestClock::new();
                let mut worker = worker(&clock, platform);
                worker.receipt.controls &= !flag;
                assert_eq!(
                    supervise_with_clock(
                        &mut worker,
                        InputFormat::Docx,
                        &AtomicBool::new(false),
                        &clock
                    )
                    .unwrap_err(),
                    SupervisionError::ContainmentUnavailable
                );
                assert_eq!(worker.receives, 0);
                assert_eq!(worker.termination_calls, 1);
                assert_eq!(worker.cleanup_calls, 1);
            }
        }
    }

    #[test]
    fn weak_or_incomplete_resource_limits_are_rejected() {
        let clock = TestClock::new();
        for bad_limits in [
            WorkerLimits {
                memory_bytes: 0,
                ..WorkerLimits::production()
            },
            WorkerLimits {
                memory_bytes: WORKER_MEMORY_LIMIT_BYTES + 1,
                ..WorkerLimits::production()
            },
            WorkerLimits {
                cpu_time: Duration::ZERO,
                ..WorkerLimits::production()
            },
            WorkerLimits {
                cpu_time: WORKER_CPU_LIMIT + Duration::from_nanos(1),
                ..WorkerLimits::production()
            },
            WorkerLimits {
                open_handles: 0,
                ..WorkerLimits::production()
            },
            WorkerLimits {
                open_handles: WORKER_HANDLE_LIMIT + 1,
                ..WorkerLimits::production()
            },
            WorkerLimits {
                wall_time: WORKER_WALL_LIMIT + Duration::from_secs(1),
                ..WorkerLimits::production()
            },
            WorkerLimits {
                core_dump_bytes: 1,
                ..WorkerLimits::production()
            },
        ] {
            let mut worker = worker(&clock, WorkerPlatform::MacOsAppSandboxXpc);
            worker.receipt.limits = bad_limits;
            assert_eq!(
                supervise_with_clock(
                    &mut worker,
                    InputFormat::Docx,
                    &AtomicBool::new(false),
                    &clock
                )
                .unwrap_err(),
                SupervisionError::ContainmentUnavailable
            );
        }

        let mut worker = worker(&clock, WorkerPlatform::WindowsAppContainerJob);
        worker.receipt.operation_nonce = [0; OPERATION_NONCE_BYTES];
        worker.cleanup_result.as_mut().unwrap().operation_nonce = [0; OPERATION_NONCE_BYTES];
        assert_eq!(
            supervise_with_clock(
                &mut worker,
                InputFormat::Docx,
                &AtomicBool::new(false),
                &clock
            )
            .unwrap_err(),
            SupervisionError::ContainmentUnavailable
        );
    }

    #[test]
    fn silence_reaches_absolute_deadline_and_cannot_return_old_output() {
        let clock = TestClock::new();
        let mut worker = worker(&clock, WorkerPlatform::MacOsAppSandboxXpc);
        worker.events.clear();
        worker.advance_per_receive = WORKER_WALL_LIMIT;
        assert_eq!(
            supervise_with_clock(
                &mut worker,
                InputFormat::Docx,
                &AtomicBool::new(false),
                &clock
            )
            .unwrap_err(),
            SupervisionError::Transport(TransportError::TimedOut)
        );
        assert_eq!(worker.termination_calls, 1);
        assert_eq!(worker.cleanup_calls, 1);
    }

    #[test]
    fn cancellation_and_native_transport_failure_are_terminal_and_cleaned() {
        let clock = TestClock::new();
        let mut cancelled_worker = worker(&clock, WorkerPlatform::WindowsAppContainerJob);
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            supervise_with_clock(&mut cancelled_worker, InputFormat::Docx, &cancelled, &clock)
                .unwrap_err(),
            SupervisionError::Transport(TransportError::Cancelled)
        );
        assert_eq!(cancelled_worker.receives, 0);

        let mut failed_worker = worker(&clock, WorkerPlatform::WindowsAppContainerJob);
        failed_worker.events = [Err(NativeAdapterError::EventTransport)].into();
        assert_eq!(
            supervise_with_clock(
                &mut failed_worker,
                InputFormat::Docx,
                &AtomicBool::new(false),
                &clock
            )
            .unwrap_err(),
            SupervisionError::Native
        );
        assert_eq!(failed_worker.termination_calls, 1);
        assert_eq!(failed_worker.cleanup_calls, 1);
    }

    #[test]
    fn failed_exit_malformed_output_and_floods_never_escape_cleanup() {
        let cases = [
            [
                Ok(Some(NativeWorkerEvent::Stdout(WIRE.to_vec()))),
                Ok(Some(NativeWorkerEvent::Exited { code: Some(65) })),
            ]
            .into_iter()
            .collect(),
            [
                Ok(Some(NativeWorkerEvent::Stdout(b"not-json".to_vec()))),
                Ok(Some(NativeWorkerEvent::Exited { code: Some(0) })),
                Ok(Some(NativeWorkerEvent::StdoutEof)),
                Ok(Some(NativeWorkerEvent::StderrEof)),
            ]
            .into_iter()
            .collect(),
            [Ok(Some(NativeWorkerEvent::Stdout(vec![
                b'x';
                crate::import_transport::MAX_PIPE_CHUNK_BYTES
                    + 1
            ])))]
            .into_iter()
            .collect(),
            [Ok(Some(NativeWorkerEvent::Stderr(vec![
                b'x';
                crate::import_transport::MAX_PIPE_CHUNK_BYTES
                    + 1
            ])))]
            .into_iter()
            .collect(),
        ];
        for events in cases {
            let clock = TestClock::new();
            let mut worker = worker(&clock, WorkerPlatform::MacOsAppSandboxXpc);
            worker.events = events;
            assert!(
                supervise_with_clock(
                    &mut worker,
                    InputFormat::Docx,
                    &AtomicBool::new(false),
                    &clock
                )
                .is_err()
            );
            assert_eq!(worker.termination_calls, 1);
            assert_eq!(worker.cleanup_calls, 1);
        }
    }

    #[test]
    fn any_missing_cleanup_fact_or_termination_failure_overrides_valid_output() {
        for field in 0..7 {
            let clock = TestClock::new();
            let mut worker = worker(&clock, WorkerPlatform::WindowsAppContainerJob);
            let mut receipt = complete_cleanup();
            match field {
                0 => receipt.worker_reaped = ControlEvidence::Missing,
                1 => receipt.process_tree_empty = ControlEvidence::Missing,
                2 => receipt.stdout_closed = ControlEvidence::Missing,
                3 => receipt.stderr_closed = ControlEvidence::Missing,
                4 => receipt.input_closed = ControlEvidence::Missing,
                5 => receipt.output_closed = ControlEvidence::Missing,
                6 => receipt.containment_closed = ControlEvidence::Missing,
                _ => unreachable!(),
            }
            worker.cleanup_result = Ok(receipt);
            assert_eq!(
                supervise_with_clock(
                    &mut worker,
                    InputFormat::Docx,
                    &AtomicBool::new(false),
                    &clock
                )
                .unwrap_err(),
                SupervisionError::CleanupUnverified
            );
        }
        let clock = TestClock::new();
        let mut termination_worker = worker(&clock, WorkerPlatform::MacOsAppSandboxXpc);
        termination_worker.terminate_result = Err(NativeAdapterError::Termination);
        assert_eq!(
            supervise_with_clock(
                &mut termination_worker,
                InputFormat::Docx,
                &AtomicBool::new(false),
                &clock
            )
            .unwrap_err(),
            SupervisionError::CleanupUnverified
        );
        assert_eq!(termination_worker.cleanup_calls, 1);

        let clock = TestClock::new();
        let mut cleanup_worker = worker(&clock, WorkerPlatform::WindowsAppContainerJob);
        cleanup_worker.cleanup_result = Err(NativeAdapterError::Cleanup);
        assert_eq!(
            supervise_with_clock(
                &mut cleanup_worker,
                InputFormat::Docx,
                &AtomicBool::new(false),
                &clock
            )
            .unwrap_err(),
            SupervisionError::CleanupUnverified
        );

        let clock = TestClock::new();
        let mut replayed_worker = worker(&clock, WorkerPlatform::WindowsAppContainerJob);
        replayed_worker
            .cleanup_result
            .as_mut()
            .unwrap()
            .operation_nonce = [0x33; OPERATION_NONCE_BYTES];
        assert_eq!(
            supervise_with_clock(
                &mut replayed_worker,
                InputFormat::Docx,
                &AtomicBool::new(false),
                &clock
            )
            .unwrap_err(),
            SupervisionError::CleanupUnverified
        );

        let clock = TestClock::new();
        let mut late_worker = worker(&clock, WorkerPlatform::MacOsAppSandboxXpc);
        late_worker.advance_during_cleanup = CLEANUP_LIMIT + Duration::from_nanos(1);
        assert_eq!(
            supervise_with_clock(
                &mut late_worker,
                InputFormat::Docx,
                &AtomicBool::new(false),
                &clock
            )
            .unwrap_err(),
            SupervisionError::CleanupUnverified
        );
    }

    #[test]
    fn event_and_receipt_debug_output_never_discloses_worker_content() {
        let event = NativeWorkerEvent::Stdout(b"PRIVATE_PATH_AND_CONTENT".to_vec());
        let debug = format!("{event:?}");
        assert!(debug.contains("24"));
        assert!(!debug.contains("PRIVATE"));
        let receipt = launch(WorkerPlatform::MacOsAppSandboxXpc);
        assert!(!format!("{receipt:?}").contains("executable"));
    }
}
