# ADR 0007: Fail-closed document-worker supervision boundary

- Status: production coordinator accepted; native containment adapters gated
- Target milestone: M2

## Decision

Use one Rust-owned `ort-documents::worker_supervisor` coordinator for both
supported platforms. A reviewed native adapter may enter the coordinator only
after it has established and measured the complete platform launch boundary:

- a separately signed, minimal App Sandbox XPC supervisor and inherited fixed
  child on macOS; or
- a zero-capability AppContainer process bound to a no-breakaway, kill-on-close
  Job Object before execution on Windows.

Both adapters must use a verified fixed executable, minimal environment, exact
descriptor/handle allowlist, read-only staged input, private output, denied
network/child/general-file/credential/IPC authority, and the common resource
ceilings. The coordinator polls cancellation and a monotonic absolute deadline,
passes bounded native events through the existing extraction transport, and
requests whole-containment termination on every path. A valid worker result is
released only after parent-observed reaping, empty-tree, pipe/handle closure,
and containment teardown evidence all succeed. A nonzero parent-owned operation
nonce binds cleanup to the exact launch and rejects a stale/replayed receipt.

Launch and cleanup receipts are explicit orchestration preconditions. They are
not self-attestation by an untrusted worker and do not prove that an operating
system primitive was configured correctly. Platform-native hostile tests and
release-package evidence remain the authority for that proof.

## Consequences

The common success/failure lifecycle no longer needs to be independently
reimplemented by the macOS and Windows adapters. Cancellation, silence,
oversized output, malformed output, failed exit, native transport errors,
termination failures, and incomplete cleanup all fail without an extraction.
Diagnostic errors and `Debug` output contain no document or parser text.

The native XPC/App Sandbox and AppContainer/Job implementations, real pipe
drivers, parser libraries, staging, UI, and supported-platform verification are
still absent. `IMPORT_ENABLED` remains `false`, and `ort-document-worker`
continues to exit 78. This ADR therefore does not authorize document import.
