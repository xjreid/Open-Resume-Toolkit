# M2 round-three preparation — 2026-09-09

> **Closed:** M2 is accepted for macOS Apple Silicon development. This file is
> historical evidence/procedure. The [closure record](m2-acceptance-closure.md)
> supersedes pending-status and further-test instructions below.


M0/M1 remain complete for macOS-arm64 development. M2 final acceptance remains
pending. These fixes are uncommitted on baseline `295e3b2`; the user's earlier
CI pass applies to that baseline, not these changes. No commit/push or installed
application replacement was performed.

## Round-two evidence and repaired behavior

Read `/Users/Shared/results-round-2/M2-test-results.md`, using its final summary
instead of superseded intermediate paragraphs. Multipage PDFs and DOCX in all
three styles, structured Present/expected/range persistence, sampled accessibility
preferences and dirty/invalid quit behaviors passed within the reported scope.
Two native observations remain important: an expired/stale import cancellation
error with an endless ordinary-quit wait, and incomplete main/overlay exit.

Native review expiry is 30 minutes. `ReviewSessions::cancel` previously called
`authorized`, which expired the slot and returned unavailable even when nothing
remained to cancel. Cancellation now confirms an empty slot idempotently. If a
new review is active, exact native owner/token checks still reject foreign or
retired tokens; expired tokens cannot reach storage or cancel a replacement.
The error path is reproduced with deterministic time in the session regression.
The original user's stale session was not inspected in memory, so expiry is the
identified reproducible cause, not an assertion about a captured native stack.

The editor previously treated the entire review as an active file operation for
quit. The new state distinguishes idle review from picker/parser/apply/cancel work.
Idle review can be discarded by normal quit, including after a cancellation transport
failure; actual operations still defer quit, and native operation sealing remains
unchanged. Review cancellation restores focus to the import button and provides a
live status. Editor mutation remains gated while review is present.

The new full-editor regression starts a pending import, verifies quit waits, cancels
the quit attempt, completes import into review, injects a cancellation failure, and
verifies ordinary quit can resolve. A separate hook regression covers Keep editing
and failed quit responses. No hook lifecycle-flag defect was found or repaired.

Saving reversed ranges is deliberate: product requirements call for a nonblocking
warning. Existing date code displays `Check this range: the end is earlier than the
start. You can still save it.` No range rejection was added. Final native testing
must establish that the warning is visible and accessible in the reported scenario.

## Native exit investigation

The synthetic Tauri example now has two blank windows and an automatic-exit mode.
It registers no application commands, opens no profile, and accesses no Keychain.

- Automatic `app.exit(0)` from Tauri's main-thread executor: process exit 0 and
  `EXIT observed`, log `target/m2-r3-two-window-exit.log`.
- Uniquely named `target/M2 Round3 Quit Probe.app` selected by its full path:
  native menu quit with overlay frontmost first canceled, second approved;
  both native callbacks/main-thread replies and `EXIT observed` recorded.
  The owned process exited 0. Log `target/m2-r3-native-menu-exit.log`.
- An initial UI selection resolved an older synthetic probe; it is excluded from
  exact two-window evidence. The separately launched owned process was stopped
  before repeating with the unique bundle.

The full-app overlay observation is NOT declared fixed or disproved by these
probes. It remains the first exact-candidate regression gate. If it persists,
record a fresh process/window inventory and trigger/state before concluding exit.
No additional lifecycle bridge/unsafe-code changes were made.

## Recovery and filesystem preparation

Added test-build-only checkpoints at restore old-directory rename, promotion,
rollback redundant-safety removal, safety-delete rename, all-data deletion intent,
key removal, and directory removal. They compile only for macOS storage tests,
never into the production app. The signed harness SIGKILLs/reaps only its own
child at each point, then uses production recovery to check expected draft and
publication, integrity, old-key removal/new identity after deletion, preserved
external backup, and cleared markers.

`target/m1-qualification/native-storage-reIyLQ/report.json` records:

- Existing native storage failure matrix passed (independent process/WAL recovery,
  migration interruption, corruption, native key separation and related primitives).
- Seven M2 recovery crash cases passed.
- Real ENOSPC on a private 64 MiB **APFS** image: failed SQLCipher write rolled back,
  committed records survived reopen, and writes recovered after freeing space.
- Image detached; temporary platform-test Keychain items/profile data cleaned by
  the harness. No installed-key read probe or account-profile operation was run.

The harness now accepts `--apfs`; default HFS+ behavior is preserved. This remains
primitive-level evidence, not every UI, power-loss or arbitrary-volume scenario.
A strict-Clippy test-length finding was resolved by splitting the test helper;
no storage behavior was changed by that refactor.

The separate ExFAT probe uses a bounded disposable image and fixed synthetic bytes.
Observed publication was refused with no chosen output. The final no-plaintext
payload assertion has NOT passed: the corrected rerun was blocked by automatic
approval review after its usage limit was reached. Empty
staging directories can remain when directory sync is unsupported. The initial
probe incorrectly required zero staging names; the corrected assertion is prepared
but not executed. The cleanup limitation remains recorded. Logs and
image: `target/m2-r3-filesystem/`. The image was detached and can be copied for the
final native Save-dialog refusal check. No plaintext fallback was added.

The new `tools/m2-account-snapshot.py` provides read-only orttest receipts for
locked-startup file equality and post-deletion old-key absence/fresh identity.
It reads bounded encrypted files and non-secret manifests, queries key metadata
without `-w`/`-g`, refuses other console accounts, and refuses receipt overwrite.
It does not lock/unlock Keychain, retrieve secrets, or change profile data. Actual
orttest receipts are still pending. Developer encrypted-file baseline, with no
running developer ORT process, is in `target/m2-r3-developer-preservation-before.json`;
compare after the test-account deletion and before reopening the developer app.

## Checks and candidate

Canonical `CI=true just check` passed: 247 Rust tests, zero failures, ten opt-in
skips; 115 desktop tests and 26 contract tests; strict all-target/all-feature Clippy,
contract regeneration, formatting, type checks, builds, repository tools, web/secret
and license checks. Log `target/m2-r3-canonical.log`. The sandbox attempt had pnpm
resolution/network failures; the normal-host rerun passed. Opt-in results above
are distinct from the ordinary suite's skips.

New candidate: `target/m2-candidate-r3/Open Resume Toolkit Dev.app`.
Packaging and strict nested signature verification passed using the existing local
signing identity. Original `target/m2-candidate` and its Shared copy remain intact.

- Desktop SHA-256: `1469545559ba3ebf737821b304635a3d26200f4a82cdb2b451b49565b2eabbcb`.
- Helper SHA-256: `7fb2672c90674f2d4998030b28daafe35a1d21f8b8ff65c73e8af28a9a21aa87`.
- Helper CDHash: `8838ab4dcd837a7ec607c2836d4c6ed9bdd05686`.
- Signer fingerprint: `314932FE09CB08143DDC66FED25BA732FD20CFE5`.

The exact new bundled helper passed PDF/DOCX extraction, wrong running-code identity
rejection, mid-job cancellation and reaping (`target/m2-r3-bundled-helper.log`), plus
repeated jobs, malformed/truncated/trailing/oversized input, parent-death EOF and
real input/output-stall watchdog checks (`target/m2-r3-parser-helper-native.json`).
These new results do not reuse the old helper's identity qualification.

Packaging/staging tools accept fresh output/candidate/checklist paths. The native
parser checker accepts an exact helper and fresh report path to preserve old logs.
The final transfer is `/Users/Shared/ORT-M2-acceptance-w7p7qqd9`, recorded in
`target/m2-candidate-r3/transfer.json`. Staging signature verification passed;
supplements include the read-only receipt helper and unmounted ExFAT fixture.
The fixture must not be used to work around the blocked final ExFAT rerun; the
handoff keeps that case pending explicit authorization or resolution of the block.

## Remaining signoff

Use [the round-three plan](m2-round3-orttest-handoff.md) from the new transfer.
It includes the real 31-minute review expiry, so a single session must allow for
that interval and coordinated restart/logout. It retains the earlier passing
reader evidence while testing affected/new boundaries. Any failed/unrun required
case remains open; one final session cannot be guaranteed if a new failure occurs.
The user must commit through the normal workflow and obtain CI for the final code.
Full M2 signoff is withheld until exact-candidate native regressions, recovery,
Keychain/deletion, filesystem UI, offline and remaining accessibility/lifecycle
requirements are reconciled. No Windows/Intel or production distribution claim.

## ExFAT retry completed

The user explicitly authorized the retry. The corrected probe passed: unsupported
publication was refused, no chosen output or named plaintext payload remained,
and the 64 MiB image detached successfully. Empty staging directories may remain;
this cleanup limitation is unchanged. Receipt: `target/m2-r3-filesystem/report.json`.
This supersedes the earlier blocked-rerun status, not the separate native Save-dialog
acceptance requirement.
