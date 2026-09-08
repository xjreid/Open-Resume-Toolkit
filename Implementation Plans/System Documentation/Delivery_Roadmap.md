# Technical delivery roadmap

**2026-09-07 architecture revision:** The user authorized replacing nonviable
containment assumptions. [ADR 0012](../../docs/adr/0012-metered-wasm-document-parsers.md)
selects metered, capability-restricted WebAssembly parsers with an isolated
helper/watchdog. For this path, guest linear-memory and instruction-fuel limits
replace the unproven whole-native-process 512 MiB / 30 CPU-second requirements.
Older native-probe requirements below are historical for that architecture;
no previous failed gate is reclassified as a pass. Production integration and
qualification remain required, and import remains disabled.


## Status

- Status: implementation sequence approved; dates intentionally unset
- Owner: maintainers
- Planning unit: demonstrable vertical milestone, not percentage completion

The Quiet Navy/Open Frame application and website direction is approved under `../../Aesthetic/`. Early milestones still use semantic, minimally styled controls and a deliberately plain renderer fixture so security, contracts, accessibility, and data behavior stabilize first. Production component polish and the non-default document-template details may be tested and refined during development, but the three promised style categories must pass their functional, accessibility, licensing, and golden-render gates before release.

M0, M1, and M2 now use macOS Apple Silicon as their only active native
qualification target. References below to Windows, Intel Mac, universal builds,
or cross-platform native evidence describe preserved later expansion work unless
the text explicitly identifies a shared CI portability check. Milestone
completion in this phase always means completion for macOS arm64, not global
cross-platform completion.

## Reasoning-effort routing

- **[HIGH]** marks work that requires high reasoning because an error could cross
  a trust boundary, lose or corrupt user data, weaken containment, misstate a
  fact, authorize spending, or invalidate release evidence. The marker applies
  to implementation, test design, review, and gate sign-off for the whole bullet.
- **[HIGH]** is a reasoning route, not a schedule priority or progress status;
  it remains on completed work to govern future changes and regression review.
- Unmarked work defaults to medium reasoning. Medium sessions may inspect or
  report the status of a **[HIGH]** item and run an already-defined check, but
  must not design, modify, approve, or declare the item complete.
- When a medium session reaches a **[HIGH]** item, it must stop that work and tell
  the user to switch to high reasoning. A user may explicitly override this
  routing rule for a bounded task.

## M0 — architecture skeleton and contracts

Current development status: the repository skeleton, contracts, isolated
development profile, static web/capability checks, and four-target CI are in
place. The 2026-09-04 tooling checkpoint adds a fail-closed Rust/JavaScript SPDX
allowlist, deterministic dependency inventory, and contract-regeneration drift
to the canonical `just check` path. The Step 3 checkpoint adds a macOS-arm64
qualification harness, exact local-certificate release-app verification,
pre-vault development-identity refusal, and non-mutating synthetic
cross-channel isolation tests. The Linux/Windows optional-fsevents license-gate
failure is repaired with an integrity-pinned single-package policy, not a broad
exception. Current local results and remaining signoff gates are recorded in
`../../evidence/0.0.0-dev/m0-macos-qualification.md`. **M0 is complete for the
macOS-arm64 development scope at `4bd2594` (2026-09-05).** The four CI jobs and
separate dependency vulnerability scan were independently verified successful
through the GitHub API. The installed app was reverified against the qualified
source digest, executable digest, and local signer. Windows/Intel native
qualification and production signing/notarization remain deferred. This signoff
applies to that checkpoint; later implementation changes require fresh artifact
verification and do not inherit its native evidence automatically.

Deliver:

- Cargo/pnpm workspaces, pinned toolchains, Tauri/React shell, and isolated dev profile;
- **[HIGH]** domain error envelope, command/event transport, generated schemas,
  and compatibility manifest;
- CI for the macOS-arm64 qualification target plus retained Windows/Intel-Mac portability builds, tests, formatting, license/vulnerability scanning, and schema drift;
- synthetic fixture policy and evidence layout;
- **[HIGH]** initial architecture decision records for Tauri/Rust, SQLCipher,
  Typst, native messaging, and external Codex.
- repository and CI skeleton matching `Development_and_Deployment_Outline.md`, including shared desktop source, shared Chrome/Edge extension source, and platform-specific packaging boundaries.

Exit evidence:

- clean checkout bootstrap;
- main and overlay windows can call a typed health command;
- **[HIGH]** production build has no remote web assets or broad Tauri
  capabilities;
- CI matrix is green.

## M1 — encrypted local core and structured resume

Current development status:

- **M1 is complete for the qualified macOS-arm64 development scope at
  `65518eb`**. Step 4 local qualification and all four hosted CI jobs plus the
  dependency scan passed; hosted results were independently verified for that
  commit. Evidence:
  `../../evidence/0.0.0-dev/m1-macos-storage-qualification.md`.
  macOS key creation uses a native exclusive add, with an upsert negative
  control and independent-adapter race tests. The empty-profile backup guard
  repair passes live UI tests and the real-account restore journey.
  Signed native tests pass actual WAL/migration process termination, corruption,
  wrong/missing keys, bounded-image ENOSPC, native key separation and exact-target
  deletion. An unapproved signed helper cannot read the installed app key.
  The real standard account passes developer file/key denial, separate active
  and safety identities, encrypted marker scans, restore/restart, Force Quit,
  locked-Keychain refusal/nonmutation, and deletion/rekey with the developer
  profile and shared backup preserved. The final signed build passes installed
  and moved-app startup with the original path absent during relocation, and
  empty-account startup/reopen. Its first test-account launch requested Keychain
  authorization; Always Allow persisted through reopen without another prompt.
  The canonical gate and bounded backup mutation campaign pass. Native helper
  evidence and user-observed installed evidence remain explicitly distinguished.
- implemented locally: narrow vault abstraction, overwrite-safe database-key
  lifecycle, pinned SQLCipher build, schema v1, structured resume validation,
  optimistic draft revisions, immutable published snapshots, settings, integrity
  checks, bounded non-sensitive diagnostics, encrypted WAL crash recovery,
  corruption/newer-schema refusal, checksummed migration v1, verified encrypted
  same-device checkpoints, and a password-protected portable backup/restore
  prototype that creates a fresh device key, plus a verified arm64 macOS local
  preview `.app`/DMG with an isolated identity and explicit ad-hoc signing;
- M1's current-scope signoff is complete. Sustained release fuzzing and broader
  distribution/preview/native-host matrices are not claimed by this local
  development checkpoint. Windows and
  Intel native qualification remain deferred.

Deliver:

- **[HIGH]** OS vault abstraction and database-key lifecycle;
- **[HIGH]** macOS vault-boundary matrix, including cross-account denial and
  desktop/native-host access behavior across locally self-signed development,
  preview, moved, and updated builds; Windows same-user proof is deferred;
- **[HIGH]** SQLCipher schema v1, migrations, repositories, transactions, and
  startup recovery;
- profile, master draft, published snapshot, settings, and diagnostic records;
- structured resume domain validation and optimistic draft revisions;
- **[HIGH]** encrypted backup container prototype with create/inspect/restore
  tests (the implemented same-device checkpoint is a migration/recovery
  primitive, not the portable cross-device container).

Exit evidence:

- **[HIGH]** synthetic resume survives restart and cannot be read from the
  database/WAL without the key;
- **[HIGH]** vault namespace/cross-account/cross-process tests match the
  documented macOS boundary without plaintext fallback;
- **[HIGH]** vault-unavailable and corrupt-database paths are safe and actionable;
- **[HIGH]** migration and backup corruption suites pass.

## M2 — complete offline resume path

### Completion scope decision (2026-09-04)

M2 is complete only when the whole manual master-resume path in
`../../Product Plans/Resume_Editor_and_Schema.md` is implemented. This includes
the build/import starting choice, optional starting profiles, navigable
document-centered editor, focused editing, the specified entry/date/link and
validation behavior, and all three initial selectable document style
categories. The Technical/Engineering, Professional/Business, and
Modern/Marketing and Sales styles require qualified PDF and DOCX output; the
plain development fixtures do not satisfy that gate.

Historical rendering follows the product retention boundary. ORT retains
immutable structured published/tracker sources, not superseded master drafts or
old executable renderer/font bundles solely for replay. When an original tuple
is unavailable, M2 must regenerate the retained structured source with the
current supported tuple and label the effective tuple truthfully. Exact-byte
replay may be claimed only while the original tuple is installed and the full
receipt matches. This replaces the earlier roadmap wording that required
superseded-draft and superseded-renderer-binary replay.

### Current status

**2026-09-07 completion update:** M0/M1 remain complete and successful.
Step 5 implementation is complete for macOS arm64, including the metered DOCX/PDF
runtime, signed disposable helper, native picker/review/cancellation flow,
empty-profile import and atomic import audit. The complete signed candidate is
assembled; its nested helper passed exact running-code identity, cancellation,
parent-death and real 60-second input/output-stall tests. Full local checks pass.
The new desktop import path is enabled only in a build with both helper identity
pins; unbundled builds and the superseded native-parser path remain disabled.

Step 6 final native/user acceptance remains open, so the full M2 milestone is not
yet signed off. See [implementation completion](../../evidence/0.0.0-dev/m2-implementation-completion.md)
and [Step 6 checklist](../../evidence/0.0.0-dev/m2-final-native-acceptance.md).
The paragraphs below preserve historical checkpoints, not the current completion
status or missing-feature inventory.

Step 5 has started from M1-complete `65518eb`. The manual starting screen,
optional section profiles, contact focus and suggested-section selector are
implemented locally with desktop regression coverage. Import stays unavailable.
See `../../evidence/0.0.0-dev/m2-manual-start.md`. The export-style contract and
bundled PDF/DOCX foundation are also implemented and locally tested, with unchanged
plain golden output and strict style/replay receipts; see
`../../evidence/0.0.0-dev/m2-style-foundation.md`. The selector now defaults to
Technical/Engineering, applies to PDF/DOCX, and labels existing previews/history
using their actual styles; 77 desktop tests pass. Final style layout/native
qualification remain open. The HIGH v2 date/link schema foundation now supports
lossless explicit upgrades, mixed-version backups and unchanged historical
receipts; see `../../evidence/0.0.0-dev/m2-schema-v2-foundation.md`. The explicit
editor upgrade, structured date controls and stable link ordering are now locally
implemented with 80 desktop tests; see
`../../evidence/0.0.0-dev/m2-date-link-editor.md`. New empty documents still use v1
until the user enables the upgrade. A collapsible section navigator, full live
HTML reading view and contact/section focused panel are now locally implemented
with 81 desktop tests; see `../../evidence/0.0.0-dev/m2-focused-editor.md`.
Entry collapse, duplication with fresh nested IDs, confirmed entry/section
removal and keyboard focus now pass 84 desktop tests; see
`../../evidence/0.0.0-dev/m2-entry-editing.md`. Exact-field validation navigation
and bullet duplication now pass 85 desktop tests; see
`../../evidence/0.0.0-dev/m2-validation-navigation.md`. Explicit current-renderer
regeneration now passes 204 Rust, 87 desktop and 24 contract tests, with retained
source identity checks and persistent substitution labels; see
`../../evidence/0.0.0-dev/m2-current-renderer-regeneration.md`. Native qualification,
final styles and exact page-layout integration remain open. Step 5's first
subsection is not complete. Reading-entry selection, optional-detail disclosure,
hidden-field error focus and advisory matching-entry hints now pass 91 desktop
and 24 contract tests; see `../../evidence/0.0.0-dev/m2-reading-details.md`.
The expanded style audit now passes 48 v1/v2 PDF/DOCX/text pairs with reviewed
local regression hashes and 24 LibreOffice DOCX reader checks; see
`../../evidence/0.0.0-dev/m2-style-parity.md`. CI runs the new corpus, with hosted
results pending. Native fonts/accessibility, final presentation and exact editor
pagination remain open. Import and lifecycle boundaries remain High-routed.
The standalone native output reader now passes real-pipe sanitizer tests and
has a macOS CI regression. Resource probes found that a 512 MiB `RLIMIT_AS`
was rejected and an ignored `SIGXCPU` outlived a one-second hard CPU setting;
plain rlimits therefore have not qualified these containment requirements.
See `../../evidence/0.0.0-dev/m2-native-output-and-resources.md`. The component
is not a production XPC adapter, and import remains disabled.
A Rust platform pipe driver now emits the existing supervisor event types,
uses its shared byte/timing ceilings and passes real-pipe regression tests
without adding unsafe Rust; see
`../../evidence/0.0.0-dev/m2-rust-native-pipes.md`. Production XPC launch, hard
resource bounds, process-tree cleanup and import integration remain open.
Reading-view add-entry actions, direct untitled-entry editing and a navigator
that releases its column are now implemented with 92 desktop tests; see
`../../evidence/0.0.0-dev/m2-reading-actions.md`. Exact saved-revision PDF pages
now share the center pane with continuous scrolling, fit-width zoom, opt-in
refresh after saves and late-result discard. The full local gate passes 101
desktop tests, with separate real PDF.js browser checks; see
`../../evidence/0.0.0-dev/m2-center-pdf-preview.md`. Installed WKWebView/VoiceOver
and lifecycle qualification remain High-routed and pending. Discarding a pending
preview waits for native rendering to return; it does not interrupt Typst.
The subsequent Medium presentation pass places recovery/storage below the editor,
adds keyboard-focusable area shortcuts and discloses text/Word export controls.
The 101-test full local gate and synthetic browser layout checks pass; details
are appended to `../../evidence/0.0.0-dev/m2-reading-actions.md`.
Section-specific entry labels, example placeholders and optional-detail
shortcuts now pass the 102-desktop-test full local gate. Browser checks cover
the picker layout and blank-value focus; see
`../../evidence/0.0.0-dev/m2-entry-guidance.md`. Existing content and schema stay
unchanged when section guidance changes. Step 5 continues with the production
worker/import adapter and lifecycle/cleanup implementation. Focused native
checks validate implementation fixes; the full native qualification matrix
belongs to Step 6 and has not begun.

The import review presentation now supports explicit per-block decisions,
original-text comparison, editable classification, existing/new/proposed section
destinations and contact conflict choices. Six new interaction/accessibility
tests bring the full local gate to 108 desktop tests. This component is not
mounted in production and does not save data; native review contracts, lifecycle
and revision-checked commit integration remain open. See
`../../evidence/0.0.0-dev/m2-import-review-ui.md`. Import stays disabled.

The native application layer now owns a single review session with separate
owner/token checks, fixed expiry, teardown and revision-checked commit lifecycle.
Five regressions include real synthetic encrypted-storage races and failures
reported after commit. Workspace Rust tests, strict Clippy and the full frontend
gate pass; see `../../evidence/0.0.0-dev/m2-import-review-sessions.md` for exact
test scope. Desktop IPC, cleanup timer and UI/storage wiring remain pending;
this component does not establish parser containment or enable import.

The review panel now submits a generated decision-only payload with a bounded
native decoder and atomic batch replacement behind the review owner's session
checks. A shared frontend/Rust fixture verifies mapping; adversarial cases cover
unknown fields, malformed destinations and resource limits. See
`../../evidence/0.0.0-dev/m2-import-review-contract.md`. Command registration,
proposal-response contracts and native dispatch/lifecycle wiring remain open.

The next checkpoint registers gated native apply/cancel handlers and connects
review state to storage teardown, restore/rollback, main-window destruction,
exit and an idle-expiry worker. Apply uses the real revision-CAS save under the
shared operation lease. Import remains disabled; production proposal/begin/read
and frontend integration remain open. Exact tests and limits are recorded in
`../../evidence/0.0.0-dev/m2-import-native-dispatch.md`.

Native retained-source proposal snapshots, a gated read handler, validated
frontend clients and the review-flow controller now connect the proposal and
apply/cancel surfaces. The flow blocks blind retries after uncertain saves.
The full local gate passes 111 desktop and 26 contract tests; see
`../../evidence/0.0.0-dev/m2-import-proposal-flow.md`. The app still cannot start
an import because native parser containment/creation remains unresolved.



macOS export publication now removes and syncs staging names before writing
document bytes, then publishes through no-clobber descriptor-based file cloning.
Focused SIGKILL tests cover empty staging, unlink completion, partial output,
flushed output and completed publication. Unsupported filesystems fail closed;
other platforms retain named staging. This is implementation evidence, not
power-loss, low-disk or full native filesystem qualification; see
`../../evidence/0.0.0-dev/m2-unlinked-export.md`. The staged native candidate
predates this change and the installed M1 app remains unchanged.

As of 2026-09-04, M2 remains incomplete for macOS arm64. The earlier weighted
cross-platform percentage is retired rather than mechanically increased when
Windows and Intel-Mac gates move later; completion is determined by the explicit
active exit gates below.

Portable-backup export, storage inventory, and authenticated backup validation
are committed through `0a706b2`. Restart-staged replacement and retained
safety-copy management are committed in `96fc983`; all-local-data deletion and
initial current-bundle replay are committed in `91e7e54`; expanded output and
accessibility checks are committed through `9a14985`; retained-publication
replay and dependency policy are committed in `1087aef`, for which the user
reported all five hosted CI jobs passing. Portable archived-source replay was
committed in `bc0712f`; the supervision core, source envelope, private staging,
and inert DOCX/PDF parser checkpoints followed through `7a8cda1`. M2 is not
macOS-arm64 preview-ready. Required native macOS evidence remains pending;
Windows and Intel-Mac evidence is deferred rather than treated as done.
The bounded **current-bundle verified render-history replay** checkpoint now
regenerates an exact current draft or any retained immutable published revision,
verifies the complete receipt before exposing bytes, supplies bounded accessible
text for retained sources, and permits exact-byte export from that verified
preview. An authenticated format-1.1 backup can now provide a bounded,
ten-minute, read-only source session for the same verification without restoring
or mutating the active profile. Explicit current-renderer regeneration of retained
sources is now implemented locally with source-identity checks and substitution
labels; its native acceptance remains an M2 gate. Superseded draft retention and old renderer-binary bundling
are explicitly outside the approved history model.

### Completed or working locally

- **Editing and lifecycle:** structured editing, validation, autosave,
  publication, undo/redo, conflict recovery, published review, and guarded quit.
- **Output and history:** bounded text and DOCX export, pinned local PDF
  rendering/preview/export, accessible text preview, and encrypted render-history
  persistence with portable-backup compatibility.
- **[HIGH]** **Import foundations:** bounded extraction decoding, conservative
  No-AI mapping/review logic, revision-safe storage integration, parent-side
  no-follow/bounded source snapshots, PDF/DOCX envelope preflight, Unix private
  operation staging, bounded parser-output construction, transport policy, and a
  fail-closed cross-platform production supervision
  coordinator/adapter contract, constrained disabled DOCX and pinned PDFium
  text parsers, and
  partial macOS containment/lifecycle probes. The native macOS adapter, PDFium
  packaging, production parser invocation and import UI remain disabled.
  Windows private staging and its native adapter are deferred.
- **[HIGH]** **Backup and recovery:** encrypted portable export, authenticated
  read-only validation, confirmed restart-staged replacement,
  retained safety-copy status/rollback, exact confirmed safety-copy cleanup, and
  crash-resumable deletion/re-keying of all currently implemented local profile
  and recovery data.
- **Storage reporting:** content-free storage usage inventory.
- **Verified retained-source replay:** exact current-draft and retained
  immutable-published receipts from either the active encrypted profile or an
  authenticated portable backup can be regenerated with the installed renderer,
  reviewed accessibly, and exported only when document, PDF, template, font and
  renderer receipt fields all match. Portable sources and passphrases are held
  only in a bounded memory session and never merged into the active profile.
- **[HIGH]** **Storage and restart safety:** no-clobber native file boundaries,
  encrypted restart tests, and Windows SQLCipher logging mitigation.
- **Reliability:** generated contracts, deterministic output/golden tests, and
  passing full local checks for the latest checkpoint. Dependency scanning is
  isolated from the build/test job and uses a SHA-pinned OSV full scan of both
  JavaScript and Rust lockfiles. Exact reviewed exceptions expire on 2026-12-04;
  a repository test prevents broad/package exceptions, changed IDs, missing
  reasons, or a disabled failure gate. All other findings and scanner failures
  remain blocking.
- **Output-only golden corpus:** one shared eight-case synthetic source set now
  pins DOCX, PDF and plain-text bytes and verifies exact cross-format text,
  semantic ordering, omitted optional data, safe HTTP/HTTPS/mailto links,
  supported multilingual content, literal code-like text, PDF structure tags,
  fixed one/two/four-page boundaries, active-content absence, and rendered-page
  layout. The DOCX accessibility audit reports zero findings across all cases.
  This does not enable the HIGH-tagged import path or replace native reader,
  assistive-technology, or final-template qualification. Later platforms must
  repeat the corpus and native-reader checks before qualification.
- **Desktop accessibility automation:** axe-core semantic checks cover the full
  reachable main/overlay routes, PDF preview, quit decision, backup/recovery,
  storage/deletion, and populated published-review surfaces, with a failing
  positive control. Live component checks cover the loaded editor, associated
  validation errors, announced revision conflict, quit-dialog focus restoration,
  exact destructive confirmation feedback, recovery/deletion busy states, and
  deletion-outcome focus. Native macOS assistive-technology, interruption, and
  interaction matrices remain pending; later-platform matrices are deferred.

### Detailed checkpoint evidence

- Native macOS termination now has a deferred AppKit bridge with isolated
  cancellation/approval, failure, and Tauri event-loop evidence. Installed Dock
  Quit/logout/shutdown and the full editor interruption matrix remain open;
  see [native termination](../../evidence/0.0.0-dev/m2-native-termination.md).
  A separately staged, locally signed
  [native editor candidate](../../evidence/0.0.0-dev/m2-native-editor-candidate.md)
  now passes build/signature and workspace checks. The user reports the focused
  standard-account invalid-edit Dock Quit/cancel/discard/reopen checks passed,
  with an initial Keychain password prompt. This is Step 5 implementation
  evidence, not Step 6 signoff; the installed M1 app remains unchanged.
- Editor and lifecycle: [editor smoke](../../evidence/0.0.0-dev/m2-editor-smoke.md),
  [close guard](../../evidence/0.0.0-dev/m2-close-guard-smoke.md), and
  [text export](../../evidence/0.0.0-dev/m2-text-export-smoke.md).
- Import security: [import core](../../evidence/0.0.0-dev/m2-import-core.md),
  [source envelope](../../evidence/0.0.0-dev/m2-import-source-envelope.md),
  [private staging](../../evidence/0.0.0-dev/m2-import-private-staging.md),
  [transport](../../evidence/0.0.0-dev/m2-import-transport.md),
  [production supervision core](../../evidence/0.0.0-dev/m2-parser-supervision-core.md),
  [sandbox probe](../../evidence/0.0.0-dev/m2-native-sandbox-probe.md),
  [hard limits](../../evidence/0.0.0-dev/m2-macos-hard-limits.md), and
  [worker lifecycle](../../evidence/0.0.0-dev/m2-macos-lifecycle.md).
- Documents and rendering: [DOCX](../../evidence/0.0.0-dev/m2-docx-export.md),
  [Windows checkout repair](../../evidence/0.0.0-dev/windows-docx-checkout.md),
  [PDF preview](../../evidence/0.0.0-dev/m2-pdf-preview.md),
  [expanded output golden corpus](../../evidence/0.0.0-dev/m2-output-golden-corpus.md),
  [installed PDF smoke](../../evidence/0.0.0-dev/m2-installed-pdf-smoke.md),
  [render history](../../evidence/0.0.0-dev/m2-render-history.md),
  [portable render history](../../evidence/0.0.0-dev/m2-portable-render-history.md),
  [verified current-bundle replay](../../evidence/0.0.0-dev/m2-current-render-replay.md),
  [retained published replay](../../evidence/0.0.0-dev/m2-retained-published-replay.md),
  and [portable archived-source replay](../../evidence/0.0.0-dev/m2-portable-archived-source-replay.md).
- Accessibility automation: [desktop semantic and destructive-state checks](../../evidence/0.0.0-dev/m2-desktop-accessibility-automation.md).
- CI reliability: [lockfile vulnerability scan](../../evidence/0.0.0-dev/m2-ci-dependency-scan.md).
- Storage and backup: [portable export](../../evidence/0.0.0-dev/m2-portable-backup-export.md),
  [storage usage](../../evidence/0.0.0-dev/m2-storage-usage.md),
  [backup validation](../../evidence/0.0.0-dev/m2-backup-validation.md),
  [replace restore](../../evidence/0.0.0-dev/m2-replace-restore.md), and
  [safety-copy management](../../evidence/0.0.0-dev/m2-safety-copy-management.md),
  and [all-local-data deletion](../../evidence/0.0.0-dev/m2-all-local-data-deletion.md).
- Cross-platform repair: [Windows SQLCipher logging](../../evidence/0.0.0-dev/windows-sqlcipher-logging.md).

### Remaining release gates

- **[HIGH]** Complete Step 6 acceptance of the implemented metered-Wasm import
  path in the exact signed candidate. The native rlimit/XPC implementation
  assumption was superseded by ADR 0012; native identity/lifecycle implementation
  and focused qualification now pass, and no plaintext import staging is needed.
- **[HIGH]** Finish native macOS vault, filesystem, quit, low-disk, and injected
  crash/failure verification, including the separate standard-account matrix.
- Complete final human/native-reader and accessibility acceptance of the
  implemented editor, dialogs, three PDF/DOCX styles and current-renderer
  regeneration of retained sources.
- **[HIGH]** Complete native macOS vault, interruption, filesystem and
  VoiceOver/keyboard evidence for all-local-data deletion. Extend its exact
  cleanup inventory when later milestones add credentials, native IPC state,
  workspace records, or ORT-owned import/drag temporary files.
- **[HIGH]** Pass the complete offline journey and all M2 exit evidence.
  Only the signed candidate's pinned metered helper may parse imported files;
  unbundled or identity-invalid builds must refuse import.

Deferred platform-expansion gates, which do not block macOS-arm64 M2, are the
Windows AppContainer/Job adapter and private staging, Windows vault/filesystem/UI
and accessibility matrices, Intel-Mac native dependencies and containment, and
Windows/Intel clean-machine package qualification.

Deliver:

- functional structured editor and publish lifecycle;
- **[HIGH]** local PDF/DOCX extraction, deterministic schema mapping, and
  temporary review staging; optional AI-assisted remapping is enabled with the
  configured backend in M3/M4;
- **[HIGH]** disposable OS-sandboxed parser worker plus deterministic No-AI
  mapping that preserves unfamiliar content as reviewable custom/simple sections;
- pinned Typst preview/PDF renderer, all three qualified PDF/DOCX style
  categories, and constrained plain-text export;
- accessible preview, save dialogs, atomic export, historical renderer metadata,
  and truthfully labeled current-renderer regeneration for retained sources;
- **[HIGH]** destructive storage deletion and full portable export;
- content-free storage usage reporting.

Exit evidence:

- **[HIGH]** critical offline journey passes without network access;
- **[HIGH]** hostile parser fixtures cannot read application data/secrets,
  access network, spawn children, or survive worker termination;
- golden corpus passes semantic, link, pagination, Unicode, and accessibility checks;
- **[HIGH]** local extraction never changes the master record; an AI-backed
  mapping remains unavailable until an AI connection is configured and the user
  confirms transmission.

## M3 — direct AI foundation

Deliver:

- No AI / Direct API connection state;
- **[HIGH]** OS-vault credential setup and lifecycle;
- **[HIGH]** OpenAI, Anthropic, and Gemini adapters behind one port;
- **[HIGH]** versioned model/preset/pricing catalog with independent signature
  verification;
- **[HIGH]** operation/attempt ledger, cancellation, retry, and crash recovery;
- basic streaming;
- **[HIGH]** token/cost normalization and transactional direct-spend
  reservations/caps;
- aggregate AI Monitoring queries, Week/Month/Year/All time token/direct-cost series and totals, secondary breakdowns, CSV/JSON export, date-range clearing, and separate cap resets; attempt rows remain internal accounting/recovery data.

Exit evidence:

- **[HIGH]** provider contract suites and live synthetic probes pass without
  exposing credentials;
- **[HIGH]** cost/cap arithmetic passes boundary and concurrent-dispatch tests;
- **[HIGH]** credentials and seeded content are absent from logs/backups.

## M4 — tailoring, alerts, and application materials

Deliver:

- **[HIGH]** tailoring, cover-letter, and application-answer prompt/schema
  versions;
- **[HIGH]** factual-evidence validator and no more than three user-visible
  change points;
- **[HIGH]** same-call Required Qualification Alert extraction, versioned
  category allowlist, deterministic per-category validation, evidence, bounds,
  persistence, dismissal/ignore/reopen behavior;
- overlay Stage 2 Resume/Cover letter/Answers tabs, required resume-regeneration instruction, resettable question capture, expanded structured editing/PDF preview, and resume/cover-letter PDF Download/drag handoff;
- **[HIGH]** adversarial AI evaluation corpus and preset-specific quality thresholds.

Exit evidence:

- **[HIGH]** no generated claim can enter accepted output without mapped input
  evidence or user entry;
- **[HIGH]** required-versus-preferred and alert false-positive gates pass;
- **[HIGH]** alerts remain informational and non-blocking.

## M5 — workspace, tracker, and browser bridge

Deliver:

- **[HIGH]** workspace/tracker state transitions and the atomic persistent
  `Finish Application` transaction;
- basic overlay Stage 1 capture/review UI;
- application snapshots, search/filter, and reopen behavior;
- **[HIGH]** Chrome/Edge MV3 extension/native-host authenticated IPC and
  install/repair state;
- basic extension/native-host status UI;
- **[HIGH]** overlay launch and capture review with version-skew handling,
  default extension-action/shortcut gesture flow, and separately gated
  optional-permission overlay-initiation experiment.

Exit evidence:

- **[HIGH]** selected-text-to-workspace journey passes on both browsers and
  operating systems without crossing the documented authority boundary;
- **[HIGH]** malicious page, spoofed client, replay, oversized frame,
  desktop-absent, repair, and uninstall tests pass;
- **[HIGH]** capture never triggers AI automatically.

## M6 — optional external Codex

Deliver only if the security gate passes:

**[HIGH] — Entire milestone.** Every M6 implementation, test, review, and
exit-gate decision crosses the external-runtime containment boundary and
requires high reasoning.

- strict official-runtime discovery/provenance verification (a user-selected path cannot waive identity checks), version/capability negotiation, and isolated ORT Codex home;
- managed ChatGPT/device-code sign-in and keyring use through the external runtime;
- app-server `stdio` adapter, lifecycle, cancellation, account/rate-limit snapshots, and quota threshold controls;
- supported-version matrix, safe disablement, and update guidance;
- platform containment implementation and evidence.

Exit evidence:

- all Codex security requirements in `Security_and_Threat_Model.md` pass on Windows and macOS;
- no tool/file/command event is accepted;
- experimental capabilities remain disabled and any command/process/filesystem/tool/permission/approval/elicitation surface kills the contained child;
- unsupported versions and containment failures disable only Codex mode.

If the gate fails, record the result and defer M6 without blocking M7.

## M7 — distribution and stable hardening

Deliver:

- **[HIGH]** Windows NSIS preview and SignPath-signed direct stable pipeline;
- Microsoft Store fallback feasibility/package path;
- unsigned macOS preview DMG and later-signing readiness;
- **[HIGH]** signed updater metadata, release channels, rollback/recovery,
  checksums, SBOM, and provenance;
- **[HIGH]** extension Store packages and compatibility sequencing;
- **[HIGH]** accessibility manual matrix, performance budgets, and clean-machine
  install/update/repair/uninstall tests;
- public support/diagnostic and release runbooks.

Exit evidence:

- **[HIGH]** all stable release gates in
  `Quality_Accessibility_and_Verification.md` pass;
- **[HIGH]** published artifacts are byte-for-byte the tested artifacts;
- download pages can be generated from the signed release manifest.

## M8 — static project website

The private website plan may be implemented once real release metadata exists. It includes public product/docs/download/support/legal pages and no resume upload, account, hosted AI, or backend user-data service.

## Cross-milestone rules

- **[HIGH]** Each milestone ships behind usable local data migrations;
  unfinished features remain absent or clearly disabled.
- **[HIGH]** Database migrations are forward-only in production. Rollback
  restores a pre-migration safety copy when compatible rather than attempting
  risky down-migrations.
- **[HIGH]** A new renderer/template/prompt/catalog version is immutable after
  release; fixes create a new version.
- **[HIGH]** Security gate design and sign-off are part of the feature, not
  cleanup work;
- Routine accessibility and license implementation;
- **[HIGH]** final accessibility and license release sign-off;
- **[HIGH]** No milestone may add telemetry or remote content storage through
  implementation convenience.

## Explicitly deferred

- mobile and Linux applications;
- cloud sync, accounts, collaboration, hosted keys, subscriptions, or server-side resume storage;
- locally hosted language models;
- Safari/Firefox extensions;
- automatic submission to job sites;
- macOS signing/notarization until the approved traction trigger;
- additional brand themes or dark mode; the approved single light aesthetic and three initial document style categories remain release scope.

## Work-item template

Every implementation issue should state:

1. product requirement and technical-plan section;
2. user-visible outcome and non-goals;
3. records/contracts touched and migration impact;
4. trust boundary and permission impact;
5. failure/cancellation/recovery behavior;
6. automated and manual acceptance tests;
7. evidence artifact and rollout/rollback plan.
