# M2 native macOS termination checkpoint

Date: 2026-09-06. Platform: macOS arm64, Darwin 25.6.0. Source baseline:
M1-complete `65518eb`; this Step 5 implementation is uncommitted. The qualified
installed M1 app and both user profiles were untouched.

## Implemented behavior

The first-party `ort-macos-lifecycle` crate connects the AppKit delegate's
termination decision to the existing native-owned close attempt. Requests defer
through `NSTerminateLater`; cancellation explicitly replies false to AppKit.
Only the main window's valid, single-use decision can approve. Tauri dispatches
the final decision to the main thread. The file/render operation gate now uses
an atomic quit seal so a new operation cannot start between the busy check and
termination approval. A stale decision releases only its own attempted seal;
a replay cannot release an approved seal. Cancellation remains available while
an operation is running.

The bridge only adds an absent delegate method. It refuses an existing method,
never replaces the delegate or an implementation, and fails startup before
storage opens if installation is unavailable. The narrow unsafe exception and
dependency record are in
[`ort-macos-lifecycle/README.md`](../../crates/ort-macos-lifecycle/README.md).
Workspace-wide unsafe Rust prohibition and frontend permissions are unchanged.

## Observed evidence

- `node tools/check-termination-macos.mjs` passed. The synthetic AppKit process
  verified deferral, repeated-request coalescing, explicit cancellation, callback
  failure cancellation, denial of worker-thread and unsolicited/repeated
  replies, rejection of another delegate instance and an existing termination
  implementation, and actual termination after explicit approval. Both native
  sources passed Clang static analysis with zero findings. The content-free
  source-hashed report is under
  `target/native-probes/termination-4BOuZ2/report.json`.
- A separate blank-webview Tauri example was built and opened as
  `com.openresumetoolkit.synthetic-termination-probe`. Its native predefined
  menu action invokes AppKit `terminate:`. The first request reached the Rust
  callback, scheduled a response from a worker thread through Tauri's main-thread
  executor, and cancelled. The native window remained open. The second request
  returned through the same executor and approved; the app inventory then
  reported the synthetic process stopped. Content-free trace:
  `target/m2-tauri-termination-probe.log`. This tests the actual pinned Tauri
  event loop, but not the production renderer's decision dialog.
- An initial main-queue delivery implementation timed out in the AppKit probe:
  a nested termination run loop could not reenter its active main-queue block.
  The final implementation schedules an AppKit selector in the relevant run-loop
  modes; both the native regression and Tauri menu test then passed.
- Desktop Rust regression tests cover operation/quit exclusion, stale decision
  recovery, replay refusal, and cancellation during an active operation in
  addition to the existing main-window and close-attempt authorization cases.
- `cargo test --locked -p ort-desktop --lib` passed all 26 tests; strict Clippy
  passed for the desktop and lifecycle crate across all targets. `CI=true pnpm
  check` passed, including 101 desktop and 24 contract tests, builds, static
  security checks, and the license inventory (728 Rust / 167 JavaScript
  packages, zero exceptions). `cargo fmt --all --check` and `git diff --check`
  also passed. Logs are `target/m2-termination-rust-tests.log`,
  `target/m2-termination-clippy.log`, and `target/m2-termination-check.log`.
- The AppKit regression is now wired into both macOS CI targets. Hosted results
  for this uncommitted checkpoint are not claimed.

Native implementation SHA-256:
`60213fef09a49473e785ec52b853a9fd207f60f56b285eba90b763ceb236b1df`.
Native probe SHA-256:
`b8c1a841d596489bda486d2e1f9d5eb44ab3b3753fd74550983bd7c992a9359c`.

## Remaining qualification

This supersedes the older source-level absence recorded in
[`m2-close-guard-smoke.md`](m2-close-guard-smoke.md); it does not supersede the
installed-artifact evidence. A refreshed signed application still needs native
Dock Quit, clean/dirty/invalid edits, Save/Discard/Keep editing, overlay focus,
in-flight writes/dialogs, failed saves and renderer interruption checks in the
standard test account. Real logout/shutdown handling remains untested; no real
logout, shutdown, Force Quit, Keychain operation or user-data write was performed
for this checkpoint. Force Quit, crashes, power loss and recovery of unsaved
invalid edits remain outside this guard. Windows logoff/close qualification is
deferred. Step 5 and M2 remain incomplete, including the import containment gate.

Protocol references: Apple's
[`applicationShouldTerminate:`](https://developer.apple.com/documentation/appkit/nsapplicationdelegate/applicationshouldterminate(_:))
and [`terminateLater`](https://developer.apple.com/documentation/appkit/nsapplication/terminatereply/terminatelater).
The pinned runtime boundary is described in
[Tauri issue #9198](https://github.com/tauri-apps/tauri/issues/9198).
