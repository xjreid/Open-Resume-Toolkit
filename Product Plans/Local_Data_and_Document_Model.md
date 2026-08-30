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

Contains local preferences, schema version, installation identity, backup preferences, provider references, update channel, and pointers to the records below. It must not duplicate AI credentials stored in the operating-system vault.

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
- Contains reviewed job text, sanitized URL, workflow state, temporary tailored versions, cover-letter drafts, question/answer drafts, user edits, and selected final materials.
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

### Operational metadata

Contains non-content diagnostic facts such as operation state, duration, error category, provider name/model, token/cost information returned by a provider, and timestamps. It must remain separable from resume, job, answer, and credential content when a user creates a diagnostic report.

## Local storage requirements

- Use a transactional local store appropriate for structured relationships and migrations; SQLite is the leading implementation candidate but is not selected by this product plan.
- Store large temporary import files separately from durable structured records and delete them as soon as the user accepts, rejects, or abandons the import.
- Apply restrictive per-user file permissions.
- Protect canonical content at rest with application-level encryption whose key is held in Windows Credential Manager or macOS Keychain, unless the implementation plan and security review document an equally protective platform-native design. Never store the decryption key beside the database.
- Never write API keys, full resume text, job descriptions, generated answers, or document contents to ordinary logs.
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

- A backup contains the local profile, master draft and published snapshot, tracker entries, retained structured materials, settings safe to transfer, schema manifests, and integrity metadata.
- Backups use a documented, versioned container such as `.ort-backup`.
- Backups should be encrypted with a user-supplied passphrase using a current password-based key-derivation and authenticated-encryption design chosen during implementation.
- API keys and credential-vault secrets are excluded by default and must be reconfigured on the destination device.
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
