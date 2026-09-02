# Desktop application plan

## Status and requirements

- Status: approved implementation baseline
- Owner: desktop maintainer
- Milestones: M0–M7
- Product authority: `../../Product Plans/Core_Workflows.md`, `Resume_Editor_and_Schema.md`, `Product_States_and_Operations.md`, `AI_and_Import.md`, and `Quality_Accessibility_and_Verification.md`

This plan defines functional structure and interaction contracts only. It expressly does not select the application aesthetic.

## Framework decision

Use Tauri 2 with a Rust backend and a React/TypeScript/Vite frontend.

Why it fits:

- native windows and always-on-top overlay without shipping a full browser runtime;
- a Rust core can share domain, storage, document, IPC, and security logic with the native host;
- Tauri capabilities allow a narrow command surface instead of exposing Node or shell APIs;
- official installer/updater integration exists for the planned Windows/macOS channels;
- the UI remains conventional, testable TypeScript while privileged work stays outside the webview.

The tradeoff is platform WebView variance. Supported Windows builds require WebView2; macOS uses WKWebView. UI and accessibility tests must run on both rather than assuming Chromium-only behavior.

## Window model

### Main window

Owns master-resume creation/import/editing/publishing plus tracker, aggregate AI Monitoring, connection setup, settings, backup, update, and repair. It contains no current-job tailoring/editor route. A single-instance guard routes file-open intents to this window and browser-capture intents to the overlay. Closing behavior follows the OS convention; the app does not stay resident merely to collect browsing data.

### Overlay window

A real Tauri always-on-top window that owns the complete current-application workflow. Stage 1 captures and reviews job text. Stage 2 supplies Resume, Cover letter, and Answers tabs; PDF preview/edit/download/drag controls; change summaries and alerts; answer copy/reset; and persistent Finish Application. It is never injected into a job site and receives only the backend commands needed for that workflow.

Overlay state is durable where user intent matters (`dismissed`, `ignored`) and ephemeral for window position/open state unless the product setting says to remember it. Closing the overlay does not mark an alert ignored and does not cancel work.

### System dialogs and external links

File selection/save uses native dialogs through specific Rust commands. External URLs are normalized, checked against `https` and approved destinations, shown to the user where ambiguity exists, then opened through the OS. No web content renders inside an application webview.

## Functional information architecture

The initial routes/views are:

```text
/start                    local profile and recovery/startup states
/resume                   master draft editor and publish state
/tracker                  application tracker and filters
/tracker/:id              saved application and immutable snapshots
/monitoring               aggregate tokens and direct cost by Week/Month/Year/All time
/settings/ai              No AI / Direct API / Codex connection and controls
/settings/data            storage, backup, restore, export, deletion
/settings/browser         extension/host status and repair
/settings/updates         channel, current version, checks, release information
/settings/accessibility   scaling/reduced motion and OS-derived preferences
/settings/about           GPL/attribution, third-party notices, provenance, official/preview status
```

This route map expresses functionality, not sidebar style or layout.

The About/Legal view renders bundled GPL, copyright/canonical-source attribution, Section 7 additional terms, third-party license notices, trademark-policy links, build commit/provenance, release channel, and signing/preview state. It contains no local identifiers, secrets, or user-content metadata.

## Frontend organization

Each feature owns route components, accessible UI primitives composed from shared basics, query/mutation hooks, view models, and tests. Shared TypeScript code contains:

- generated command/event types and runtime validation;
- a command client that maps Rust error envelopes to typed outcomes;
- operation subscriptions that reconcile after missed events;
- local formatting for dates, token counts, money, and provenance;
- focus restoration, announcement, and keyboard utilities;
- unsaved-change and destructive-confirmation helpers.

Use a small explicit client-state store for window/session state and TanStack Query or an equivalent cache for backend-owned state. Canonical resumes, workspaces, activities, settings, and policies never live only in the frontend store. Forms keep a working copy plus the base revision; saves use optimistic concurrency.

## Backend command surface

The M2 local close-guard checkpoint uses native-owned, single-use attempt IDs
instead of a cached renderer dirty flag. Window close and the app-owned Quit
menu/shortcut pause for the editor's Save/Discard/Keep editing decision. Only the
main window may read or resolve an attempt; an event merely asks it to reread
native state. In-flight mutations must finish before quit, and save failure must
not implicitly discard edits. Missed listener events are reconciled on startup.

The pinned macOS runtime's system termination path (Dock Quit/logout/shutdown)
does not yet expose a cancellable callback. This remains a release gate, not a
claim of complete close protection. The UI warns users to wait for Saved on
these paths. Avoid unsafe runtime method replacement or widening frontend
process privileges to work around it. Follow-up must prove an OS-supported
termination hook or bounded encrypted recovery of unfinished edits, including
invalid forms. Windows needs native close/quit/logoff verification separately.

Tauri command functions:

1. deserialize a generated request type;
2. check window capability and basic size limits;
3. call one application use case;
4. map the domain result to the common response/error envelope.

Commands cannot accept raw SQL, arbitrary filesystem paths, shell commands, provider endpoints, or unbounded template/source text. User-chosen paths are returned by an approved dialog token that is consumed once by the associated import/export command.

The frontend renders resume, job, import, provider, and error content only as escaped text/typed values. Production code forbids raw HTML insertion and string-to-code APIs. PDF preview receives a backend-created opaque resource handle for validated bytes; the internal protocol does not accept a filesystem path, remote host, user-controlled MIME type, traversal segment, or open-ended query. Each resource handle is scoped to a window/session/artifact and expires on replacement, Finish/discard, or bounded cleanup.

## Startup state machine

```text
starting
  -> first_run
  -> vault_locked
  -> migration_required -> migrating -> ready
  -> recovery_required
  -> unsupported_newer_data
  -> ready
```

Startup checks application-data permissions, vault/database access, incomplete operations, migration phase, required renderer assets, and pending capture intent. Network and update checks are not prerequisites for reaching `ready`.

A blocking startup state provides safe diagnostics and approved recovery actions; it never replaces an inaccessible database with an empty profile.

## Resume editor implementation

- Load the current draft and base revision from the backend.
- Edit normalized sections/entries addressed by stable IDs, not array positions.
- Validate field types locally for responsive feedback and authoritatively in Rust before persistence.
- Autosave a debounced snapshot only when locally valid; retain explicit save/retry status.
- Reordering emits an ordered list of IDs and is keyboard-operable.
- Publish invokes a backend transaction that validates the whole document and creates an immutable snapshot.
- Preview requests are revision-addressed and cancellable. A late result for an older revision is discarded.
- Import opens a separate review model; accepting chosen changes creates a normal draft revision with an audit summary.
- Import parsing occurs outside the desktop process in the disposable document worker. The UI can receive only bounded extraction/proposal records and safe worker error codes; it never receives worker filesystem/process authority.
- With No AI selected, the review model shows deterministic local mappings into known fields and proposed custom/simple sections for unfamiliar content. Every extracted block remains accounted for until the user accepts, moves, relabels, merges, keeps, or rejects it.

The UI never directly edits rendered Typst, PDF, or DOCX source. Theme/template controls use known IDs and structured parameters. Document templates are isolated from ORT application branding; the default Technical template adapter implements the approved Jake's Resume-derived professional structure subject to the recorded source/license gate.

## Workspace and application flow

The overlay implements an explicit state machine:

- **Stage 1 / capture:** Capture job description; receive selected text; review/edit/remove URL; Capture again; Continue. Continue is disabled for empty text and is the only action that starts initial tailoring.
- **Stage 2 / Resume:** show the current PDF card, no more than three sharp change points, qualification alerts, Preview and edit, Download, drag handle, and Regenerate resume. Regeneration requires a nonempty correction instruction.
- **Stage 2 / Cover letter:** begin with Generate cover letter; after success show the same PDF-card controls and expanded structured editor/PDF preview.
- **Stage 2 / Answers:** Capture question; review/edit; Generate answer; edit/copy output; Reset and capture new question.
- **Persistent Stage 2 action:** Finish Application remains visible independent of the selected tab and optionally commits selected materials to the tracker before resetting to Stage 1.

Expanded preview/edit grows the overlay substantially and places the structured editor beside a large PDF preview. The PDF binary is never edited directly. Resume and cover-letter PDF cards each expose both a keyboard-accessible Download action and a pointer drag source; drag materializes a private temporary file. If a browser rejects a drop, the UI keeps Download and save/open-folder guidance available.

The UI always distinguishes source facts, AI proposals, and user-accepted content. Switching tabs or closing/reopening the overlay does not destroy work; the activity record and local workspace reconnect it.

`Finish Application` first validates current artifacts, then submits one transaction command. It does not claim success until tracker entry and immutable snapshots commit. Export can occur before or after finishing but is not itself tracker persistence.

## Required Qualification Alerts

Alerts are informational and never block editing, export, or finishing an application.

Each row/view model contains:

- `confirmed_mismatch` or `not_found`;
- normalized required-qualification category;
- exact requirement excerpt/span and mandatory-classification basis;
- cited resume field/entry evidence for confirmed mismatches;
- concise explanation;
- presentation state: `active`, `dismissed`, or `ignored`.

An alert surface can be closed without changing record state. `Dismiss` hides an individual alert for the current workspace view; `Ignore` records an explicit decision while leaving it discoverable under “Show ignored.” All alerts can be reopened. No control auto-edits the resume or asks the model to fabricate a missing qualification.

On generation completion, a polite live-region summary announces the number and types, not the entire alert text. Opening a popover/side region moves focus only in response to a user action. Escape closes it and returns focus to its trigger. Overlay and main window call the same alert queries/commands.

## AI connection settings

The selected connection mode is exactly one of `none`, `direct`, or `codex`.

### Direct

- Add/test/delete one credential per provider through a secret-entry dialog backed by the OS vault.
- The UI receives only key presence, label, last test time, and safe error; it cannot read the secret back.
- Settings explain that the vault protects against offline/other-user access, not malware or another person already controlling the same unlocked OS account. Optional user-presence protection is not promised until native-host, recovery, and accessibility behavior is proven.
- Provider/model selection is derived from the verified catalog and adapter capability result.
- Estimates, billing/privacy links, requested/effective model, spending caps, thresholds, and account caveats are presented before dispatch where required.

### Codex

- Locate supported runtime, choose an executable if automatic discovery fails, verify version/signature/path, and initiate managed ChatGPT/device-code authentication.
- Show runtime compatibility, containment status, account state, usage snapshot age/provenance, model intersection, and quota threshold controls.
- Do not show Codex as available when the platform containment gate is not satisfied.
- Removal disconnects ORT's isolated configuration/reference without uninstalling Codex or affecting unrelated Codex use.

Switching modes cancels no active request silently. If an operation is running, require it to complete/cancel before the new mode becomes active.

## AI Monitoring

The primary view queries backend-computed aggregates, not a paginated call list. A Week/Month/Year/All time period control drives a token time series, a direct estimated-cost time series, and selected-period totals. Week/month use daily buckets; year/all-time use monthly buckets unless the backend returns a documented adaptive all-time bucket. Graphs have accessible tabular/text equivalents.

Secondary breakdowns can group by provider, effective model/preset, operation type, and status. Completeness, currency, catalog version, estimate provenance, retries, missing usage, and Codex account/quota provenance remain visible in aggregate explanations. Codex activity is never assigned an invented dollar amount or added to direct cost.

Attempt records remain backend accounting/recovery data and may enter an explicitly requested scrubbed diagnostic export; no individual-call route or primary call table ships. Ordinary CSV/JSON export contains selected aggregate buckets and breakdowns. Clearing by date range and resetting direct/Codex guardrails use separate commands and confirmations.

## Progress, cancellation, and errors

All long tasks share the operation lifecycle from the system architecture. The UI offers cancellation only when the backend says it is meaningful. Cancellation is best effort: the final state may be `completed`, `cancelled`, or `outcome_unknown` depending on provider timing.

Errors show a plain-language summary, safe support code, retry eligibility, and recovery action. Raw stack traces/provider bodies remain out of UI. Offline errors never imply local editing/export is unavailable.

## Accessibility requirements

- Native keyboard order follows the functional reading order; every action is reachable without pointer or drag.
- Drag reorder has move-up/down and position controls with announcements.
- Form fields have programmatic labels, descriptions, error association, and status regions.
- Headings/landmarks identify overlay stage, tabs, editor, alerts, preview, monitoring, and settings regions.
- Focus is restored after dialogs/popovers and intentionally placed after route-changing actions.
- Screen-reader output does not expose hidden resume sections or repeat streaming tokens continuously.
- OS scaling, text zoom, high-contrast/forced-colors, and reduced motion are supported structurally. ORT ships one light color scheme; forced-colors support is an accessibility behavior, not a second theme.
- Minimum target size and non-color status indicators are enforced by automated component tests later, independent of the final visual theme.

Manual release coverage includes NVDA with Windows WebView2 and VoiceOver with WKWebView, keyboard-only use, 200% text scaling, and reduced motion.

## Performance and resource behavior

- Initial ready view should not await renderer, extension, provider, Codex, or updater initialization.
- Lists are paginated/virtualized only where measured; accessibility cannot be sacrificed for premature virtualization.
- Autosave and preview are debounced independently.
- Only one remote AI operation can execute per local profile.
- Large parsing/rendering runs outside the UI thread and exposes cancellable phase progress.
- Memory tests cover a maximum-size import, long tracker history, and repeated preview cycles.

## Tests

- unit: reducers/view models, validation summaries, focus restoration, formatters;
- component: forms, conflict handling, alerts, activity provenance, confirmations;
- contract: every command/event against generated schemas and error envelope;
- integration: Tauri command to temporary encrypted repositories and mocked adapters;
- end-to-end: first run, draft/publish, import review, preview/export, capture/tailor/alert/finish, backup/restore, connection changes;
- failure: vault locked, stale revision, provider timeout, process restart, low disk, renderer failure, incompatible Codex/extension/update;
- accessibility: axe-style checks plus the manual platform matrix.

## Rollout and completion

Offline routes are enabled first. Direct AI, extension, updater, and Codex appear only when their backend capability reports the relevant gate as passed. Disabled capability explanations are local and actionable; they do not link users into unsupported workarounds.

Complete when all critical journeys work in keyboard and screen-reader testing on both platforms, state survives restart/crash as specified, no webview has broad privileged capability, and functional behavior is independent of the later aesthetic layer.
