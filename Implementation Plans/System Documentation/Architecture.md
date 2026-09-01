# System architecture

## Status and authority

- Status: approved baseline for implementation
- Owner: core maintainers
- First milestone: M0 architecture skeleton through M7 release hardening
- Product authority: all files in `../../Product Plans/`, especially `Product_Scope_and_Principles.md`, `Local_Data_and_Document_Model.md`, `Product_States_and_Operations.md`, and `Security_Privacy_and_Open_Source.md`

This document defines how the approved local-first desktop product will be assembled. It does not define visual styling.

## Architectural goals

1. The desktop application remains useful without an account, extension, website, AI connection, or network access.
2. Resume content, tracker records, activity history, and credentials remain on the device except for the minimum content the user deliberately sends to a chosen AI provider.
3. Every durable record has one owner and one versioned schema.
4. The same structured document produces preview and export output through the same renderer.
5. External input is untrusted at every boundary: job pages, imported files, AI output, update metadata, backups, and extension messages.
6. UI crashes, provider failures, extension failure, and interrupted updates cannot corrupt the last committed user state.

## Runtime topology

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

The website and GitHub Releases are separate distribution/documentation surfaces. They never receive resume content.

## Process boundaries

### Desktop process

The Tauri process owns all authoritative state and privileged operations. Bundled web UI code can request typed commands but cannot directly access the filesystem, database, network, shell, credential vault, native-messaging registration, or updater.

Long-running work runs as cancellable Rust tasks behind an operation coordinator. CPU-heavy parsing and rendering use a bounded worker pool so the window event loop remains responsive. A process-wide single-instance lock routes secondary launch intents to the existing process.

### UI webviews

There are two application windows: the main window and an optional always-on-top overlay. Both load only bundled assets under a restrictive content-security policy. They share a typed client library but have separate view state. Closing the overlay never cancels or mutates the underlying workspace.

### Native host

`ort-native-host` is a separate, small executable installed beside the desktop application. It accepts one native-messaging request, validates it, forwards it over protected local IPC, returns a bounded response, and exits. It does not open the database or hold content on disk.

### External Codex runtime

Codex is not packaged in ORT. When enabled by the user, ORT launches a supported external executable as an app-server child using `stdio`, an isolated ORT-specific Codex home, a non-user-content working directory, and an allowlisted protocol surface. Direct API mode and Codex mode are mutually exclusive.

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
| Workspace | captured job input and generated artifacts | one active tailoring operation per workspace; user edits remain distinct from AI proposals |
| Tracker entry | application status and structured snapshots | `Finish Application` commits workspace and tracker changes atomically |
| AI operation | logical user request with one or more attempts | every dispatched attempt is persisted before network I/O |
| Guardrail policy | cap configuration, reservations, settlements | direct calls fail closed when cost cannot be bounded under an enabled cap |
| Qualification alert | mismatch/not-found evidence linked to an operation | informational only; dismissal never edits resume facts |
| Backup | encrypted portable snapshot | excludes provider keys, Codex credentials, and device-bound vault material |

## Command, query, and event contracts

UI calls use namespaced commands such as:

- `profile.get`, `profile.update`
- `resume.draft.get`, `resume.draft.save`, `resume.publish`
- `workspace.capture`, `workspace.get`, `workspace.finish`
- `ai.operation.estimate`, `ai.operation.start`, `ai.operation.cancel`
- `ai.activity.query`, `ai.activity.export`
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

1. A user selects text and invokes the extension.
2. The content script returns selection and page metadata only to the extension service worker.
3. The extension shows a confirmation, removes fragments/credentials from the URL, enforces size limits, and sends a versioned native message.
4. The host authenticates local IPC and delivers a capture intent.
5. Desktop creates or updates a workspace and shows the received text for user review. No AI call starts automatically.

### Direct-provider operation

1. User chooses an action, provider, model/preset, and confirms the estimate.
2. Coordinator validates the published input, stores the logical operation, and transactionally reserves guardrail budget.
3. The provider adapter sends minimized content over HTTPS and streams a structured result.
4. A schema and factual-boundary validator accepts or rejects the response.
5. Usage is normalized, the reservation is settled, and the attempt is finalized.
6. Tailoring stores proposed material, change summaries, and Required Qualification Alerts together.

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
