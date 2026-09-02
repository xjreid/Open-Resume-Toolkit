# Document worker containment implementation gate

Status: transport policy plus partial native macOS probe, 2026-09-02. **Not a
production sandbox or permission to enable import.** The parser worker still exits 78.
This refines M2; it does not introduce an additional milestone.

## Implemented parent transport policy

`ort-documents::import_transport` accepts events from a future trusted native
adapter, not from the renderer. Read chunks are at most 8 KiB. Stdout has a
512 KiB total ceiling checked before copying/allocation; stderr has a 16 KiB
total ceiling and is discarded, never logged as parser-provided text. A single
JSON extraction is decoded only after both pipes reach EOF and the OS reports
exit zero. Exit may arrive before buffered pipe data. Failed/signal exits,
duplicate events, trailing JSON, incomplete output, backwards timestamps,
cancellation, and the 60-second absolute wall deadline reject the result.

The first transport failure is terminal and releases its buffer. Release is
not a guarantee that all allocator/OS copies of text have been securely erased.
No transcript is persisted, sent to telemetry, or included in Debug output.

This policy cannot kill a process, wake a blocked read, restrict memory/CPU,
validate source-file type, or verify cleanup. The native adapter must use
bounded nonblocking/cancellable reads, drain both pipes, poll cancellation and
the monotonic deadline independently of output, and terminate/reap the whole
contained job on **every** completion or failure. `finish` is a data validator,
not proof of containment: discard the result if cleanup fails. Never use an
unbounded output collector or allocate according to a worker-provided length.

## Platform candidates and unresolved proof

### Windows

Investigate a capability-free AppContainer, explicit handle allowlist, child
process restriction, and a Job Object. Microsoft documents AppContainer as a
boundary for credential, file, network and process isolation; it does not mean
all filesystem reads are denied. Check the actual effective token and ACLs,
including resources granted to broad application-package groups.
[AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation)

Create the process suspended or assign its job at creation. Establish the
security capabilities, inherited read-only input/output handles, child-process
policy and all limits before any parser code executes. Do not inherit parent
environment secrets, arbitrary handles, profile roots, or shell command lines.
Use an absolute verified bundled executable, never a PATH search. The public
creation attributes support capability, handle-list, child-process and job-list
configuration; verify compatibility on our supported Windows versions.
[Process creation attributes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)

Use job-wide termination, kill-on-close, active-process limit one, memory and
CPU ceilings, and no breakaway. A Job Object alone is not a filesystem/network
sandbox and not every process-creation route automatically joins it. Confirm
no broker/WMI/COM route can escape the token/child-process restriction. Check
post-termination accounting instead of treating a sent kill request as proof.
[Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)

Unresolved: exact handle-count enforcement, mandatory ACL/private staging
behavior, parent-death cleanup, and denied broker/credential/IPC paths. An
unsupported security primitive must disable imports, not silently weaken limits.

### macOS

Investigate a separately signed embedded XPC service with its own minimal App
Sandbox entitlements and descriptor-only input transfer. Apple documents XPC
as a privilege-separation mechanism, supports file-descriptor transfer, and
requires signing to attach sandbox entitlements. Ordinary `posix_spawn` or
`NSTask` does not independently provide that separation. An XPC connection is
not itself a sandbox, and signing alone is not proof that the intended policy
was applied.
[Creating XPC Services](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingXPCServices.html)

Do not share the desktop's app group, Keychain access group, network privileges,
user-selected-file entitlement, profile storage or general application IPC.
Give the helper only required descriptors plus unavoidable verified runtime
resources. Determine the exact filesystem and broker access left by App Sandbox.

Unresolved: a supported way to deny subprocess creation, enforce the resource
ceilings, guarantee a fresh process for each operation, and forcibly end the
whole job on cancellation/parent death. XPC invalidation/idle termination must
not be assumed to kill a currently compromised or busy service. No deprecated
custom sandbox profile is accepted as the shipping solution without a separate
support/security decision. Keep import unavailable until this proof exists.

Any native FFI must live behind a narrowly reviewed adapter with ownership and
error-path tests. Do not relax workspace-wide `unsafe_code = "forbid"` merely
to make a prototype compile. A proposed isolated binding/bridge must explicitly
document its safety obligations before integration.

## Implemented macOS probe: partial evidence, not a passed gate

`just probe-document-sandbox-macos` builds a separate ad-hoc hardened-runtime
test app and embedded XPC helper from `tools/native/macos-document-probe`.
The helper has only the App Sandbox entitlement; neither executable is packaged
in ORT. The runner verifies signatures and the helper checks its entitlement.
Only freshly generated synthetic fixtures and a parent-owned IPv4 loopback
listener are used. No personal document, profile or credential is opened.

Local macOS 26.6.2 arm64 results:

- transferred read-only descriptor: exact marker read succeeded; writing failed;
- seeded sibling read/write-open and symlink-follow: denied;
- connection to the parent's local TCP listener: denied;
- direct `/usr/bin/true` child creation: **allowed**, contrary to the no-child gate;
- cooperative helper exit: XPC disconnect observed, not forced-kill proof.

The same probes run in the unsandboxed parent first to prove the targets are
accessible. Only permission errors count as denials; missing targets and other
OS errors reject evidence. A child being created does not establish that it
escaped the sandbox, but this candidate still fails the specified prohibition.
Do not enable parsing or weaken that requirement based on these results.

The runner saves a bounded report and helper/host hashes under
`target/native-probes`. A retained measurement and limitations are recorded in
`../../evidence/0.0.0-dev/m2-native-sandbox-probe.md`. Both macOS CI jobs now run
this subset check. A green probe job asserts only descriptor, seeded filesystem,
loopback and cooperative-disconnect checks; it never asserts full containment.
Child creation remains an explicit reported failure of the larger design gate.

## Remaining executable proof

Build synthetic probe helpers, not PDF/DOCX parsers. Use only temporary seeded
files and disposable test credentials; never probe a real browser profile,
user document, development database or personal credential. Start with one
platform adapter, keep the other explicitly unsupported, and enable no importer
until both advertised platforms meet the gate.

For each platform/architecture, record OS version, helper hash, signing and
effective policy, allow/deny outcomes, and supervisor cleanup outcome:

- Positive control: read only the preopened synthetic input and emit bounded
  output. Required runtime dependencies load without a writable search path.
- Deny seeded sibling/profile-file reads and writes, path/symlink/reparse
  redirection, network and loopback/UDP/DNS, credential access and native IPC.
  A missing target is not denial evidence: verify the parent can access each
  disposable test target first.
- Deny child launch and broker-mediated launch; detect surviving processes.
- Exercise memory/CPU/handle exhaustion, idle and output-flood timeouts,
  cancellation before/during/after output, crashes, launch failures, parent
  death, malformed messages and cleanup failures.
- Reject valid output after a failed/signal exit or cleanup failure. No pending
  result or previous worker response may be reused after cancellation/restart.
- Verify no source bytes, handles, result files or runnable processes leak into
  a later operation; no profile mutation occurs on any failed import.

The Rust transport tests remain deterministic event simulations. The separate
macOS probe adds only the native subset above. Forced process-tree termination,
parent-death handling, resource ceilings, credentials/brokers, broader filesystem
and network denial, hostile-code cleanup, release signing and supported OS/CPU
matrices remain unproven. Windows has no native containment evidence yet.
Production input staging, real pipe drivers, sandbox adapters, PDFium/DOCX
parsers and the import-review UI remain unimplemented.
