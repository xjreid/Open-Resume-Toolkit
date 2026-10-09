# Codex integration review — October 8, 2026

Reviewed the local ChatGPT plan integration, supervised Codex adapter, installer,
request routing, UI polling, activity accounting, and profile lifetime boundaries.
Applied the thermo-nuclear code quality review skill. Findings below were fixed.

## Findings and repairs

**P1 — Settings and network authorization shared one blocking control path.**
`connected()` forced `account/read` with `refreshToken: true` for ordinary status
reads. Saving enabled settings invoked that check and then invoked it again while
building the response. Each refresh allowed ten seconds, before any quota request.
The session mutex also made background polls wait behind protocol work.

`PlanRuntime` now owns the serialized protocol and a separate, non-secret,
profile-scoped snapshot. Settings validation and save responses use that local
snapshot, without network calls or waiting for protocol ownership. UI polls skip
a busy protocol. Local account reads use `refreshToken: false`; every workflow
pass still checks authorization and fresh quota before dispatch. Disconnect can
interrupt a background refresh. The documented distinction between local reads
and forced renewal is in the [managed authentication contract](https://learn.chatgpt.com/docs/app-server#auth-endpoints).

**P1 — Unrecognized protocol messages were mislabeled as containment violations.**
The adapter treated every notification outside a small allowlist as a tool or
permission action and killed the process. Normal model verification, safety
buffering, auth-recovery, and warning notifications could therefore be reported
as permission violations. Destroying the memory-only process also ended sign-in.

Documented passive notifications from the pinned protocol are now accepted.
Actual tool/permission actions still stop the process. Unknown messages still
fail closed, with a protocol error distinct from a containment error. Monitoring
receives fixed, privacy-safe reason identifiers and an explanation of why sign-in
ended. No filesystem, subprocess, Keychain, or network permission was broadened.
The previous attempt's generic error does not identify its actual notification;
this review cannot establish which event caused that historical failure.

**P2 — Two UI loading owners and overlapping polls could overwrite newer actions.**
The workspace and plan page loaded independently. Interval polls overlapped;
mutation responses lacked the same stale-response guards as refresh responses.

One `useChatGptPlan` controller now owns loading, polling, and mutations. It
coalesces polls, suppresses polling during mutation, invalidates stale refreshes,
and ignores responses after unmount/profile replacement. Background usage refresh
has its own busy state and does not disable model/reserve settings.

**P2 — Ephemeral threads and per-pass diagnostics outlived their useful scope.**
Completed threads were never explicitly released. A new thread's model rejection
could be attributed to the model from the previous operation. A later preflight
failure could retain earlier diagnostic fields.

Finished provider turns now release their ephemeral thread through
`thread/unsubscribe`; failed cleanup destroys the process. Each pass clears its
reports, checks the frozen connection identity, and records the newly requested
model before a rejection can arrive. Accounting settlement also verifies the
profile. Usage is still replaced rather than summed across cumulative events.

**P2 — Workflow and runtime files crossed 1,000 lines.**
Application-material orchestration now lives in `ai_material_request.rs`;
runtime tests live in `codex_runtime_tests.rs`. Runtime ownership lives in
`plan_runtime.rs`, while React orchestration lives outside the presentation
component. This also removes duplicate fake-session construction and the UI's
manual reconstruction of partial busy snapshots.

## Containment checks retained and strengthened

- Exact official executable digest and protected root-owned path checks.
- Default-deny OS sandbox; private auth/scratch roots; no shell/fork or general
  filesystem access; only the provider-host CONNECT gateway.
- Ephemeral credential storage, effective configuration validation, fresh quota
  before every pass, paused-key locks, no API-key fallback, and atomic acceptance
  of reviewed, validated one-page results.
- More model-facing features explicitly disabled. Startup rejects incompatible
  effective feature, web, credential, and configured MCP/plugin policies.
- Both `update_plan` and `experimental_request_user_input` must be explicitly
  disabled. Codex 0.162.0's `ToolsV2` projection omits those fields; the adapter
  checks the supported raw config layers in high-to-low precedence instead.
  Missing, enabled, or malformed flags fail closed. Config layers are never
  logged, monitored, or returned to the UI.

The pinned [configuration types](https://github.com/openai/codex/blob/rust-v0.162.0/codex-rs/app-server-protocol/src/protocol/v2/config.rs)
and [configuration reader](https://github.com/openai/codex/blob/rust-v0.162.0/codex-rs/app-server/src/config_manager_service.rs)
were checked directly. Automatic approval review rejected removing the new tool
check; the supported layer check preserves enforcement and passed qualification.

## Verification

- Desktop native suite: **126 passed**, 8 explicitly ignored qualifications.
- AI, storage, backup, and installer suites: **148 passed** in addition to desktop.
- Desktop UI: **280 passed**; shared contracts: **41 passed**.
- Rust Clippy with warnings denied, TypeScript, formatting, secret scanning, web
  security scanning, and whitespace checks passed.
- Separately invoked real-runtime qualifications passed: signed-out thread
  creation/release, synthetic memory-only auth/restart/logout, and OS confinement.
- Delayed fake-provider tests verify 20 local control/status checks finish within
  200 ms while forced OAuth renewal takes at least 450 ms. Settings and skipped
  polls stay responsive during a delayed quota call; disconnect interrupts it.
- React tests verify stale refresh rejection, single polling, mutation locking,
  and no publication after unmount.

These are deterministic and signed-out checks. No real account login or live
resume inference was performed. A user-initiated tailoring attempt is still
needed to confirm account-specific behavior. A real prohibited tool action will
continue to stop the process and end memory-only sign-in.

The development app is rebuilt and installed without automatically launching it;
the delivery manifest and recoverable previous bundle are kept under the build's
`target/codex-installer-update-*` directory.
