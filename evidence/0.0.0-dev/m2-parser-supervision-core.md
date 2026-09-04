# M2 production parser-worker supervision-core checkpoint

Date: 2026-09-04. Base commit: `bc0712f`. Implementation changes are
uncommitted. Local verification platform: macOS arm64. All worker events and
extraction data used by tests are synthetic.

## Implemented bounded slice

`ort-documents::worker_supervisor` is the production cross-platform coordinator
between future native containment adapters and the existing bounded extraction
transport. It does not launch an ordinary child as a sandbox substitute and does
not accept a generic `sandboxed` boolean. Before reading an event it requires one
complete parent-observed profile:

- macOS: fixed verified executable, minimal environment, exact descriptors,
  read-only input, private output, denied network/child/general-file/credential/
  IPC authority, verified helper signature, effective App Sandbox and sandbox
  inheritance, and no shared app/Keychain entitlements; or
- Windows: the same common boundary plus an effective zero-capability
  AppContainer token and a kill-on-close/no-breakaway Job Object bound before
  parser execution.

Both profiles require no more than 512 MiB memory, 30 seconds CPU, 64 open
handles, zero core output and the exact existing 60-second monotonic wall limit.
Stricter nonzero memory/CPU/handle ceilings are accepted; weaker or absent limits
are rejected.

The coordinator polls cancellation and the absolute deadline at most 25 ms
apart, independently of output. It feeds only owned native events through the
existing 8 KiB-chunk/512 KiB-stdout/16 KiB-discarded-stderr transport. On every
result—including invalid launch evidence, success, failed exit, malformed
output, cancellation, timeout or adapter error—it requests whole-containment
termination and performs bounded cleanup. It withholds otherwise valid output
unless the trusted adapter observes worker reaping, an empty tree, both pipe
closures, input/output-handle closure and containment teardown. Termination or
cleanup uncertainty wins over a valid extraction. A nonzero parent-owned
128-bit operation nonce must match the cleanup receipt, preventing a stale
receipt from a previous worker from settling a later operation.

The adapter errors, receipts and native-event `Debug` implementation contain no
paths, document text, stderr text or parser diagnostics. No new dependency,
unsafe Rust, command, UI route, capability, entitlement or package permission was
added. ADR 0007 records this boundary.

## Adversarial automated evidence

The `ort-documents` test suite uses deterministic mock clocks and mock native
adapters for both platform profiles. It verifies:

- valid macOS and Windows event sequences release extraction only after one
  termination and one complete cleanup;
- removing every required common or platform-specific launch control rejects
  before event receipt while still terminating and cleaning up;
- zero or over-limit memory/CPU/handle values, extended wall time and nonzero
  core output reject;
- immediate cancellation, silent absolute timeout and native event failure are
  terminal and cleaned;
- nonzero exit, malformed JSON, stdout flood and stderr flood cannot bypass
  cleanup;
- every individually missing cleanup observation, cleanup failure and
  termination failure overrides valid worker bytes;
- zero launch nonces and mismatched/replayed cleanup nonces reject; and
- event/receipt debug formatting does not disclose a seeded private marker.

Local targeted `ort-documents` tests and strict Clippy pass. `just check` then
passed after all code and documentation changes: Prettier, TypeScript lint,
103 JavaScript tests, frontend/extension production builds, static web/secret
checks, workspace Rustfmt and Clippy with warnings denied, and 146 Rust tests
across all targets (one explicitly gated OS-vault test remained ignored). The
existing inert-worker regression returned exit 78 with empty stdout. Contract
regeneration completed with no generated-file drift.

Both existing native macOS probes were rerun outside the enclosing agent sandbox
so their positive controls remained meaningful. The App Sandbox/hard-limit probe
passed its documented subset and emitted
`target/native-probes/macos-document-XfNPzA/report.json`; the nine-case XPC
direct-child lifecycle probe passed and emitted
`target/native-probes/macos-lifecycle-Vgj4B0/report.json`. Both content-free
reports explicitly retain `fullContainmentProven=false` and `importEnabled=false`.
They validate the existing synthetic candidates, not the new production adapter
contract. Cross-compilation or Windows execution is not claimed: Windows native
verification remains a hosted/native gate.

## Import remains disabled

This coordinator is production lifecycle policy, not production native
containment. `ort_documents::IMPORT_ENABLED` remains `false`; the desktop exposes
no import command or capability; `ort-document-worker` remains inert; no PDF/DOCX
parser or hostile file is opened. Launch/cleanup receipts come only from a future
trusted adapter and are orchestration preconditions, not proof that an OS
enforced them.

No milestone or release gate is closed by this checkpoint. It removes duplicated
cross-platform lifecycle policy from the future adapters and makes missing
native evidence fail closed.

## Remaining native verification gates

### Common to macOS and Windows

- Implement the real nonblocking/cancellable pipe driver and prove that every
  wait returns within the supervisor poll bound even under silence, partial
  writes, pipe closure races and output floods. Prove the five-second cleanup
  bound under injected launch, I/O, termination, wait and teardown failures.
- Verify the fixed bundled worker and every runtime dependency against release
  signatures/hashes and immutable parent directories; reject symlink/reparse,
  writable search paths, PATH lookup and executable replacement races.
- Inventory every inherited/received handle, descriptor, environment value,
  dynamic-library search location, Mach port/Windows object and broker surface.
  Prove no profile, database, vault, browser/native IPC, diagnostics, updater or
  provider authority is reachable.
- With accessible positive controls, deny seeded sibling/profile reads and
  writes, path/symlink/reparse redirection, loopback and external TCP, UDP, DNS,
  credential access, native IPC and broker-mediated process creation. Missing
  targets or unreachable listeners are not denial evidence.
- Exercise hard memory, aggregate CPU, handle/descriptor, thread and platform
  object limits; decompression/nesting/object/image/parser limits; cancellation
  before/during/after output; crashes; malformed/partial/multiple messages;
  stdout/stderr floods; stalled children; and cleanup failures.
- Prove parent/desktop/supervisor death removes all runnable descendants and
  source/result authority. Scan for survivors and post-termination resource
  accounting instead of accepting a sent kill. Prove no bytes, handles, result
  files or process state cross into a later import.
- Integrate private input staging and cleanup without granting the worker a user
  path. Then add pinned parsers and hostile PDF/DOCX/fuzz corpora. Prove every
  failure leaves canonical/profile data unchanged and No-AI review preserves
  every extracted block.
- Repeat the full matrix for every advertised OS version, architecture, install
  channel, signing identity and update/repair transition. Retain content-free
  reports containing OS/build versions, binary hashes, effective policy,
  resource accounting and cleanup outcomes.

### macOS-specific

- Implement and release-sign the separately embedded minimal XPC supervisor and
  sandbox-inheriting fixed child without desktop app groups, user-selected-file,
  network or Keychain entitlements. Verify actual entitlements and container
  behavior after install, move, update and notarization.
- Prove hard limits are established before parser code/input processing; cover
  memory, CPU, threads and Mach ports beyond the current NPROC/NOFILE/CORE probe.
- Prove broker/LaunchServices/XPC/AppleEvent/credential denial and full process-
  tree cleanup on client, desktop and XPC-supervisor crash/death. XPC invalidation
  or direct-child `waitpid` alone is insufficient.
- Re-run on every supported Intel/Apple-silicon and macOS target. Current native
  evidence is local/ad-hoc/synthetic and covers only the documented subset.

### Windows-specific

- Implement process creation with a verified zero-capability AppContainer token,
  explicit handle list, child-process policy and Job list/suspended assignment so
  no parser instruction runs before containment. Verify the effective token,
  integrity level, capabilities, default/broad package ACL effects and private
  staging ACLs.
- Enforce job-wide kill-on-close, active-process limit one, memory and CPU limits,
  no breakaway and parent-death cleanup. Verify post-termination Job accounting,
  handle ceilings and that COM/WMI/broker/service paths cannot create an escaping
  descendant.
- Use bounded overlapped/cancellable I/O and verify pipe handle inheritance and
  close races. Cover reparse points, alternate streams, device/UNC paths,
  Credential Manager, named pipes, ALPC/RPC and loopback/external network.
- Run the complete native suite on every supported Windows architecture/version
  and both direct-package/MSIX identities. There is currently no Windows native
  hostile-document containment evidence.

Until all applicable items pass, parser integration, import UI and release
advertising remain blocked.
