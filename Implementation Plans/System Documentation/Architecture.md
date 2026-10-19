# System architecture

## Status and authority

- Status: implemented development architecture with planned release/platform adapters
- Owner: core maintainers
- First milestone: M0 architecture skeleton through M7 release hardening
- Product authority: all files in `../../Product Plans/`, especially `Product_Scope_and_Principles.md`, `Local_Data_and_Document_Model.md`, `Product_States_and_Operations.md`, and `Security_Privacy_and_Open_Source.md`

This document records the architecture and remaining release design. The [implementation map](../../docs/development/current-state.md) identifies working adapters and open qualification. It does not define visual styling.

## Architectural goals

1. The desktop application remains useful without an account, extension, AI connection, or network access.
2. Canonical content and credentials remain local; deliberate AI transmission and the separately opted-in planned M5 answer feature disclose only authorized content to the chosen provider or foreground application website. Browser filling does not imply that the website waits for submission to read a value.
3. Every durable record has one owner and one versioned schema.
4. The same structured document produces preview and export output through the same renderer.
5. External input is untrusted at every boundary: job pages, imported files, AI output, update metadata, backups, and extension messages.
6. UI crashes, provider failures, extension failure, and interrupted updates cannot corrupt the last committed user state.

## Runtime topology

The diagram describes existing capture. The distinct planned M5 answer monitor,
popup, library service, and authorized desktop-to-field response path are defined
in the [autofill plan](../Browser%20Extensions/Application_Answer_Autofill_Plan.md).

```text
Chrome / Edge page
    | selected text + sanitized URL, explicit user action only
    v
MV3 extension service worker
    | browser native-messaging framing
    v
ort-native-host (short-lived Rust process)
    | authenticated named pipe / Unix-domain socket
    v
Tauri desktop process
    +-- bundled React UI windows
    +-- Rust application services
    +-- SQLCipher database
    +-- OS credential vault
    +-- document/import workers
    +-- direct-provider HTTPS adapters
    +-- optional external `codex app-server` child over stdio
```

The public GitHub repository is the primary home for project information, documentation, support, and releases. GitHub Releases distribute verified artifacts and release metadata. Neither receives resume content from the application.

## Process boundaries

### Desktop process

The Tauri process owns all authoritative state and privileged operations. Bundled web UI code can request typed commands but cannot directly access the filesystem, database, network, shell, credential vault, native-messaging registration, or updater.

Long-running work runs as cancellable Rust tasks behind an operation coordinator. Rendering and trusted CPU-heavy work may use a bounded internal worker pool. Untrusted PDF/DOCX parsing never runs in the desktop process: the packaged, identity-pinned `ort-parser-helper` runs constrained metered-Wasm guests and returns a bounded extraction record. ADR 0012 supersedes the older native-worker/XPC design. A stable OS profile lock is acquired before recovery and retained for the process lifetime. Secondary launches refuse profile access; intent forwarding/activation is not implemented. Tracker reads use the bounded background-work facility and a canonical search projection.

### Hostile document worker

The following native-worker description is the legacy design. The active macOS
import path is the pinned helper/metered-Wasm architecture in
[ADR 0012](../../docs/adr/0012-metered-wasm-document-parsers.md), with explicit
review before save. The legacy process must not be enabled as a parser fallback.


`ort-document-worker` has no database, vault, provider, updater, browser, or native-IPC code dependency. The parent opens a validated staged input, launches the worker with an inherited handle or containment-verified path, and grants a single private output location. Platform policy denies all network, child processes, unrelated file reads/writes, application data, user documents, and secret stores. The parent enforces memory/CPU/wall-time/handle limits, validates the extraction result again, and kills the complete process tree after every outcome. A worker failure cannot commit canonical state.

### UI webviews

There are two application windows: the main window and an always-on-top application overlay. Both load only bundled assets under a restrictive content-security policy and share a typed client library with separate view state. The main window owns master-resume authoring and supporting tracker/monitoring/settings functions; it has no job-specific workspace route. The overlay owns the complete two-stage application workflow. Closing it never cancels or mutates the underlying workspace.

### Native host

`ort-native-host` is a separate, small executable installed beside the desktop application. It validates bounded browser native-messaging frames and forwards requests over protected local IPC. The current Chrome development path keeps a native port for serialized heartbeat/capture commands until disconnect; it is not a one-request host. It does not open the database or hold content on disk.

### External Codex runtime

Codex is not packaged in ORT. When enabled by the user, ORT launches a supported external executable as an app-server child using `stdio`, an isolated ORT-specific Codex home, a non-user-content working directory, and an allowlisted protocol surface. Direct API mode and Codex mode are mutually exclusive. The user starts/stops the server explicitly; launch resets server permission and memory-only sign-in.

Codex support is a gated adapter, not a core dependency. Failure to locate, authenticate, update, or safely contain Codex disables only that connection mode.

## Layering and dependency direction

```text
UI / extension adapters / CLI test harnesses
                 |
                 v
Application use cases and operation coordinator
                 |
                 v
Domain types, validation, policy, and state machines
                 ^
                 |
Storage / vault / provider / Codex / renderer / OS adapters
```

Rules:

- Domain crates depend on no Tauri, HTTP, SQL, browser, or OS APIs.
- Application use cases depend on traits defined at the domain/application boundary.
- Infrastructure crates implement those traits.
- Tauri commands perform deserialization, authorization, and mapping only; business rules stay in use cases.
- No provider-specific response type crosses the provider-adapter boundary.
- No SQL row type crosses the repository boundary.
- Generated cross-process schemas are checked in and verified for drift in CI.

## Primary domain aggregates

| Aggregate | Authority | Important invariants |
|---|---|---|
| Profile | local database | one active local profile in v1; no remote identity |
| Master resume | one draft plus zero-or-one immutable published snapshot | tailoring reads/copies the published snapshot, never a mutable draft |
| Workspace | overlay stage/tab state, captured job input, generated artifacts, and PDF materializations | one current workspace; one active tailoring operation; user edits remain distinct from AI proposals |
| Tracker entry | application status and structured snapshots | `Finish Application` commits workspace and tracker changes atomically |
| Reusable answer library (planned M5) | separate encrypted profile service | conservative matching/aliases, revisioned corrections, capture/reuse approval, restore Off; never an implicit overlay/tracker import |
| AI operation | logical user request with one or more attempts | every dispatched attempt is persisted before network I/O |
| Guardrail policy | cap configuration, reservations, settlements | direct calls fail closed when cost cannot be bounded under an enabled cap |
| Qualification alert | mismatch/not-found evidence linked to an operation | informational only; dismissal never edits resume facts |
| Backup | encrypted portable snapshot | excludes provider keys, Codex credentials, and device-bound vault material |

## Command, query, and event contracts

UI calls use namespaced commands such as:

- `profile.get`, `profile.update`
- `resume.draft.get`, `resume.draft.save`, `resume.publish`
- `workspace.capture`, `workspace.review`, `workspace.continue`, `workspace.get`, `workspace.finish`
- `workspace.resume.regenerate`, `workspace.letter.generate`, `workspace.answer.capture`, `workspace.answer.generate`, `workspace.answer.reset`
- `document.preview`, `document.download`, `document.drag.materialize`
- `ai.operation.estimate`, `ai.operation.start`, `ai.operation.cancel`
- `ai.monitoring.summary`, `ai.monitoring.series`, `ai.monitoring.breakdown`, `ai.monitoring.export`
- `guardrail.get`, `guardrail.update`, `guardrail.reset`
- `backup.create`, `backup.inspect`, `backup.restore`
- `extension.status`, `extension.repair`
- `update.check`, `update.install`

Every request includes `contractVersion`, `requestId`, and a typed payload. Responses are either `{ok: true, value}` or `{ok: false, error}`. The stable error envelope contains:

```json
{
  "code": "AI_OUTPUT_INVALID",
  "messageKey": "errors.aiOutputInvalid",
  "retryable": false,
  "operationId": "optional-uuid",
  "details": {}
}
```

`details` may contain safe machine-readable fields but never raw credentials, full provider responses, or resume text. UI-visible text is resolved locally from `messageKey`.

Long tasks emit ordered events with `operationId`, monotonic `sequence`, lifecycle state, and bounded progress data. The UI always reconciles against the persisted operation record after reconnecting rather than assuming an event stream is complete.

## Main data flows

### Offline authoring and export

1. UI edits a versioned structured draft through application commands.
2. The domain validator normalizes dates, links, ordering, and stable IDs.
3. Storage commits the new revision transactionally.
4. Publishing creates an immutable snapshot.
5. The document service renders the selected snapshot with a pinned template/font/renderer tuple.
6. Preview receives the PDF bytes; export writes those same bytes through a user-selected safe path.

### Browser capture

1. In the current Chrome development flow, overlay Capture authorizes a two-corner rectangle on the active page; Edge capture remains open M5 work.
2. The content script returns selection and page metadata only to the extension service worker.
3. The extension removes fragments/credentials from the URL, enforces size limits, and sends a versioned native message. It has no parallel review UI.
4. The host authenticates local IPC and delivers a capture intent.
5. The overlay creates or updates Stage 1 and shows the received text for review/edit, Capture again, or Continue. No AI call starts automatically.

The implemented Chrome capture uses declared HTTP/HTTPS host access because an
overlay click cannot grant `activeTab`; the current [extension plan](../Browser%20Extensions/Browser_Extension_and_IPC_Plan.md)
owns its permission/session contract. Host access alone does not authorize reading.

### Reusable application answers — planned M5

The [product authority](../../Product%20Plans/Application_Answer_Autofill.md) adds
a separate application service and main-window library, a foreground isolated-world
monitor, and an extension-owned approval/Undo/stop popup. The service worker owns
browser lifecycle and native transport, not DOM scanning. Versioned v2 lookup,
capture, approval, and revocation messages preserve v1 capture compatibility.

The desktop owns normalization/classification, typed saved answers/aliases,
encrypted ordinary corrections, preferences, and revocable field-bound results.
Content scripts cannot enumerate the library or reach credentials/providers;
only authorized answers flow back to current empty supported fields. This path
does not create an overlay workspace, tracker entry, or AI operation. See the
technical plan for schema/backup transitions and the staged implementation order.

### Overlay PDF handoff

1. Resume or cover-letter structured content is validated and rendered to PDF locally.
2. Preview streams/reads the derived bytes through an application-owned resource path; expanded editing changes structured content and requests a new render.
3. Download consumes a native save-dialog token and writes atomically to the selected destination.
4. Drag-out materializes the same validated bytes under a private session temporary directory and exposes a platform file-drag payload from the overlay card.
5. Finish/discard removes ORT-owned drag files after the state transition commits. Startup cleanup removes interrupted-session remnants; user downloads are never deleted.

### Direct-provider operation

1. User chooses an action, provider, model/preset, and confirms the estimate.
2. Coordinator validates the published input, stores the logical operation, and transactionally reserves guardrail budget.
3. The provider adapter sends minimized content over HTTPS and streams a structured result.
4. A schema and factual-boundary validator accepts or rejects the response.
5. Usage is normalized, the reservation is settled, and the attempt is finalized.
6. Tailoring stores proposed material, change summaries, and Required Qualification Alerts together.

### Local document import

1. A one-use native dialog token selects the source; the parent validates size, type, permissions, and containment before staging.
2. The disposable document worker extracts bounded text and structural hints under the hostile-input sandbox.
3. The parent validates the extraction message and runs the versioned deterministic mapper.
4. Recognized content maps to existing fields/entry types; unfamiliar headings become proposed custom sections and unclassified text becomes proposed simple text/list blocks.
5. No-AI mode proceeds directly to complete user review with no network. Direct/Codex mode may offer a separately confirmed AI mapping of the extracted text.
6. Only explicitly accepted proposals create a normal draft revision; cleanup follows settlement.

The disabled M2 preparation boundary now composes the first step through Unix
private staging: the same parent-owned snapshot is envelope-validated, written
under a capability-held random operation directory, and transferred once as a
read-only handle. The future execution function orders native launch, common
supervision, adapter destruction and exact stage cleanup, with cleanup failure
overriding extraction. Constrained DOCX and pinned PDFium text adapters now exist
only in the inert worker crate and emit the bounded extraction record. The PDF
adapter verifies a target-specific packaged-library size and digest and has no
system fallback. Windows staging, native adapters, PDFium packaging and UI remain
unavailable, so this does not advance the public pipeline past step 1.

### Codex operation

The logical lifecycle is the same, but the adapter speaks the app-server protocol and records Codex-reported token/quota provenance rather than estimating provider billing. Any tool request, unsupported method, or attempted filesystem/command capability is a containment failure and aborts the operation.

### Backup and restore

Backup reads a consistent database snapshot, creates a portable manifest and content payload, encrypts the entire container with a user passphrase, and writes atomically. Restore inspects and decrypts into a staging directory, validates all schemas/checksums/limits, migrates a copy, then swaps databases only after successful validation.

## Concurrency and crash model

- SQLite is the serialization point for durable state; writes use short explicit transactions.
- The operation coordinator permits one mutating operation per aggregate and bounded parallel read-only renders.
- AI attempts use persisted leases. At startup, expired `dispatching` or `streaming` attempts become `outcome_unknown`; guardrail reservations remain consumed until reconciled or explicitly cleared under product rules.
- Draft saves use optimistic revision numbers. A stale write returns `REVISION_CONFLICT` with the current revision rather than overwriting it.
- Exports write to a sibling temporary file, flush, then atomically rename when the platform permits.
- Low disk space blocks new imports, renders, backups, and updates before existing durable records are endangered.

## Versioning policy

Independent monotonically increasing versions exist for:

- database schema;
- structured resume and workspace schemas;
- UI command contracts;
- extension/native-host protocol;
- backup container;
- prompt and AI response schemas;
- renderer/template bundle;
- provider/model/pricing catalog;
- Codex protocol compatibility range.

Versions are never inferred only from application version. Readers reject newer unsupported major versions and tolerate documented additive minor fields.

## Resource budgets

Initial engineering budgets, subject to measurement:

- extension capture: 128 KiB UTF-8 after normalization;
- native-message and authenticated IPC envelope: 256 KiB maximum;
- one active remote AI operation per local profile;
- import source: 10 MiB, 10 pages, and 50,000 extracted characters maximum;
- backup entry count: 100,000 and total archive size at most 1 GiB before a segmented strategy is required;
- preview renders debounced and only the newest requested revision retained;
- diagnostic log: bounded ring buffer with redaction, rotated locally.

## Observability without telemetry

There is no remote product telemetry. Local structured diagnostics record category, severity, component, safe error code, version tuple, and timestamp. User content, page text, URLs, prompts, responses, API keys, and Codex credentials are excluded. A diagnostic bundle is generated only on explicit user action and is reviewable before sharing.

## Architectural completion criteria

- A skeleton application can edit and persist a synthetic resume, restart, render it, and export it without network access.
- UI code has no filesystem, database, credential, or arbitrary network capability.
- Contract generation and drift checks pass on Windows and macOS.
- Crash tests prove atomic draft saves, operation recovery, and safe exports.
- Threat-model gates in `Security_and_Threat_Model.md` are linked to automated or manual evidence.
