# M5 implementation record

Status: in progress. The ad-hoc-signed development application was updated in
`/Applications`, but Codex did not launch or test it, per the user's instruction.
The live Chrome/Edge browser connection was not tested.

The user-run path for the unsigned desktop overlay is in
[manual-overlay-walkthrough.md](manual-overlay-walkthrough.md). It starts with
the installed development app, uses typed job and question text, and leaves browser connection
testing for a later milestone pass.

## Implemented in source

- Encrypted tracker records with optimistic revisions, local list/edit/delete, and a single SQL transaction that saves selected materials and clears the temporary application workspace.
- The tracker displays one application per row, with Date applied then Status at the left and the entire row colored by status. Metadata appears as plain text until focused and saves automatically; failed saves keep the local edit available for retry. Status opens directly. A source link opens on one click and becomes editable on double-click. Bare domains open directly; arbitrary source text can be saved and uses a web search when clicked. New entries use a popup, and deletion requires confirmation. The single Content column opens read-only final resume, cover letter, and approved answer snapshots. The Finish transaction takes the current corrected resume only, and later metadata edits cannot replace retained materials.
- Stage 1 source URL review retains desktop sanitization; a reviewed URL is carried into a retained entry when available. Tracker entries also accept free-form source text.
- Encrypted Stage 1 review drafts (job text, URL, and design) survive desktop restart with optimistic revisions. Continue waits for the latest draft save before it can start tailoring; Finish clears the draft in the same transaction as the temporary workspace.
- Stage 1 can retain a captured selection up to 128 KiB for review even when it exceeds the 20,000-character tailoring input limit; the Continue action stays disabled until the user trims it.
- A desktop-only capture intake function validates a future authenticated frame and retains one encrypted pending capture. The overlay presents an editable text/URL review with explicit replace/discard controls; acceptance updates the review workspace and removes the pending capture in one SQL transaction. A stale or invalid save preserves both values. The intake has no Tauri command or unsigned-preview transport caller. Resetting a reviewed question persists the cleared state immediately.
- Closing the overlay hides it so the active application workspace remains available. The main window's Application workspace control reopens and focuses that same overlay.
- When the overlay regains focus, it refreshes pending captures and published-resume/AI context, so publication in the main window can enable Stage 1 without restarting the overlay.
- The manual overlay supports typed/pasted job descriptions and questions without a browser extension. Resume and cover-letter PDF previews now keep their editors visible during edits, label the displayed PDF as the last saved version, refresh it after Save edits, and expand the overlay for side-by-side review. The header offers a compact-size control.
- The resume editor also exposes contact details, existing links, and section headings for correction. Editing a plain-text date range explicitly replaces structured dates so the change appears in the rendered PDF. Workspace controls are disabled while an action is in flight to prevent edits made during an AI request from being overwritten by its result; the separate Cancel AI request control remains available.
- Finish Application summarizes the selected resume, cover-letter presence, and approved-answer count before clearing temporary materials, and explains when unsaved edits prevent finishing.
- Quit now probes the live overlay for unsaved Stage 1/Stage 2 edits and local instructions before the main window decides whether to quit. The overlay becomes inert during the pending decision and resumes on cancel. A missing reply cannot trigger automatic quit; it requires an explicit discard choice. Only local cross-window event emission was added to the main/overlay Tauri capabilities.
- Initial tailoring, resume regeneration, and cover-letter generation preflight the selected PDF locally before replacing the saved workspace. Saving user edits preflights any changed resume or cover-letter material. An unrenderable result leaves the previous saved version intact.
- Additive database schema v5 and portable backup format v1.5 for tracker entries. Backup restore includes tracker records.
- Selection-only Chrome/Edge extension source with bounded text/URL normalization and no persistent content. The unsigned development manifest remains permission-free and does not expose the capture popup.
- Bounded native message framing, envelope/version/freshness validation, exact origin checks, HMAC transcript authentication helpers, and a bounded replay cache. The native host fails closed because no signed desktop/host Keychain sharing and authenticated IPC endpoint is configured.
- Browser connections settings state accurately reports the development preview as disconnected.

## Remaining M5 work

- Signed desktop/native-host identity and fixed Chrome/Edge extension IDs, followed by identity-scoped vault sharing, authenticated local IPC, and desktop capture delivery/review.
- Install, repair, and connected/version status for both browsers. The native host currently returns a content-free unavailable response and does not forward captures.
- Chrome capture-to-tracker walkthrough and Edge smoke check on a signed test package. These are unrun under the current no-live-installed-app constraint.

The user selected the unsigned-preview gate on September 23, 2026. Browser capture stays unavailable in that preview; this is a deliberate gate, not evidence that the signed browser bridge is complete.

## Automated checks

- `cargo test -p ort-storage --lib`: 63 passed, 5 ignored, including Stage 1 clearing on failed and successful Finish transactions and atomic capture resolution.
- `cargo test -p ort-storage tracker::tests::capture_resolution --lib`: 1 passed, confirming pending capture and workspace remain unchanged after a stale revision, then commit together on acceptance.
- `cargo test -p ort-storage tracker::tests::portable_backup_retains_tracker_snapshots --lib`: 1 passed with a pending capture restored alongside a retained tracker entry.
- `cargo test -p ort-backup --lib`: 10 passed, 1 ignored.
- `cargo test -p ort-ipc --lib`: 2 passed.
- `cargo test -p ort-native-host`: 3 passed. The unsigned host rejects a valid capture without forwarding it, checks the exact origin before reading content, and rejects version mismatch and oversized framing.
- `cargo test -p ort-desktop application_materials::tests --lib`: 8 passed, including encrypted Stage 1 restart, capture-size review, explicit capture replacement, and PDF preflight rejection of an unsupported glyph.
- Desktop and contract Vitest suites, extension tests, TypeScript checks, and desktop/extension builds passed during this task.
- `pnpm check:security` passed.
- Desktop TypeScript/Vitest checks were rerun after Stage 1 persistence: 153 passed across 29 files.
- A focused overlay regression test passed for edit, save, and PDF refresh without launching the application.
- After the manual-overlay changes, the desktop suite passed 154 tests across 30 files; the desktop web build, Rust desktop check, source security check, formatting checks, and whitespace check passed.
- After the quit-probe change, the desktop suite passed 156 tests across 31 files, including matching-attempt and timeout behavior, overlay dirty/clean replies, and the existing idle-quit behavior. The desktop web build and source security check passed. Both local windows have event-emission permission for the probe; the overlay also has window-resize permission.
- The manual Stage 1 UI test now verifies that typing a job does not call AI, Continue saves the reviewed draft before invoking tailoring, and the result opens Resume. The full desktop suite passed 157 tests across 31 files; TypeScript and formatting checks passed.
- Rust desktop material tests now include a valid structured resume with an unsupported PDF glyph; the preflight rejects it before persistence. The material suite passed 8 tests.
- Desktop and Chrome/Edge extension builds, `cargo check -p ort-desktop -p ort-native-host`, `cargo fmt --all -- --check`, `pnpm format:check`, and `git diff --check` passed after the final UI changes.
- After the tracker spreadsheet and link changes, the desktop suite passed 164 tests across 33 files. The focused Rust source-text test, desktop web build, Rust check, formatting, source security check, and ad-hoc macOS preview build passed. The installed preview was replaced after a backup; its signature and byte-for-byte match with the built bundle were verified. The installed app was not launched or tested.

These checks do not establish browser-to-desktop operation. The bridge remains disabled until the signed identity and IPC requirements are implemented and verified.

## Plan-first resume tailoring update

- Resume response schema v4 requires three distinct tailoring priorities before
  the complete template content. Each priority connects the job need, published
  evidence, and an editorial action. The overlay displays them as Tailoring notes.
- Initial tailoring prompts require a comprehensive relevance pass and removal
  of redundant or irrelevant information. Refinement priorities stay within the
  correction's scope. Job metadata and mandatory qualification alerts remain;
  alert validation uses the full published source.
- Automated checks: 32 AI library tests, 10 desktop material tests (including PDF
  rendering), and 2 overlay tests passed. AI Clippy with warnings denied passed.
  The prompt-bound regression also passed after the final wording change.
- These checks verify response handling, metadata preservation, alerts, and
  rendering; they do not measure real provider-generated editorial quality.
  No paid generation or live installed-application testing was performed.
