# Technical delivery roadmap

## Status

- Status: implementation sequence approved; dates intentionally unset
- Owner: maintainers
- Planning unit: demonstrable vertical milestone, not percentage completion

The Quiet Navy/Open Frame application and website direction is approved under `../../Aesthetic/`. Early milestones still use semantic, minimally styled controls and a deliberately plain renderer fixture so security, contracts, accessibility, and data behavior stabilize first. Production component polish and the non-default document-template details may be tested and refined during development, but the three promised style categories must pass their functional, accessibility, licensing, and golden-render gates before release.

## M0 — architecture skeleton and contracts

Deliver:

- Cargo/pnpm workspaces, pinned toolchains, Tauri/React shell, and isolated dev profile;
- domain error envelope, command/event transport, generated schemas, and compatibility manifest;
- CI for Windows/macOS build, tests, formatting, license/vulnerability scanning, and schema drift;
- synthetic fixture policy and evidence layout;
- initial architecture decision records for Tauri/Rust, SQLCipher, Typst, native messaging, and external Codex.
- repository and CI skeleton matching `Development_and_Deployment_Outline.md`, including shared desktop source, shared Chrome/Edge extension source, and platform-specific packaging boundaries.

Exit evidence:

- clean checkout bootstrap;
- main and overlay windows can call a typed health command;
- production build has no remote web assets or broad Tauri capabilities;
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
- still release-gated: native macOS and Windows vault matrices, signed-build
  access behavior, platform crash/migration/low-disk suites, cross-platform
  backup files, and expanded hostile restore/fuzz tests.

Deliver:

- OS vault abstraction and database-key lifecycle;
- platform vault-boundary matrix, including Windows same-user limitations and macOS desktop/native-host access behavior across preview, signed, moved, and updated builds;
- SQLCipher schema v1, migrations, repositories, transactions, and startup recovery;
- profile, master draft, published snapshot, settings, and diagnostic records;
- structured resume domain validation and optimistic draft revisions;
- encrypted backup container prototype with create/inspect/restore tests (the
  implemented same-device checkpoint is a migration/recovery primitive, not the
  portable cross-device container).

Exit evidence:

- synthetic resume survives restart and cannot be read from the database/WAL without the key;
- vault namespace/cross-user/cross-process tests match the documented Windows and macOS boundaries without plaintext fallback;
- vault-unavailable and corrupt-database paths are safe and actionable;
- migration and backup corruption suites pass.

## M2 — complete offline resume path

Current development status:

- implemented locally: generated load/save/publish contracts, a main-window-only
  encrypted command boundary, fail-closed OS-vault startup, optimistic draft
  saves, immutable published snapshots, and the first accessible structured
  editor slice for contact details, ordered sections, entries, and bullets;
- still gated: local PDF/DOCX parsing, hostile-worker containment, review
  staging, Typst preview/PDF rendering, DOCX/plain-text export, save dialogs,
  storage management, and complete offline journey evidence.

Deliver:

- functional structured editor and publish lifecycle;
- local PDF/DOCX extraction, deterministic schema mapping, and temporary review staging; optional AI-assisted remapping is enabled with the configured backend in M3/M4;
- disposable OS-sandboxed parser worker plus deterministic No-AI mapping that preserves unfamiliar content as reviewable custom/simple sections;
- pinned Typst preview/PDF renderer and constrained DOCX/plain-text exporters;
- accessible preview, save dialogs, atomic export, and historical renderer metadata;
- storage usage, deletion, and full portable export.

Exit evidence:

- critical offline journey passes without network access;
- hostile parser fixtures cannot read application data/secrets, access network, spawn children, or survive worker termination;
- golden corpus passes semantic, link, pagination, Unicode, and accessibility checks;
- local extraction never changes the master record; an AI-backed mapping remains unavailable until an AI connection is configured and the user confirms transmission.

## M3 — direct AI foundation

Deliver:

- No AI / Direct API connection state and OS-vault credential setup;
- OpenAI, Anthropic, and Gemini adapters behind one port;
- versioned model/preset/pricing catalog with independent signature verification;
- operation/attempt ledger, streaming, cancellation, one retry, crash recovery;
- token/cost normalization and transactional direct-spend reservations/caps;
- aggregate AI Monitoring queries, Week/Month/Year/All time token/direct-cost series and totals, secondary breakdowns, CSV/JSON export, date-range clearing, and separate cap resets; attempt rows remain internal accounting/recovery data.

Exit evidence:

- provider contract suites and live synthetic probes pass;
- cost/cap arithmetic passes boundary and concurrent-dispatch tests;
- credentials and seeded content are absent from logs/backups.

## M4 — tailoring, alerts, and application materials

Deliver:

- tailoring, cover-letter, and application-answer prompt/schema versions;
- factual-evidence validator and no more than three user-visible change points;
- same-call Required Qualification Alert extraction, versioned category allowlist, deterministic per-category validation, evidence, bounds, persistence, dismissal/ignore/reopen behavior;
- overlay Stage 2 Resume/Cover letter/Answers tabs, required resume-regeneration instruction, resettable question capture, expanded structured editing/PDF preview, and resume/cover-letter PDF Download/drag handoff;
- adversarial AI evaluation corpus and preset-specific quality thresholds.

Exit evidence:

- no generated claim can enter accepted output without mapped input evidence or user entry;
- required-versus-preferred and alert false-positive gates pass;
- alerts remain informational and non-blocking.

## M5 — workspace, tracker, and browser bridge

Deliver:

- overlay Stage 1 capture/review, workspace/tracker state machine, and atomic persistent `Finish Application` transaction;
- application snapshots, search/filter, and reopen behavior;
- Chrome/Edge MV3 extension, native host, authenticated IPC, install/repair/status UI;
- overlay launch and capture review with version-skew handling, default extension-action/shortcut gesture flow, and separately gated optional-permission overlay-initiation experiment.

Exit evidence:

- selected-text-to-workspace journey passes on both browsers and operating systems;
- malicious page, spoofed client, replay, oversized frame, desktop-absent, repair, and uninstall tests pass;
- capture never triggers AI automatically.

## M6 — optional external Codex

Deliver only if the security gate passes:

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

- Windows NSIS preview and SignPath-signed direct stable pipeline;
- Microsoft Store fallback feasibility/package path;
- unsigned macOS preview DMG and later-signing readiness;
- signed updater metadata, release channels, rollback/recovery, checksums, SBOM, provenance;
- extension Store packages and compatibility sequencing;
- accessibility manual matrix, performance budgets, clean-machine install/update/repair/uninstall tests;
- public support/diagnostic and release runbooks.

Exit evidence:

- all stable release gates in `Quality_Accessibility_and_Verification.md` pass;
- published artifacts are byte-for-byte the tested artifacts;
- download pages can be generated from the signed release manifest.

## M8 — static project website

The private website plan may be implemented once real release metadata exists. It includes public product/docs/download/support/legal pages and no resume upload, account, hosted AI, or backend user-data service.

## Cross-milestone rules

- Each milestone ships behind usable local data migrations; unfinished features remain absent or clearly disabled.
- Database migrations are forward-only in production. Rollback restores a pre-migration safety copy when compatible rather than attempting risky down-migrations.
- A new renderer/template/prompt/catalog version is immutable after release; fixes create a new version.
- Security, accessibility, and license checks are part of the feature, not cleanup work.
- No milestone may add telemetry or remote content storage through implementation convenience.

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
