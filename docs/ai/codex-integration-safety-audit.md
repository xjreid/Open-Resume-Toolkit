# Codex integration safety and quality audit

Status on October 9, 2026: all six findings below have been repaired. The
resolution and verification record follows the original findings.

Original audit verdict: changes required before approval. Reviewed the local changes,
including untracked Codex modules, using the thermo-nuclear code quality review
skill. This audit follows the earlier integration review; it identifies remaining
gaps rather than treating the earlier repairs as sufficient.

No production code or installed application was changed during the initial audit. No
application launch, administrator approval, real OAuth login, or live inference
was performed. Diagnostic fixtures live in the ignored `target/` directory.

## Findings

### 1. P1 — Protect the installer helper before executing it as root

Location: `apps/desktop/src-tauri/src/codex_install.rs:391–413`.

`prepare_helper` verifies the bundled helper's digest while copying it to a
user-owned temporary directory. It makes that copy mode `0500`, then passes its
path to an AppleScript that executes it with administrator privileges. A process
running as the same user can replace that file between verification and elevated
execution, including while the approval dialog is open. File permissions do not
prevent the owner from replacing a directory entry. The root helper's payload
verification happens after that helper has already been executed, so it cannot
protect this boundary.

This requires a local process running under the user's account and approval of
the installation. It is not a remote attack. A harmless filesystem reproduction
confirmed that a mode-`0500` file in a user-owned private directory can be replaced
by renaming another file over it; no elevated payload was executed.

Use a signed privileged service with authenticated requests, or a trusted system
bootstrap that copies and verifies the helper into a protected root-owned
location before executing that protected copy. Rehashing the same mutable path
immediately before execution still leaves a race. Add a regression that swaps
the staged helper during the approval boundary and verifies the substitute can
never be executed with elevated privileges.

### 2. P1 — Make runtime writes cancellable and include them in deadlines

Location: `apps/desktop/src-tauri/src/codex_runtime.rs:462–466,524–525` and
`apps/desktop/src-tauri/src/plan_runtime.rs:210–227`.

`Session::send` performs blocking writes to the child process's stdin. `rpc`
starts its deadline only after that write returns, and checks cancellation only
while receiving. A runtime that stops reading can therefore block a supported
request indefinitely. The request holds the session mutex, while disable,
disconnect, and shutdown wait for that mutex before dropping/killing the child.
Setting the interrupt flag cannot interrupt a blocked write.

A diagnostic built from a copy of the current runtime source reproduced this
with a fake child and a 100,000-byte request. The requested deadline and
cancellation threshold were 25 ms; the RPC returned only after a watchdog killed
the child at 250 ms, reporting an elapsed 263 ms and
`PLAN_RUNTIME_UNAVAILABLE`. This was a synthetic local child, not the installed
Codex runtime.

Give the transport an absolute deadline established before dispatch and bounded,
cancellable writes as well as reads. Process termination must be available
without acquiring the mutex held by a blocked exchange. Keep that ownership in
one transport abstraction instead of adding more timeout checks around callers.
Test pipe backpressure, cancellation before dispatch, disable during a stalled
write, and shutdown without a watchdog. Reproduce the current defect with:

```sh
cargo run --offline --manifest-path target/codex-review-proof/Cargo.toml --quiet
```

### 3. P1 — Put installer probes under the universal runtime permission boundary

Location: `apps/desktop/src-tauri/src/codex_install.rs:328–361`.

The installer starts two temporary runtime sessions through `probe`, directly
calling `Session::start_runtime`. These starts bypass `PlanRuntime` and the saved
Enable Codex permission. Installing while Codex is disabled therefore still
executes the runtime. Disabling Codex cannot retire these separately owned
sessions; the post-install probe even uses an unconditional false cancellation
callback.

Use offline archive/hash/signature verification while disabled. Any protocol
probe must use the canonical runtime permission and process owner, with a
permission recheck at launch and cancellation on disable/profile replacement/
shutdown. Installation permission should not implicitly bypass the user's
explicit runtime enable switch. Tests must verify zero process starts during a
disabled installation and termination when disabled during a permitted probe.

### 4. P2 — Scope ephemeral thread cleanup across every exit after creation

Location: `apps/desktop/src-tauri/src/codex_pass.rs:147–182`.

Thread cleanup runs only after `run_turn` returns. A model mismatch, storage
failure after thread creation, or provider error from `turn/start` returns
earlier. These errors can be nonfatal under `PlanRuntime`'s session policy, so
the process and loaded ephemeral thread remain alive. Repeated failures can
accumulate loaded threads; input accepted before a turn-start error can also
remain in memory. This finding does not establish persistence on disk.

Own the ephemeral thread's entire lifetime in one scoped operation. Every
nonfatal exit after creation must unsubscribe it; failed cleanup must destroy
the session. Define fatal-session classification once rather than maintaining
parallel string lists in the pass and runtime manager. Exercise model mismatch,
dispatch-accounting failure, turn-start rejection, and failed unsubscribe.

### 5. P2 — Clear unknown quota instead of displaying the previous snapshot

Location: `apps/desktop/src-tauri/src/chatgpt_plan.rs:241–256` and
`apps/desktop/src/shared/ApplicationCodexControls.tsx:81–93`.

A failed quota RPC or invalid quota response exits before replacing
`session.quota`. On nonfatal failures the manager republishes the old snapshot
with an error code, while the overlay still displays its percentages as current
remaining usage. A successful read followed by an unavailable read therefore
violates the requested behavior of hiding unknown usage.

Invalidate the quota snapshot when its refresh fails, or represent freshness and
availability explicitly and hide unavailable values. Keep the existing fresh
quota preflight before every pass; this is a display correctness defect, not an
API-key fallback or reserve-enforcement bypass. Add a known-then-failed refresh
test through both the native status response and overlay rendering.

### 6. P2 — Collapse the duplicated material workflow into one orchestration loop

Location: `apps/desktop/src-tauri/src/ai_material_request.rs:54–118,139–324`.

The Codex branch duplicates the API branch's logical-operation lifetime,
cancellation/profile checks, stage loop, progress, validation advancement, and
completion handling. That creates two places to maintain the four-call tailoring
policy. There is already observable diagnostic drift: when local validation
rejects a successful Codex pass, the branch reconstructs its failure with a new
start timestamp and hardcoded zero retries, losing the real duration and reported
retries.

Select a typed, frozen provider executor once, then run one canonical material
stage loop. Provider executors should own dispatch, provider-specific preflight,
accounting, and settlement, and return a typed completed pass containing attempt
ID, text, original start time, and reported retries. The shared loop should own
workflow advancement and logical-operation closure. Verify API and Codex stage
parity and preservation of Codex timing/retry diagnostics after local validation.

## Boundaries checked

The normal runtime owner rechecks enabled/profile permissions under its process
lock. Enabled Codex routes exclusively to Codex, including when signed out, and
does not fall back to an API key. Runtime identity is pinned, managed credentials
are memory-only, the OS sandbox and host-restricted gateway constrain execution,
and window authorization limits overlay mutations. Model/reasoning changes use
revision checks and shared notifications. These strengths do not resolve the
installer's separate process and privilege boundaries.

No tracked file crossed from below 1,000 lines to above that threshold in the
current diff. Several already-large files grew. The duplicated orchestration in
finding 6 is a concrete opportunity to delete complexity; splitting it into more
files without removing the duplicate loop would not address the design issue.

## Initial audit verification and limits

- Desktop native tests: 131 passed, 8 ignored qualifications.
- Desktop UI: 290 passed across 45 files; TypeScript check passed.
- AI tests: 56 passed; backup: 11 passed, 1 ignored; installer: 10 passed;
  storage: 69 unit tests and 2 additional tests passed, 5 ignored.
- Clippy with warnings denied, Rust formatting, web-security scanning,
  secret scanning, and whitespace checks passed.
- The installer/storage test run needed sandbox permission for local socket/FIFO
  fixtures; the rerun passed. It did not perform administrator installation.
- Blocked-write failure reproduced using copied current source and a synthetic
  child. Mutable-helper replacement demonstrated without root execution.

Passing the original tests did not cover the six findings. Real account login,
live inference, privileged installer behavior, and the explicitly ignored runtime
qualifications were not exercised during the initial audit.

## Repairs — October 9, 2026

1. The elevated command now runs a fixed system-Perl bootstrap with a cleared
   environment and taint mode. The bootstrap opens one bounded regular-file
   descriptor using no-follow/non-blocking flags, copies into a private protected
   directory owned by root, and checks the copy against the compiled helper
   digest before invoking only that copy without a shell. Swapping the
   user-owned helper before approval cannot authorize different root code.
2. `codex_transport.rs` makes stdin non-blocking using safe `rustix` APIs. The
   RPC deadline starts before dispatch, and every write checks cancellation and
   the absolute deadline, including during backpressure. Incomplete writes kill
   the process. Session retirement can now acquire the process lock promptly
   after interruption; no blocked pipe write can hold it indefinitely. Every
   exchange also observes the runtime owner's shared interruption signal, even
   when its caller supplies no cancellation callback. Sign-out drops the
   interrupted process directly, destroying its memory-only credentials.
3. The installer performs only offline archive, executable, and protected-path
   verification. Both installer runtime probes were deleted. Runtime startup
   remains with the enabled `PlanRuntime` permission boundary; installation
   cannot start a separate process that disable/shutdown fails to own.
4. `codex_session.rs` owns an ephemeral thread scope covering every nonfatal
   result after creation, including model validation, dispatch accounting, and
   turn-start errors. Failed cleanup or fatal protocol exchanges terminate the
   process. Session-retirement classification is shared by the manager, pass
   preflight, and thread scope.
5. Status polling and per-pass preflight share `refresh_quota`, which clears its
   old snapshot and publishes a new one only on success. Native status and
   overlay tests cover a known value followed by an invalid/unavailable refresh.
6. API and Codex select a typed frozen provider and share one material stage
   loop and logical-operation owner. Provider-specific dispatch/accounting stays
   in the executors. A typed completed pass retains attempt identity, original
   start time, requested model/reasoning, and reported retries through local
   validation failures.

Regression coverage checks protected-helper execution and replacement rejection,
symlink/hard-link/FIFO/size rejection, literal AppleScript arguments, zero-byte
cancelled/expired dispatch, pipe backpressure, disconnect/shutdown interruption,
early thread failures and unsubscribe failure, quota invalidation, shared
four-stage attempt linkage, cancellation, and duration/retry preservation.
The bootstrap tests run without elevation; only the root-identity requirement
is omitted in the test copy. The shipped bootstrap always requires root.

The application backup round-trip test still expected the previous format and
database versions. Its expectations now match the existing Codex upgrade:
backup format minor 8 and database schema 7. Publication retention, mixed
document versions, rendering, reopen, and restore assertions are retained.

### Repair verification

- Desktop native: **142 passed**, 8 explicitly ignored qualifications.
- Desktop TypeScript and UI: **291 passed** across 45 files.
- Application workflow: **47 passed**; offline installer: **10 passed**.
- Clippy with warnings denied, formatting, web-security and secret scans, and
  whitespace checks passed.
- No source file was pushed above 1,000 lines by these repairs. The material
  orchestration has one stage loop; semantic session lifetime and bounded writes
  have explicit owners rather than additional caller-specific timeout branches.

No real account login, live inference, or elevated runtime installation was
performed. Existing development-signing/notarization limitations remain; this
repair does not claim production distribution approval. The installed app is
updated from the verified development bundle without being launched.

Delivery record: [codex-installer-update-20261009T041621Z](/Users/xavierreid/Open-Resume-Toolkit/target/codex-installer-update-20261009T041621Z/delivery-manifest.json). The installed bundle was signature/hash verified and confirmed closed after replacement; a rollback bundle is retained.
