# M2 final menu-quit repair — 2026-09-09

> **Closed:** M2 is accepted for macOS Apple Silicon development. This file is
> historical evidence/procedure. The [closure record](m2-acceptance-closure.md)
> supersedes pending-status and further-test instructions below.


Round-three report: `/Users/Shared/results-round-3/M2-test-results.md`.
The report observed app-menu Quit doing nothing with the overlay frontmost and an
unapplied import review; Command-Q subsequently exited and reopening retained the
saved revision. The exact underlying custom-menu event failure was not instrumented.

## Change

On macOS, `apps/desktop/src-tauri/src/menu.rs` now creates the standard native
`PredefinedMenuItem::quit`, labeled `Quit Open Resume Toolkit`. It invokes AppKit
termination and the already-installed `ort_macos_lifecycle` delegate bridge rather
than depending on the custom Quit menu-event route. That bridge requests the main
editor's close decision, waits for the asynchronous response and replies to AppKit.
Existing dirty-state and in-flight-operation guards remain authoritative. Other
platforms retain the custom menu item. No storage format or resume behavior changes.

## Verification

- `cargo fmt --check -p ort-desktop`: passed.
- `cargo check -p ort-desktop --examples`: passed.
- `cargo test -p ort-desktop --lib close_guard --no-fail-fast`: two passed.
- Focused frontend `close-guard.test.tsx` and `document-import.test.tsx`: three tests
  passed in two files.
- `git diff --check`: passed.
- Prior round-three synthetic native probe already exercised the predefined Quit
  API, two windows, and asynchronous cancel/approve through the same bridge. That
  is historical supporting evidence, not a new full-candidate observed pass.

The full app was not launched against the developer profile. Exact-candidate native
acceptance remains the short test in [the final handoff](m2-quit-final-handoff.md).
The user waived remaining manual checks; no repeated expiry wait is required.
Hosted CI for these uncommitted changes has not been claimed.

## Signed candidate

`target/m2-candidate-r4/Open Resume Toolkit Dev.app` built and passed strict nested
signature verification using the existing local development identity. Desktop
SHA-256: `34b674a0f3061d1e1c8b9d02fb713862396fcf57434cb17180b3c95d19f94b9b`. Parser identity matches round three: False.
The first sandboxed packaging attempt could not access the signing identity; the
normal-host packaging retry succeeded. Log: `target/m2-r4-package-host.log`.
Source hashes are in `target/m2-candidate-r4/source-qualification.json`.

The regenerated helper passed exact-candidate synthetic DOCX, PDF and invalid-header
smoke checks (`target/m2-r4-helper-smoke.json`). Its earlier full watchdog matrix
was not rerun and is not attributed to the new binary identity.

Transfer: `/Users/Shared/ORT-M2-acceptance-rn13guag`, including the signed candidate,
manifest, synthetic fixtures, launcher and final quick checklist. Prior transfers
and results are preserved. Final real-account result is pending.
