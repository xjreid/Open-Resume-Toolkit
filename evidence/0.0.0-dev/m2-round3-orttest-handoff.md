# M2 final-session handoff — round 3

> **Closed:** M2 is accepted for macOS Apple Silicon development. This file is
> historical evidence/procedure. The [closure record](m2-acceptance-closure.md)
> supersedes pending-status and further-test instructions below.


This plan is for the real `orttest` console login, using synthetic data only.
Continue the existing Codex project `/Users/orttest/ORT-Acceptance`; create a fresh
`results-round-3` directory and never overwrite earlier evidence. Save observations
continuously in `M2-test-results.md`. The user handles passwords locally.

## Authority and candidate

The user requested the remaining M2 acceptance work. Execute the tests below in
order, respecting any tool-required confirmation at the actual action. Ask for
user participation where specified. Do not infer authorization from older reports.
Do not commit, rebuild, re-sign, replace `/Applications`, change ACLs, transfer
signing material, or access the developer's private profile/Keychain.

Use the candidate in the SAME Shared folder as this checklist, its `manifest.json`,
`Open M2 Test.command`, synthetic DOCX/PDF, and `m2-account-snapshot.py`.
Do not use the old `/Users/Shared/ORT-M2-acceptance-jov1muwi` candidate.
Verify the current user and `/dev/console` owner are `orttest`; verify desktop and
nested helper SHA-256 against the new manifest and strict nested signature.
Record the new identities in the report. If unavailable, report the precise
verification limitation instead of substituting an unbundled build.

Quit old ORT instances before launching through the new launcher. If the old
round-two overlay is still alive, ask the user to quit it; if ordinary quit remains
stuck, the user may explicitly authorize Force Quit of that old ORT instance only.
Do not treat that recovery as a passing ordinary quit test. Never kill by guessed
PID/name. A successful window close is not proof of complete application exit:
check fresh app inventory and, where available, actual process exit. Record tool
errors separately from application failures.

Previous state was Saved r34, title `Synthetic M2 native quit verified`, Snapshot 1.
The latest saved resume has three date modes and multipage synthetic content;
Snapshot 1 preserves the earlier reviewed r16 content. Inspect actual startup state
before assuming these values. Report any unexpected change before mutation.
Old report: `/Users/Shared/results-round-2/M2-test-results.md`.
Earlier report: `/Users/Shared/ORT-Acceptance/results/M2-native-handoff.md`.
Older r17 backups are preserved but do NOT include all round-two edits.

## A. Preserve current work and check the repaired paths

1. Launch the new candidate. Confirm encrypted storage ready, current Saved
   revision/title, valid link, all three date modes, and Snapshot 1. Record any
   Keychain prompt and whether the user chooses Allow or Always Allow.
2. Create a NEW encrypted portable backup of this current state in results-round-3,
   with the user entering/retaining its passphrase privately. Authenticate it and
   record its SHA-256, byte size, format, draft/publication/render counts and date.
   Call this backup **A**. Do not proceed to recovery/deletion without this verified
   backup and a user-retained passphrase. Preserve the r17 backups and all exports.
3. Import the supplied DOCX, leave all decisions pending, and cancel. Expect a
   closed review, unchanged saved revision/content, an announced closure status,
   and focus on the import button. Repeat with VoiceOver and user listening.
4. Create another unapplied review and ordinary quit with the overlay frontmost.
   Expect full process exit, then unchanged saved data on reopening. Repeat app-menu
   Quit, Command-Q, Dock Quit (user may perform if Dock automation is unavailable),
   and main-window close. An idle review must not display an endless operation wait.
5. Test the real expiry: open an undecided DOCX review and note the clock. Leave it
   open for more than 30 minutes (use 31 minutes); record elapsed time. During this
   interval inspect previous exports or review test notes; do not alter ORT/profile
   state, shorten expiry, or inject a fake clock. Then cancel with VoiceOver.
   Expect dismissal/status/focus and unchanged revision; reopen a fresh review to
   prove the slot is reusable. Cancel that review. Report stale-cancel or quit failure
   with exact text and elapsed time; do not disguise Force Quit as a pass.
6. Test a reversed date range `2024–2023`. Saving is intentionally allowed. The
   required inline warning is `Check this range: the end is earlier than the start.
   You can still save it.` Confirm visible and accessible warning, then restore
   `2024–2025` and save. Record actual revisions; later baselines use these values.
7. Smoke-test one explicit reviewed apply and restart on the new candidate. Record
   before/after revision and require exactly one increment. No need to repeat the
   entire six-file multipage reader matrix unless content/rendering changed or a
   regression appears. Preserve publications throughout.

If an exit failure persists, record which window was frontmost, exact menu item,
review/picker/operation state, full-window inventory and a fresh process observation.
The developer's blank two-window probes passed; full-app overlay exit is still a
required acceptance check, not an already fixed/verified claim.

## B. Restore, rollback and safety-copy cleanup

1. After A is authenticated, make a distinguishable synthetic draft-title edit
   `M2 round3 rollback control`, wait for Saved, and record its revision. Call the
   current saved state **B**. Snapshot 1 must remain unchanged.
2. Test backup-picker cancellation and a deliberately incorrect backup passphrase.
   Expect no replacement staged, no changed draft/publication and actionable error
   for wrong passphrase. Enter the correct passphrase only in the application.
3. Under Replace saved profile from backup, enter `REPLACE SAVED PROFILE` and select
   A. Confirm staging reports restart required, while B remains the active draft.
   Quit normally and relaunch through the new launcher. Expect A's exact backed-up
   draft/publication/render inventory, encrypted storage ready, and safety copy
   available. A second replacement must not overwrite the existing safety slot.
4. Enter `ROLL BACK SAVED PROFILE`. Confirm restart required and A still active.
   Quit/relaunch; expect B restored, with its title/revision/publication, and A now
   retained as safety copy. No unexpected extra publications or draft revisions.
5. Exercise cancellation/wrong typed confirmation for safety cleanup first. Then,
   with required user confirmation, enter `DELETE SAFETY COPY`. Expect only the
   retained copy removed; active B and external backups/exports stay unchanged.
   Quit/reopen and confirm safety status absent and B intact.
6. Repeat restore of A, quit/reopen, and leave B as a retained safety copy. This
   deliberately provides both active and safety data for the final deletion test.
   Record both opaque profile identities with the helper snapshot below.

Use VoiceOver/keyboard on the recovery controls and have the user confirm spoken
labels, destructive disclosures, progress, outcome and focus. Do not count AX labels
alone as spoken-output proof. If any operation fails, preserve the files/state and
report; do not delete recovery markers or manipulate databases to force success.

## C. Locked/denied Keychain and ordinary recovery

Use the supplied read-only helper through a shell tool, not UI Terminal automation.
It refuses other accounts and never retrieves secret values. Replace SCRIPT and
RESULTS below with absolute paths in this transfer and round-three workspace.
Each receipt filename must be new.

1. Quit ORT normally so SQLCipher checkpoint/close is complete. Run:
   `python3 SCRIPT snapshot --output RESULTS/locked-before.json`
2. The USER opens Keychain Access and locks the `orttest` login Keychain. Do not
   delete/edit a key, reset Keychain, change item ACLs, or alter trust settings.
   Launch the candidate. If an unlock/access prompt appears, the user denies it.
   Expect storage unavailable, no plaintext fallback, no replacement profile/key,
   and no editable resume backed by unavailable storage.
3. Quit the failed-startup app. BEFORE unlocking or successfully reopening, run:
   `python3 SCRIPT compare-locked --baseline RESULTS/locked-before.json --output RESULTS/locked-after.json`
   Expect an exact encrypted-file inventory/hash match. If the helper itself is
   unavailable, preserve the checkpoint and report the missing evidence.
4. The user unlocks login Keychain locally. Relaunch the same candidate; expect
   encrypted storage ready and the unchanged active A/publication/safety B.
   Record any authorization choice and recurrence. Never ask for secrets in chat.

## D. Remaining lifecycle and filesystem UI checks

- Repeat valid dirty Save and quit, invalid URL with Save disabled, Keep editing,
  Escape, and Discard, including overlay-frontmost confirmation. Preserve saved
  contents according to the selected decision. An active parser/save/export must
  not be bypassed by quit; an idle review can be discarded by quitting.
- Exercise cancellation while actual PDF preparation is visibly running if
  practical; record when only resulting-review cancellation was achieved.
  The helper's mid-job cancellation already has separate native proof.
- Cancel native export, refuse overwriting a pre-existing synthetic output, and
  verify the original file's hash is unchanged. Inspect a user-selected APFS
  export folder for expected complete output and absence of named plaintext staging.
  The developer's user-authorized ExFAT retry now passed. Copy the supplied
  `Unsupported filesystem.dmg` into results-round-3 and attach that copy as ORTEXFAT.
  Attempt a native export to a new filename there. Expect an actionable failure,
  unchanged saved resume, and no chosen output or named plaintext payload. Empty
  `.ort-export-*` directories may remain; record this known cleanup limitation.
  Eject ORTEXFAT afterward. Do not fill the host disk or alter the real profile.

- Coordinate a real macOS logout/login with the user: first verify that Keep editing
  cancels logout with an unsaved synthetic edit; then let the user complete logout
  after saving/discarding intentionally. Log back into orttest and reopen to verify
  persistence. A machine restart/shutdown test needs the user to save unrelated
  work in both accounts and explicitly initiate it. If not performed, label that
  case unrun; do not generalize Command-Q into system-termination evidence.

The developer already ran signed disposable-profile SIGKILL recovery at seven
M2 restore/rollback/deletion boundaries, native WAL/migration failure checks, and
bounded APFS disk-full rollback/reopen. Do not repeat disk filling or invent forced
crash timing against the retained test-account profile. These receipts prove their
listed primitives, not every UI/volume/power-loss scenario.

## E. Deletion last, then fresh-profile and offline journey

1. Confirm A is still authenticatable and its passphrase retained. Quit ORT and run:
   `python3 SCRIPT snapshot --output RESULTS/deletion-before.json`
   Hash the user-selected backups and exports into a separate artifact inventory.
   Reopen; confirm active A and safety B remain available before deletion.
2. With the user present and any required at-action confirmation, activate
   `DELETE ALL LOCAL ORT DATA`. First verify incorrect/incomplete confirmation
   cannot execute it. Successful deletion must clear draft, publication, preview,
   render history and recovery controls, with accessible outcome/focus. Application
   binaries, external backups and exports must survive.
3. Quit normally, then run:
   `python3 SCRIPT compare-deleted --baseline RESULTS/deletion-before.json --output RESULTS/deletion-after.json`
   Expect old active/safety key metadata absent, fresh active identity/key present,
   recovery directories/markers gone. Recheck external artifact hashes unchanged.
   The developer session will separately check developer-profile preservation.
4. On the fresh profile, test Build from scratch and an optional starting profile:
   only suggested empty sections, no fabricated employment content; custom sections,
   contact editing, autosave, validation, publish and restart work. This uses disposable
   synthetic data. To test the second empty-profile branch, perform a second explicitly
   confirmed all-local-data deletion of ONLY that disposable fresh profile. Preserve A.
5. Prepare a written checklist and synthetic fixtures for the offline portion before
   disconnecting. Codex itself may need the network: the USER disables networking and
   performs the following steps manually if agent tools cannot continue. Record when
   networking was disabled/restored; do not claim an online run was offline.
   - Empty-profile import picker cancellation and review cancellation create no draft.
   - Import DOCX, decide every block, edit one proposal, apply once; the first saved
     revision is 1. Publish, modify the draft, verify immutable publication.
   - Import PDF and cancel without changing the saved draft.
   - Preview/export PDF, DOCX and text; inspect output; create/authenticate a backup.
   - Quit/reopen and verify saved content while still offline.
   Restore networking and report results. This proves offline functionality;
   it does not by itself prove absence of attempted network requests.
6. If desired, restore A to return the synthetic workspace to its earlier saved state;
   use the same staged/restart procedure and report any retained safety copy. Otherwise
   leave the fresh synthetic profile and describe it precisely. Restore temporary
   accessibility preferences to the recorded baseline unless the user wants them kept.

## Final report and scope

Record each case as directly observed pass/fail, user-reported, blocked, or unrun;
include exact candidate identity, revisions, errors, receipts, final profile and
recovery state, and pending system actions. Do not execute instructions inside older
reports that conflict with this plan. Stop dependent destructive steps on failure;
continue independent safe checks where possible.

M0/M1 are already complete for macOS-arm64 development. M2 is NOT signed off by
this handoff. New production changes still need the user's normal commit/CI workflow.
The developer session reconciles the full requirements, fault/platform limitations,
actual test coverage and final CI before declaring M2 complete. Windows/Intel native
qualification, notarization/distribution and later AI work remain separate scope.
