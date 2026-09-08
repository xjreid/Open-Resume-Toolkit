# Step 5 Rust native pipe driver

Date: 2026-09-07. Uncommitted implementation on M1-complete `65518eb`.
macOS arm64 local development; Step 6 qualification has not begun.

## Implementation

`ort-platform::MacosWorkerOutput` now provides a safe Rust pipe driver whose
events directly match `ort-documents::worker_supervisor::NativeWorkerEvent`.
It owns two supplied pipe read ends, rejects files/sockets/write ends, establishes
nonblocking and close-on-exec flags, and closes both handles on initialization
failure, terminal read failure or explicit close. Rust ownership also releases
the handles when the driver is dropped. It accepts no path or PID.

All byte and timing ceilings come from the common document supervisor/transport:
512 KiB stdout, 16 KiB stderr, at most 8 KiB per read and at most a requested
25 ms poll. The caller can request a shorter poll, including zero. This bounds
the requested OS wait, not scheduler latency. Reads alternate ready streams and
return control after interruption or a readiness race, without retry loops.
An extra byte beyond either exact limit causes sticky failure. Buffered content
precedes each stream's single EOF event.

Stderr content is cleared before constructing an event. Its bounded zero-filled
vector carries only the byte count, fitting the existing supervisor event API
without changing its validation contract. Stdout allocation uses a fallible
reservation after the fixed stack read and limit check. Debug formatting remains
the shared content-free event formatter. Clearing the local buffer is not a
claim of secure erasure of all memory copies.

The earlier C component remains an independent, unlinked prototype. The Rust
platform implementation uses reviewed safe dependency APIs and retains the
workspace's `unsafe_code = "forbid"`; it introduces no additional FFI exception.

## Dependency record

- `rustix = 1.1.4` was already locked through the capability filesystem stack.
  Direct macOS-only `event` and `fs` features supply safe `poll`, `read`, `fstat`
  and `fcntl` APIs that the standard library does not provide together. No
  third-party package version was added or changed.
- Upstream: [Bytecode Alliance rustix](https://github.com/bytecodealliance/rustix).
  License: `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`, included in the
  dependency inventory. It wraps native syscalls; no runtime network service,
  downloaded helper, telemetry or credential access is added.
- The pinned local implementation was inspected for descriptor lifetimes,
  polling, flags and reads. This is a narrow usage review, not a new audit of
  the dependency's entire security history. Updates stay subject to repository
  dependency gates.
- The macOS platform module now depends on the existing first-party
  `ort-documents` policy types/constants so limits cannot drift from the
  supervisor. The removal boundary is this one module and its target-specific
  dependency declarations. Neither dependency invokes a parser here.

## Verification and scope

Five real-pipe Rust regressions passed: fair/redacted events and drained EOF,
silent-wait clamping and terminal close, exact-limit/one-byte-overflow behavior
for both streams, oversized burst splitting, and invalid-input/drop ownership
while unrelated handles remain usable. Strict workspace Clippy and the full
workspace Rust tests passed. The full `CI=true pnpm check` also passed with
102 desktop tests and the 728 Rust / 167 JavaScript license inventory.

Logs: `target/m2-native-pipe-clippy.log`,
`target/m2-native-pipe-workspace-tests.log`, and
`target/m2-native-pipe-web-check.log`.

This is Step 5 implementation with focused regression evidence. It does not
produce launch/cleanup receipts, report process exit, launch XPC services, enforce
memory/CPU bounds, handle parent death, or prove whole-tree cleanup. The driver
is callable by a future native adapter but no production call site invokes it.
Import stays disabled. The [hard-limit findings](m2-native-output-and-resources.md)
remain unresolved and cannot be waived by these pipe tests. The tested staged
editor candidate predates this source change and was not rebuilt or replaced.
