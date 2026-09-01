# Local data and document model

## Authority and ownership

The user's local profile is the authoritative source for all ORT product content. There is no ORT cloud replica, administrative copy, or server-side recovery mechanism.

Implementation plans must preserve these logical boundaries without prematurely prescribing a particular database library:

- Structured records are versioned and locally durable.
- Secrets are separated from content records.
- Temporary workspace content is separated from retained tracker content.
- Derived exports are not silently imported back into canonical data.
- Migrations are atomic, backed up, and recoverable.

## Canonical records

### Local profile

Contains local preferences, schema version, installation identity, backup preferences, the active AI connection mode, opaque direct-credential references or a Codex connection reference, update channel, and pointers to the records below. It must not duplicate API keys or Codex authentication tokens stored in the operating-system vault.

### Master-resume draft

- Exactly one per profile.
- Mutable structured content with stable identifiers and explicit ordering for sections, entries, fields, links, bullets, skills, and custom values.
- Saves locally and crash-safely after changes.
- Distinguishes absent values from intentionally empty values.

### Published master resume

- At most one per profile.
- Immutable snapshot of an accepted draft revision until deliberately replaced.
- Records its schema, style, and publication revision so application materials retain their factual source.

### Application workspace

- Zero or one current workspace per profile.
- Contains reviewed job text, sanitized URL, workflow state, temporary tailored versions, Required Qualification Alerts and their dismissed/ignored presentation state, cover-letter drafts, question/answer drafts, user edits, and selected final materials.
- Each alert stores a stable workspace-local identifier, classification, normalized mandatory requirement, bounded job-text evidence reference, optional published-resume conflict evidence reference, source published-master revision, generation/validation version, and presentation state. It stores no inferred personal qualification or eligibility decision.
- Stored locally with a bounded recovery policy so crashes and restarts do not destroy current work.
- Cleared by successful Finish Application, explicit finish without saving, or explicit discard.

### Application tracker entry

- Durable local tracking record with the approved fields in `Core_Workflows.md`.
- Associates with zero or one selected final tailored-resume snapshot, zero or one selected cover-letter snapshot, and one ordered approved answer set.
- Uses stable identifiers and timestamps so exports, migrations, and future linking do not depend on row position.

### Structured material snapshot

- Immutable saved JSON typed as tailored resume, cover letter, or question-answer set.
- Records schema version, source published-master revision, provider/model and prompt configuration identifiers when AI-assisted, template/style version, renderer version, font-package version, page target, locale, creation time, and integrity checksum.
- Editing a retained artifact creates a new snapshot and deliberately replaces the association; it does not rewrite historical source bytes silently.

### AI activity ledger

- Durable, user-visible local records for logical AI operations and their individual provider-call attempts.
- Stores opaque local identifiers, operation type, connection mode, provider, credential identity where applicable, requested and effective model, preset version, timestamps, duration, status, retry relationship, coarse error category, provider-reported usage categories, local size estimates, contemporaneous cost estimate where applicable, currency, estimate completeness, and pricing-catalog version/effective date.
- Does not store API keys, prompt or response bodies, resume/job/question content, full URLs, filenames, or provider-account credentials.
- Preserves raw provider-reported usage separately from locally derived estimates so later code or pricing changes do not obscure provenance.
- Represents only calls initiated by this ORT installation and is not an account-wide provider billing record.

### AI guardrail state

- Durable accounting state separate from activity-history display and retention.
- Direct-API records contain the opaque credential identity, enabled calendar/all-time periods, configured currency caps, fixed time zone and boundaries, warning thresholds, counted estimated spend, active reservations, unresolved outcomes, next reset, and explicit policy/baseline-change audit facts.
- Codex records contain stable provider quota-bucket identifiers, user thresholds, last provider-reported used percentage/window duration/reset time, refresh status, and the mapping/review state when provider buckets change.
- Stores no API key, Codex access/refresh token, prompt, response, resume/job content, or provider cookie.
- Clearing or expiring AI Activity cannot mutate this state. Changing or resetting it is a separate, explicit operation.

### Codex account-usage snapshot

- Optional last-known provider data used to render Codex status: plan type, account-level token summary, daily token buckets, quota bucket identifiers/names, used percentages, window durations, reset times, retrieval time, and availability/error state.
- Account-level measurements remain labeled as account-wide and are not merged into ORT-only per-operation totals.
- The snapshot is replaceable cache, not an authoritative invoice or the source of guardrail history. Authentication secrets and full account identifiers are excluded.

### Operational diagnostics

Contains bounded non-content troubleshooting facts needed beyond the durable AI activity ledger. Diagnostic data must remain separable from resume, job, answer, credential, and activity-history records when a user creates a diagnostic report. Short-lived diagnostic logs do not replace, silently add content to, or become the authoritative source for AI Activity.

## Local storage requirements

- Use a transactional local store appropriate for structured relationships and migrations; SQLite is the leading implementation candidate but is not selected by this product plan.
- Store large temporary import files separately from durable structured records and delete them as soon as the user accepts, rejects, or abandons the import.
- Apply restrictive per-user file permissions.
- Protect canonical content at rest with application-level encryption whose key is held in Windows Credential Manager or macOS Keychain, unless the implementation plan and security review document an equally protective platform-native design. Never store the decryption key beside the database.
- Never write API keys, full resume text, job descriptions, generated answers, or document contents to ordinary logs.
- Treat the AI activity ledger as encrypted canonical profile data rather than a plaintext log.
- Treat AI guardrail state as encrypted canonical profile data with transactional reservation and settlement. Activity deletion, retention cleanup, crash recovery, and migration must not weaken or accidentally reset an enabled cap.
- Codex app-server authentication belongs in the OS credential store selected by the implementation. ORT must not copy or import a general-purpose Codex auth file into its content database or backups.
- Crash recovery may use journals or temporary files, but they must inherit the same protection and cleanup rules as the canonical data.

## Structured document requirements

- Content schema and presentation schema remain separate.
- Empty optional fields do not render.
- Templates preserve all compatible structured content when changed.
- Links store user-visible labels separately from validated `https` or other explicitly approved destinations.
- Dates support partial knowledge without inventing a day or month.
- Unknown future fields and status codes fail safely and remain exportable where possible.
- Schema migrations never silently change factual content.

## Rendering and retained artifacts

- PDF, DOCX, and plain text are generated locally from structured data.
- The tracker retains structured snapshots rather than generated binaries by default.
- A render manifest makes historical material reproducible within reasonable compatibility bounds.
- ORT may state that a document is semantically reproducible; it must not promise byte-identical output after renderer, font, or operating-system changes unless that property is specifically implemented and tested.
- Exact submitted-file archiving is deferred and would require separate storage, privacy, and user-experience approval.

## Backup, restore, and device migration

Because ORT has no cloud recovery, the application must provide a first-class portable backup.

- A backup contains the local profile, master draft and published snapshot, tracker entries, retained structured materials, activity and guardrail settings safe to transfer, schema manifests, and integrity metadata.
- Backups use a documented, versioned container such as `.ort-backup`.
- Backups should be encrypted with a user-supplied passphrase using a current password-based key-derivation and authenticated-encryption design chosen during implementation.
- API keys, Codex sessions, and credential-vault secrets are excluded by default and must be reconfigured on the destination device. Imported guardrail policies remain inactive until the user binds and confirms them for a new credential or Codex connection; historical counters never silently attach to a different identity.
- Restore validates format, integrity, schema compatibility, available disk space, and conflicts before replacing or merging data.
- Before a destructive migration or restore, ORT creates or offers a recoverable safety copy.
- Automatic backup may target a directory chosen by the user, including a folder synchronized by a third party. ORT must explain that choosing such a folder causes that third party—not ORT—to receive the encrypted backup.

## Import and export portability

- Full data export uses a documented versioned format and does not require ORT servers.
- Tracker CSV includes tracking fields and artifact-presence metadata, not embedded document JSON.
- A full portable export includes structured material snapshots and render manifests.
- Import must validate untrusted archives, paths, sizes, compression ratios, schemas, and checksums before writing canonical data.

## Storage limits

ORT imposes no product-tier quotas. Implementation safety limits should guard individual file size, rendered page count, extracted text size, nested collections, and archive expansion. Defaults must be generous for ordinary resumes and applications and configurable only where doing so remains safe.

The app reports local storage use and allows users to remove temporary imports, individual retained artifacts, or tracker entries. Low-disk-space failures must preserve existing canonical data and provide a recoverable error.
