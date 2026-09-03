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
  saves, idempotent immutable publication, a structured editor for contact
  details, sections, entries, bullets, links, and named custom/skill fields,
  race-safe debounced autosave, shared-limit inline validation, keyboard
  reordering, bounded session undo/redo, explicit reload/discard recovery, and
  read-only published text review. Native macOS synthetic save/publish/restart
  checks are recorded under `../../evidence/0.0.0-dev/m2-editor-smoke.md`;
- added locally: native-owned single-use quit attempts, main-window-only
  lifecycle commands, and accessible Save/Discard/Keep editing confirmation
  for the main close button and application Quit menu/shortcut, with pending
  operation waits and save-failure recovery. Verified macOS paths and the
  upstream termination gap are in `../../evidence/0.0.0-dev/m2-close-guard-smoke.md`;
- added locally: deterministic bounded UTF-8 text export from an exact saved
  draft revision or latest immutable published snapshot, a Rust-only native Save
  dialog, single-use held-directory destination authority, and no-clobber atomic
  publication. The UI warns that exports are unencrypted; cancellation/export
  failure never changes resume revisions or pauses autosave. Existing-file
  replacement is intentionally unavailable in this checkpoint. Evidence and
  limitations: `../../evidence/0.0.0-dev/m2-text-export-smoke.md`;
- added locally: the backend-only No-AI import-review foundation. A bounded,
  versioned extraction decoder, conservative multilingual heading/explicit
  contact-label mapping, and source-indexed proposals preserve every extracted
  block. In-memory review requires a decision for each block, supports explicit
  section merge/keep-both and contact conflict choices, and prepares a validated
  revision-bound save without modifying storage. Synthetic encrypted-storage
  tests cover commit races, replay refusal, published-snapshot isolation, and
  restart. Evidence: `../../evidence/0.0.0-dev/m2-import-core.md`. No import UI,
  file picker, parser, or worker launch was enabled by this checkpoint;
- added locally: bounded parent-side worker transport policy with capped stdout
  and discarded stderr, monotonic wall deadline, cancellation, terminal failures,
  and both-EOF/successful-exit validation. These are event simulations, not native
  pipe/process/sandbox proof. The implementation candidates and access-denial
  checklist are in `Document_Worker_Containment.md`; evidence is in
  `../../evidence/0.0.0-dev/m2-import-transport.md`;
- added locally: a separately signed synthetic macOS App Sandbox/XPC probe.
  Read-only descriptor transfer, seeded sibling/symlink restrictions and loopback
  denial passed on local arm64. Direct child creation was allowed; cooperative
  disconnect is not forced process-tree cleanup. This is partial evidence, not
  a production sandbox or an import-enablement gate. Both macOS CI jobs run the
  measured subset. See `../../evidence/0.0.0-dev/m2-native-sandbox-probe.md`;
- added locally: helper-only hard limits now deny direct `fork`/`posix_spawn`
  in the macOS probe, enforce a 64-descriptor ceiling with recovery and reject
  raising the hard limits. The parent is unaffected; the plain App Sandbox
  baseline is retained. Memory/CPU/broker/forced-cleanup and Windows containment
  remain unproven. All four CI jobs were subsequently reported passing after `723a97f`.
  See `../../evidence/0.0.0-dev/m2-macos-hard-limits.md`;
- added locally: a separate minimal XPC supervisor/inherited-child lifecycle
  probe. Nine native cases cover normal completion, cancellation, timeouts,
  output floods, invalid/nonzero results and complete output/EOFs without exit.
  The supervisor terminates and reaps its own direct child before accepting
  completion; no XPC PID is signaled. Both macOS CI jobs now run the new subset.
  This does not prove supervisor-death cleanup, broker descendants or the full
  inherited-child authority boundary. Production parser/UI remain disabled.
  See `../../evidence/0.0.0-dev/m2-macos-lifecycle.md`;
- subsequent CI status: the user confirmed all four jobs passing for `e978cfe`
  before the DOCX checkpoint began; no run URL was independently retrieved;
- added locally: a constrained, output-only DOCX generator and main-window
  saved-draft/published-snapshot export integration. Fixed OPC parts, literal
  escaped content, semantic headings/lists and allowlisted links use a versioned
  plain layout. It shares the native one-operation/no-clobber export boundary,
  not the gated hostile-input worker. Synthetic encrypted-restart/file-write,
  negative-content, package/semantic and headless-render checks accompany it.
  The user subsequently reported three of four CI jobs passing for `e349856`;
  the supplied Windows log failed only at the DOCX golden-byte check after
  Rust tests passed. CRLF-converted embedded XML reproduced that failure locally.
  A scoped LF checkout policy, cross-platform checkout regression tests and
  explicit output diagnostics restore the unchanged goldens locally; the repaired
  CI run was subsequently confirmed green on all four jobs by the user for
  `748d13b`; native document-reader/dialog verification remains pending.
  See `../../evidence/0.0.0-dev/windows-docx-checkout.md`.
  See `../../evidence/0.0.0-dev/m2-docx-export.md`;
- added locally: pinned embedded Typst 0.15.1 PDF rendering, six bundled
  Libertinus Serif faces, original plain PDF layout, exact-byte PDF.js 6.3.289
  preview and native no-overwrite PDF export from saved revisions. One bounded
  expiring native preview ticket, SHA-256/version receipts, stale-preview guards,
  shared render/export/quit gate, accessible text view and bundled license notices
  accompany encrypted-restart, hostile-literal, layout-limit, contract, filesystem,
  independent PDF parser/golden and synthetic browser/CSP checks. This is an
  output-only fixed-template integration, not hostile-file containment. All four
  CI jobs for `296610a` were subsequently confirmed
  passing by the user (not independently retrieved). A rebuilt, installed macOS
  arm64 app passed saved-draft native canvas rendering at 100%/150%, accessible
  text expansion and native PDF Save cancellation without changing saved data.
  Broader native WebView/dialog/AT checks remain pending. See
  `../../evidence/0.0.0-dev/m2-pdf-preview.md` and
  `../../evidence/0.0.0-dev/m2-installed-pdf-smoke.md`;
- added locally: additive encrypted schema v2 migration and bounded historical
  PDF render manifests. Successful previews durably record source/revision,
  renderer/template/font identities, hashes, page/byte counts and timestamps
  before the UI receives bytes. Identical render identities deduplicate with a
  count; the newest 100 remain and the UI exposes 20 without PDF bytes, resume
  text, paths or preview tickets. Schema-v1 upgrade and interrupted exact-manifest
  handoff recovery preserve existing records. Portable backup format 1.1 now
  includes the same bounded, content-free manifests, restores them atomically
  into a separately keyed profile, and retains format-1.0 read compatibility.
  Historical renderer replay and native backup/export UX remain pending. See
  `../../evidence/0.0.0-dev/m2-render-history.md` and
  `../../evidence/0.0.0-dev/m2-portable-render-history.md`;
- Windows CI repair passed per user report after `bdc3e10`: stage logs narrowed the
  stack overflow to encrypted-profile opening. A matching upstream SQLCipher
  Windows logging-recursion defect is mitigated by compiling out its native
  diagnostic logger, with a fail-closed build-policy check. Encryption and
  allocation memory protection remain enabled; no test is skipped or given a
  larger stack. All four CI jobs were reported passing; the run URL/log has not
  been independently retrieved. This does not prove Windows vault/UI or parser
  containment. See `../../evidence/0.0.0-dev/windows-sqlcipher-logging.md`;
- still gated: macOS Dock/system-shutdown quit protection, Windows native editor
  verification, local PDF/DOCX parsing, hostile-worker containment, review
  UI/session integration, private binary staging, richer entry/date/link mapping,
  remaining native PDF preview/export verification, final PDF/DOCX templates
  and reader verification, confirmed replacement and crash-cleanup policy for
  exports, historical renderer replay, Windows native Save-dialog/ACL/filesystem
  proof, storage management, and complete offline
  journey evidence. Portable render-manifest backup is implemented, but native
  full-backup export/restore UX and its release verification remain gated. M2 is
  not complete; do not enable hostile-file parsing or
  advance to public release based on the text-export checkpoint alone.

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
