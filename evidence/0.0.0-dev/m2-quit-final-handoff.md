# M2 final quick acceptance — ORT test account

> **Closed:** M2 is accepted for macOS Apple Silicon development. This file is
> historical evidence/procedure. The [closure record](m2-acceptance-closure.md)
> supersedes pending-status and further-test instructions below.


Continue in `/Users/orttest/ORT-Acceptance`. Create `results-round-4` and preserve previous results. This session is limited to the repaired menu-quit regression and saved-data persistence. Budget approximately 5–10 minutes after launch; no timed expiry check or repeat of the previous acceptance matrix is needed.

## Authorization and operation

The user authorizes autonomous execution of this checklist: inspect candidate identity and processes, launch and relaunch the supplied app, use native UI and ordinary shell commands as needed, create/edit a synthetic draft, import the supplied synthetic document without applying it, quit the candidate, and save evidence. Proceed without asking for confirmation at each step. If a genuine OS password/permission prompt prevents progress, report the specific blocker; this handoff does not change macOS or Codex permission enforcement.

## Quick check

1. Use the candidate beside this checklist and `Open M2 Test.command`. Compare the desktop executable SHA-256 with `manifest.json`, record the candidate path, and launch it in the real `orttest` login. Close any previous ORT instance normally first.
2. Wait for encrypted storage ready. If there is no draft, choose Build from scratch. Set the draft title to `M2 final quit check`, wait for Saved, and record its revision and publication count.
3. Import the supplied `Synthetic resume.docx`. Leave the review unapplied. Bring the candidate's overlay window to the front, then click the application menu's exact **Quit Open Resume Toolkit** item. Verify the whole candidate process exits and both windows disappear. This step must use the menu item because Command-Q previously passed while clicking the menu failed. Allow up to 15 seconds for ordinary completion; record an unresolved quit instead of repeatedly waiting.
4. Relaunch the same candidate. Verify encrypted storage ready, title `M2 final quit check`, the same saved revision and publication count, and no pending import review. Then use **Command-Q** and verify the whole process exits.

## Result and completion decision

Save a concise `results-round-4/M2-final-quit-results.md` with candidate identity, each observed outcome, saved-state values before/after, and any blocker or failure. Screenshots/logs are useful only when they establish an outcome or explain a failure. Do not invent passes from a tool action alone.

If both quit routes and saved-data persistence pass, report **M2 accepted for the macOS Apple Silicon development scope, with remaining manual checks waived by the user**. The user has accepted that skipped round-three tests remain unrun; they are not failed or passed by this session. Preserve the earlier restore/archive-selection ambiguity as inconclusive. This scoped acceptance does not claim release/distribution qualification, other-platform qualification, or a new hosted CI result. If the focused regression fails, report the concrete failure and leave this final acceptance pending.
