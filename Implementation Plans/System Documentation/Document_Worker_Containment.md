# Document worker containment implementation gate

Status: source-envelope preflight, Unix private staging, transport policy, common
production supervision coordinator, and macOS sandbox/hard-limit and lifecycle
probes, 2026-09-04.
**Not a production native
sandbox or permission to enable import.** The parser worker still exits 78.
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

This transport policy alone cannot kill a process, wake a blocked read, restrict
memory/CPU, validate source-file type, or verify cleanup. The native adapter must use
bounded nonblocking/cancellable reads, drain both pipes, poll cancellation and
the monotonic deadline independently of output, and terminate/reap the whole
contained job on **every** completion or failure. `finish` is a data validator,
not proof of containment: discard the result if cleanup fails. Never use an
unbounded output collector or allocate according to a worker-provided length.

## Implemented parent source envelope

`ort-platform::read_native_document` acquires a dialog-selected PDF/DOCX once
through a held parent-directory capability, refuses final-component symlinks and
non-regular files, and returns a parent-owned snapshot bounded to 10 MiB.
`ort-documents::import_source` checks that same snapshot's selected-format magic
and bounded outer structure. DOCX inspection parses no XML and decompresses no
entry; it rejects malformed/multidisk/ZIP64 packages, unsafe or duplicate names,
unsupported flags/methods, encryption, known active parts, missing minimum OPC
parts and declared expansion beyond 100:1. PDF inspection is limited to supported
header versions and terminal EOF. See ADR 0008 and
`../../evidence/0.0.0-dev/m2-import-source-envelope.md`.

The source-envelope layer is deliberately not a content parser. Structurally
valid input remains hostile. The user path is not reread after the snapshot;
the staging subset below owns the Unix read-only handle, while Windows must still
implement and prove equivalent private ACL/reparse behavior before integration.

## Implemented private staging subset

`ort-platform::ImportStagingRoot` now creates operation-owned UUIDv7 stages under
the fixed application-data `imports` root. Unix stages require `0700`
directories, two exact fixed regular files, a bounded ownership marker, a `0600`
source and one transferred read-only handle. Explicit cleanup removes only that
inventory. A 128-entry startup scavenger removes only exact stages older than 24
hours and preserves unknown, fresh, symlinked, malformed or additional content.

`ort-application::document_import` binds envelope inspection to staging of the
same bytes and defines the future launch → supervision → adapter destruction →
exact stage cleanup ordering. Cleanup failure withholds output. The public path
still checks `IMPORT_ENABLED=false` and cleans without invoking a launcher.
`ort-documents::worker_output` symmetrically bounds parser-produced pages,
blocks, characters, controls and JSON before the parent decoder sees it. See ADR
0009 and `../../evidence/0.0.0-dev/m2-import-private-staging.md`.

Windows staging remains unavailable until its private ACL/reparse implementation
and native evidence exist. No desktop call site creates stages. Constrained DOCX
and pinned PDFium text parsers are now isolated in the worker crate, but remain
unreachable from the inert executable. PDFium packaging remains absent.

## Implemented constrained DOCX parser, still unreachable

`ort-document-worker::extract_docx` accepts only an already-open reader and
returns extraction wire v1. It independently rechecks the bounded DOCX envelope,
ZIP local/central metadata, CRC/data descriptors, the non-macro main content
type and fixed root document relationship. It inflates only fixed metadata,
document and optional relationship parts. Streaming XML limits depth, events,
relationships, blocks and characters; active elements/relationship types,
DTD/PI/CDATA, unknown entities, unsafe targets and external non-hyperlink
relationships fail closed. Targets are never resolved or fetched.

The parser preserves source-order paragraphs, built-in headings, list hints,
Unicode, breaks and tabs while ignoring deleted text. It deliberately reports
one logical DOCX page because layout pagination is not trustworthy here. Empty
or image-only input returns the existing OCR-unavailable result. Stored and
deflated synthetic packages, adversarial structures and the shipping DOCX
export shape are covered. See ADR 0010 and
`../../evidence/0.0.0-dev/m2-docx-worker-parser.md`.

This parser is not containment evidence. The executable continues to exit 78,
and no application crate depends on the worker. Native adapters, resource and
lifecycle qualification, real/fuzz corpora and both-platform packaged tests must
pass before it can be invoked.

## Implemented pinned PDFium text adapter, still unreachable

`ort-document-worker::extract_pdf` independently rechecks the bounded PDF
envelope and parses only in-memory bytes through `pdfium-render` 0.9.3's explicit
`pdfium_7881` API. It accepts no password and does not render, execute script,
traverse attachments/forms, fetch a URI or perform OCR. The target library must
match the exact filename, size and extracted-library SHA-256 recorded for the
immutable non-V8/non-XFA PDFium 151.0.7881.0 macOS ARM64/x64 or Windows ARM64/x64
artifact; system-library fallback is forbidden.

The adapter caps pages at 10, top-level page objects at 20,000 per page and
extracted text at the shared 50,000-character limit. Literal line content is
returned in PDF definition/page order with only line-ending normalization and
conservative known-heading/list hints. Image-bearing pages below the documented
16-character threshold fail as partially scanned, while completely unreadable
input fails as OCR-unavailable. Pure/adversarial tests and one pinned macOS ARM64
native synthetic smoke pass. See ADR 0011 and
`../../evidence/0.0.0-dev/m2-pdf-worker-parser.md`.

This parser is also not containment evidence. Packaging, attestation/license
verification, native invocation and macOS x64/Windows native parser runs remain
gated; the executable still exits 78.

## Implemented common production supervision coordinator

`ort-documents::worker_supervisor` now owns the cross-platform lifecycle above
the native adapters. It accepts only one of two exact launch profiles: a macOS
App Sandbox/XPC supervisor with an inherited fixed child, or a Windows
zero-capability AppContainer child bound to a kill-on-close/no-breakaway Job
Object before execution. Both profiles enumerate common executable, environment,
handle, input/output, filesystem, network, child-process, credential and IPC
controls rather than accepting a generic `sandboxed` flag. Resource receipts may
be stricter than the 512 MiB memory, 30-second CPU and 64-handle ceilings, but
cannot be weaker; wall time is the existing absolute 60 seconds and core output
must be disabled.

The coordinator polls a parent cancellation token and the monotonic deadline at
25 ms intervals independently of pipe output, translates only owned bounded
native events into the existing transport, and always requests termination of
the complete containment object. It then grants a five-second cleanup budget and
requires parent-observed worker reaping, an empty process tree, both pipe closures,
input/output-handle closure and containment teardown. Any missing launch control,
native event failure, termination failure or missing cleanup fact withholds even
otherwise valid extraction bytes. A nonzero parent-owned operation nonce binds
cleanup to the exact launch and rejects stale/replayed cleanup evidence. Adapter
and debug errors are content-free.

This is production orchestration code but not a native implementation. A launch
receipt is a trusted-adapter precondition, not an untrusted worker claim or OS
proof. No macOS XPC or Windows AppContainer/Job adapter implements the trait yet;
therefore no product call site can launch a parser. See ADR 0007 and
`../../evidence/0.0.0-dev/m2-parser-supervision-core.md`.

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

Local direct `fork`/`posix_spawn` denial now has a hard-limit candidate (below).
Unresolved: broker-mediated launches and executable replacement, remaining resource
ceilings, guaranteeing a fresh process for each operation, and forcibly ending the
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

Plain App Sandbox baseline on local macOS 26.6.2 arm64:

- transferred read-only descriptor: exact marker read succeeded; writing failed;
- seeded sibling read/write-open and symlink-follow: denied;
- connection to the parent's local TCP listener: denied;
- direct `/usr/bin/true` child creation: **allowed**, contrary to the no-child gate;
- cooperative helper exit: XPC disconnect observed, not forced-kill proof.

The same probes run in the unsandboxed parent first to prove the targets are
accessible. Filesystem/network denials require permission errors; missing targets
and other OS errors reject evidence. A child being created does not establish that it
escaped the sandbox, but this candidate still fails the specified prohibition.
Do not enable parsing or weaken that requirement based on these results.

### Helper-only hard-limit extension

After retaining the baseline, the same synthetic helper sets both soft and hard
`RLIMIT_NPROC` to zero, `RLIMIT_NOFILE` to 64 and `RLIMIT_CORE` to zero. The root
user is explicitly refused. Only this helper calls `setrlimit`; no shell,
desktop, launchd, account-wide or system settings change. The parent's original
limits are compared afterwards and it must still be able to spawn a child.

Two local runs passed direct `posix_spawn` and `fork` denial after those limits.
The same helper must successfully create/reap both baseline children first.
For these child tests only, `EAGAIN` counts as denial if the non-root helper's
zero soft/hard NPROC limit is verified; `EAGAIN` alone is never sufficient.
Opening paths and network sockets retains the stricter permission-error rule.
Attempts to raise each hard limit must fail with `EPERM` without changing it.

Descriptor exhaustion duplicates the preopened synthetic input at most 65 times,
expects `EMFILE`, closes only those duplicates, and verifies another duplication
succeeds afterwards. This is a small process-local test, not machine-wide resource
exhaustion. The zero core-file limit is read back; it does not prove the absence
of every crash/diagnostic artifact. Mach ports, threads, memory and CPU are not
bounded by the descriptor test.
Lowering NOFILE does not revoke existing descriptors. Production must separately
verify its inherited/received descriptor allowlist and reject unexpected handles;
this duplication test is not an audit of every inherited or broker-supplied handle.

Apple documents the per-process hard-limit API and privileged-only increases in
[setrlimit](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setrlimit.2.html).
Its published [XNU process-creation source](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_fork.c)
checks the non-root caller's NPROC limit for direct creation. This supports the
candidate, not a claim that it covers every broker or advertised OS version.
The synthetic baseline intentionally runs before tightening limits; a production
adapter must establish all boundaries before parser code/input processing and
must never expose such an unrestricted phase to untrusted content.

The runner saves a version-2 report and helper/host hashes under
`target/native-probes` and prints the content-free metadata in CI logs. The old
baseline record remains in `../../evidence/0.0.0-dev/m2-native-sandbox-probe.md`;
new evidence is in `../../evidence/0.0.0-dev/m2-macos-hard-limits.md`.
Both macOS CI jobs now require the descriptor, seeded filesystem, loopback,
direct spawn/fork denial, hard-limit raise denial, descriptor-exhaustion/recovery,
parent-unaffected and cooperative-disconnect checks. A green step still does
not assert full containment. The user reported all four CI jobs passing after
`723a97f`; a successful-run log was not independently retrieved.

### Trusted supervisor and direct-child lifecycle experiment

`just probe-document-lifecycle-macos` builds a **separate test candidate** with
three executables: an unsandboxed synthetic client, a minimal App Sandbox XPC
supervisor, and a fixed child inheriting the supervisor's sandbox. It does not
inherit the desktop's broader authority. The child lowers its own hard limits
before reading the synthetic input; no parser is linked into either process.
The supervisor remains trusted and must be included in future security review.

This addresses direct-child PID ownership: only the trusted supervisor calls
`posix_spawn`, `kill` and `waitpid` for that child. One thread owns signaling and
reaping, with default SIGCHLD handling; it never signals after reaping or after
unexpected loss of wait ownership. No XPC PID, process-group signal, private
audit-token API or arbitrary requested executable is used. The runner verifies
the nested signatures; a production executable-identity policy is still required.
Apple documents [sandbox inheritance](https://developer.apple.com/library/archive/documentation/Miscellaneous/Reference/EntitlementKeyReference/Chapters/EnablingAppSandbox.html)
and [direct-child wait status](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/waitpid.2.html).

Nine local cases measure normal completion, explicit cancellation, silent timeout,
stdout/stderr floods, nonzero exit with valid bytes, malformed output, full output
without exit, and both EOFs without exit. Cancellation/timeout/flood cases require
observed SIGKILL exit status, reaping and both EOFs. A sent signal or XPC disconnect
alone is insufficient. Complete data cannot be accepted until successful OS exit.

The synthetic supervisor uses 1 KiB fair nonblocking read chunks, 4 KiB ceilings
per stream, a 64-byte retained stdout prefix, discarded stderr and a one-second
monotonic operation deadline. Counters saturate at 4097 as an over-limit marker;
they are not claimed as total bytes emitted by a flooded child. These deliberately
small test limits do not replace the production transport policy. A four-second
cleanup observation budget fails closed; the test child's 12-second alarm is only
a fallback, never accepted as forced-stop evidence. The child ignores SIGTERM.

The supervisor explicitly inherits only stdin/stdout/stderr into its child using
spawn file actions and `POSIX_SPAWN_CLOEXEC_DEFAULT`; it passes no inherited
environment secrets. Only a validated read-only synthetic file is passed as input.
No worker-provided output becomes a command, path, PID or profile mutation.

Evidence: `../../evidence/0.0.0-dev/m2-macos-lifecycle.md`. The user subsequently
reported all four CI jobs passing for `e978cfe`, including both macOS probes;
the run was not independently retrieved. The result is **direct-child
supervision evidence**, not proof of full process-tree containment, cleanup after
supervisor/client death, or the inherited child's complete filesystem, credential,
network, broker and IPC boundary. Those gates remain mandatory before integration.

## Remaining executable proof

Keep native containment probes synthetic and do not link the production parser
into them. Use only temporary seeded files and disposable test credentials;
never probe a real browser profile,
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
macOS probes add the native subsets above, including direct-child kill/reap.
Forced process-tree termination,
parent-death handling, memory/CPU/thread/Mach-port ceilings, credentials/brokers, broader filesystem
and network denial, hostile-code cleanup, release signing and supported OS/CPU
matrices remain unproven. Windows has no native containment evidence yet.
Windows private worker staging, native macOS/Windows pipe and containment adapters,
PDFium parsing, production DOCX invocation and the import-review UI remain
unimplemented. The common
coordinator and its adapter contract are implemented, but cannot supply or
validate the missing OS proof by themselves.

The subsequent M2 DOCX **export** checkpoint is an output-only fixed OPC/XML
generator over validated saved records. It neither opens supplied DOCX files
nor links an input parser into the desktop. `IMPORT_ENABLED=false` and the
worker exit-78 gate remain unchanged. Export evidence does not satisfy any
missing containment proof.
