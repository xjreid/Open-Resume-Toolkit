# Technical delivery roadmap

## Status

- Status: implementation sequence approved; dates intentionally unset
- Owner: maintainers
- Planning unit: demonstrable vertical milestone, not percentage completion

The Quiet Navy/Open Frame application and website direction is approved under `../../Aesthetic/`. Early milestones still use semantic, minimally styled controls and a deliberately plain renderer fixture so security, contracts, accessibility, and data behavior stabilize first. Production component polish and the non-default document-template details may be tested and refined during development, but the three promised style categories must pass their functional, accessibility, licensing, and golden-render gates before release.

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

Deliver:

- Cargo/pnpm workspaces, pinned toolchains, Tauri/React shell, and isolated dev profile;
- **[HIGH]** domain error envelope, command/event transport, generated schemas,
  and compatibility manifest;
- CI for Windows/macOS build, tests, formatting, license/vulnerability scanning, and schema drift;
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

- implemented locally: narrow vault abstraction, overwrite-safe database-key
  lifecycle, pinned SQLCipher build, schema v1, structured resume validation,
  optimistic draft revisions, immutable published snapshots, settings, integrity
  checks, bounded non-sensitive diagnostics, encrypted WAL crash recovery,
  corruption/newer-schema refusal, checksummed migration v1, verified encrypted
  same-device checkpoints, and a password-protected portable backup/restore
  prototype that creates a fresh device key, plus a verified arm64 macOS local
  preview `.app`/DMG with an isolated identity and explicit ad-hoc signing;
- **[HIGH]** still release-gated: native macOS and Windows vault matrices,
  signed-build access behavior, platform crash/migration/low-disk suites,
  cross-platform backup files, and expanded hostile restore/fuzz tests.

Deliver:

- **[HIGH]** OS vault abstraction and database-key lifecycle;
- **[HIGH]** platform vault-boundary matrix, including Windows same-user
  limitations and macOS desktop/native-host access behavior across preview,
  signed, moved, and updated builds;
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
- **[HIGH]** vault namespace/cross-user/cross-process tests match the documented
  Windows and macOS boundaries without plaintext fallback;
- **[HIGH]** vault-unavailable and corrupt-database paths are safe and actionable;
- **[HIGH]** migration and backup corruption suites pass.

## M2 — complete offline resume path

### Current status

As of 2026-09-03, M2 is **about 66% complete** by stage-gate weighting after
the all-local-data deletion, current-bundle render-replay, desktop accessibility,
and expanded output-golden checkpoints:
approximately 82% of the implementation foundation, 64% of end-to-end
functionality, and 54% of release/exit evidence.
The estimate gives security,
recovery, accessibility, cross-platform behavior, and the complete offline
journey more weight than raw feature count.

Portable-backup export, storage inventory, and authenticated backup validation
are committed through `0a706b2`. Restart-staged replacement and retained
safety-copy management are committed in `96fc983`. M2 is not release-ready.
The all-local-data deletion checkpoint is implemented in the current working
tree based on `104d8aa`; native and cross-platform evidence remains pending.
The bounded medium-reasoning **current-bundle verified render-history replay**
checkpoint is also implemented locally. It regenerates only an exact current
draft/latest-published revision, verifies the complete retained receipt before
exposing bytes, and reports unavailable history without substituting content.
Archived-source and superseded-renderer binary replay remains a later M2 gate.

### Completed or working locally

- **Editing and lifecycle:** structured editing, validation, autosave,
  publication, undo/redo, conflict recovery, published review, and guarded quit.
- **Output and history:** bounded text and DOCX export, pinned local PDF
  rendering/preview/export, accessible text preview, and encrypted render-history
  persistence with portable-backup compatibility.
- **[HIGH]** **Import foundations:** bounded extraction decoding, conservative
  No-AI mapping/review logic, revision-safe storage integration, parent-side
  transport policy, and partial macOS containment/lifecycle probes. Production
  parsing and import UI remain disabled.
- **[HIGH]** **Backup and recovery:** encrypted portable export, authenticated
  read-only validation, confirmed restart-staged replacement,
  retained safety-copy status/rollback, exact confirmed safety-copy cleanup, and
  crash-resumable deletion/re-keying of all currently implemented local profile
  and recovery data.
- **Storage reporting:** content-free storage usage inventory.
- **Verified current-bundle replay:** exact current draft/latest-published
  receipts can be regenerated with the installed renderer and are exposed only
  when document, PDF, template, font and renderer receipt fields all match.
- **[HIGH]** **Storage and restart safety:** no-clobber native file boundaries,
  encrypted restart tests, and Windows SQLCipher logging mitigation.
- **Reliability:** generated contracts, deterministic output/golden tests, and
  passing full local checks for the latest checkpoint. The production dependency
  audit now uses bounded retries for npm advisory-endpoint timeouts while still
  failing on advisories or persistent endpoint unavailability.
- **Output-only golden corpus:** one shared eight-case synthetic source set now
  pins DOCX, PDF and plain-text bytes and verifies exact cross-format text,
  semantic ordering, omitted optional data, safe HTTP/HTTPS/mailto links,
  supported multilingual content, literal code-like text, PDF structure tags,
  fixed one/two/four-page boundaries, active-content absence, and rendered-page
  layout. The DOCX accessibility audit reports zero findings across all cases.
  This does not enable the HIGH-tagged import path or replace native Word,
  assistive-technology, cross-platform, or final-template qualification.
- **Desktop accessibility automation:** axe-core semantic checks cover the
  medium-routed main/overlay, PDF preview, quit-decision and populated published
  review surfaces, with a failing positive control. Live component checks cover
  the loaded editor, associated validation errors, announced revision conflict,
  and quit-dialog focus restoration. HIGH-tagged recovery and deletion panels
  are excluded; native assistive-technology and interaction matrices remain
  pending.

### Detailed checkpoint evidence

- Editor and lifecycle: [editor smoke](../../evidence/0.0.0-dev/m2-editor-smoke.md),
  [close guard](../../evidence/0.0.0-dev/m2-close-guard-smoke.md), and
  [text export](../../evidence/0.0.0-dev/m2-text-export-smoke.md).
- Import security: [import core](../../evidence/0.0.0-dev/m2-import-core.md),
  [transport](../../evidence/0.0.0-dev/m2-import-transport.md),
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
  and [verified current-bundle replay](../../evidence/0.0.0-dev/m2-current-render-replay.md).
- Accessibility automation: [desktop semantic checks](../../evidence/0.0.0-dev/m2-desktop-accessibility-automation.md).
- Storage and backup: [portable export](../../evidence/0.0.0-dev/m2-portable-backup-export.md),
  [storage usage](../../evidence/0.0.0-dev/m2-storage-usage.md),
  [backup validation](../../evidence/0.0.0-dev/m2-backup-validation.md),
  [replace restore](../../evidence/0.0.0-dev/m2-replace-restore.md), and
  [safety-copy management](../../evidence/0.0.0-dev/m2-safety-copy-management.md),
  and [all-local-data deletion](../../evidence/0.0.0-dev/m2-all-local-data-deletion.md).
- Cross-platform repair: [Windows SQLCipher logging](../../evidence/0.0.0-dev/windows-sqlcipher-logging.md).

### Remaining release gates

- **[HIGH]** Complete and prove production PDF/DOCX parser containment on macOS
  and Windows; then integrate the file picker, review UI, private binary staging,
  and richer deterministic mapping.
- **[HIGH]** Finish native/cross-platform vault, filesystem, quit, low-disk, and
  injected crash/failure verification.
- Finish native/cross-platform editor, dialog, accessibility, final PDF/DOCX
  templates, archived-source/superseded-renderer replay, and native reader
  verification.
- **[HIGH]** Complete native macOS/Windows vault, interruption, filesystem and
  assistive-technology evidence for all-local-data deletion. Extend its exact
  cleanup inventory when later milestones add credentials, native IPC state,
  workspace records, or ORT-owned import/drag temporary files.
- **[HIGH]** Pass the complete offline journey and all M2 exit evidence.
  Hostile-file parsing must remain disabled until its containment gate passes.

Deliver:

- functional structured editor and publish lifecycle;
- **[HIGH]** local PDF/DOCX extraction, deterministic schema mapping, and
  temporary review staging; optional AI-assisted remapping is enabled with the
  configured backend in M3/M4;
- **[HIGH]** disposable OS-sandboxed parser worker plus deterministic No-AI
  mapping that preserves unfamiliar content as reviewable custom/simple sections;
- pinned Typst preview/PDF renderer and constrained DOCX/plain-text exporters;
- accessible preview, save dialogs, atomic export, and historical renderer metadata;
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
