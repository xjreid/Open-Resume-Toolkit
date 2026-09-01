# Local data and migration plan

## Status and requirements

- Status: approved implementation baseline; SQLCipher build proof required in M1
- Owner: storage maintainer
- Milestones: M1, then incremental migrations in every later milestone
- Product authority: `../../Product Plans/Local_Data_and_Document_Model.md`, `Local_Data_Retention_and_Recovery.md`, `Product_States_and_Operations.md`, and `Configuration_Limits_and_Defaults.md`

Non-goals include cloud sync, a remote database, multiple signed-in profiles, backing up credentials, and attempting guaranteed secure erasure from SSD media.

## Storage layout

Resolve all paths through the operating-system application-data APIs. Never derive them from the current working directory.

```text
<app-data>/
  profile.json                 non-secret bootstrap: install id, channel, db format
  data/profile.db              SQLCipher database
  data/profile.db-wal          SQLCipher-managed encrypted WAL
  data/profile.db-shm          SQLite coordination file
  renders/                     bounded derived preview cache
  imports/                     private, short-lived staging
  codex-home/                  isolated external Codex configuration, no resume files
  ipc/                         macOS socket directory where applicable
  diagnostics/                bounded redacted local event files
  recovery/                    pre-migration database safety copies
```

Windows and macOS paths use separate stable, preview, and development application identifiers. Files/directories are created with current-user-only permissions and checked at startup. Symlinks/reparse points are rejected in private staging and recovery directories.

## Database and encryption

Use SQLite through `rusqlite` and SQLCipher Community Edition. The bootstrap spike must pin an auditable SQLCipher build configuration and confirm its BSD-style attribution is present in the in-app/license materials.

On first profile creation:

1. generate a 256-bit database key with the OS CSPRNG;
2. store it in the OS credential vault under the profile/install identifier;
3. initialize SQLCipher with explicit compatibility/KDF/page settings;
4. run migrations and a cipher-integrity check;
5. commit `profile.json` only after the database and vault entry exist.

The key is held in locked memory where practical and zeroized after use. It never appears in SQLite pragmas logged to diagnostics, the environment, command line, or backup. Opening a database without a matching vault key is a recoverable `VAULT_KEY_UNAVAILABLE`/`DATABASE_KEY_MISMATCH` state; ORT must not create a fresh database over it.

Use WAL mode after proving journal/WAL encryption and recovery on all targets. Apply foreign keys, busy timeout, secure-delete behavior where supported, and bounded cache settings on every connection. One writer connection is owned by the storage service; read connections are bounded.

## Schema v1

All IDs are UUIDv7 strings generated locally. Every mutable row includes `created_at`, `updated_at`, and an integer `revision`. JSON payloads include an independent `schema_version`.

### Content tables

| Table | Purpose | Important fields/indexes |
|---|---|---|
| `profiles` | local profile metadata | id, schema payload, revision |
| `resume_drafts` | current editable master | profile id unique, document JSON, revision |
| `published_resume` | zero-or-one immutable current master snapshot | profile id unique, revision id, document JSON, published_at, renderer tuple |
| `workspaces` | overlay-owned active/captured application work | stage, active tab, source kind, job text, sanitized URL, company/title, revision |
| `workspace_artifacts` | tailored resume, letter, answer proposals | workspace id, kind, content JSON, origin operation, accepted/current state, render receipt |
| `qualification_alerts` | confirmed mismatch/not-found alerts | workspace/operation id, type, requirement/evidence JSON, state |
| `tracker_entries` | application status and searchable summary | status, company/title, dates, workspace reference, revision |
| `material_snapshots` | immutable structured content at tracker save | tracker id, kind, content JSON, source artifact id |
| `settings` | non-secret application preferences | namespaced key, versioned value |

### AI and policy tables

| Table | Purpose |
|---|---|
| `ai_operations` | one logical user request, input snapshot hashes, selected mode, final status |
| `ai_attempts` | dispatch/retry attempts, provider/model, timestamps, outcome and safe error |
| `ai_usage_components` | normalized input/output/cache/reasoning tokens and provider-reported usage |
| `ai_cost_components` | estimate/actual values, currency, catalog version, completeness/provenance |
| `direct_guardrail_policies` | user caps, notification thresholds, time zone, revision |
| `direct_guardrail_periods` | calendar windows and settled totals by provider/model/currency |
| `direct_guardrail_reservations` | estimated maximum held before dispatch and settlement state |
| `codex_quota_policies` | local thresholds/notification settings, independent of direct caps |
| `codex_usage_snapshots` | app-server account/rate-limit values, bucket identity, observed time, provenance |
| `connection_records` | non-secret provider/Codex state and vault reference IDs |
| `diagnostic_events` | bounded safe operational records only |

### System tables

`schema_migrations`, `app_metadata`, `renderer_bundles`, `catalog_receipts`, and `maintenance_jobs` record compatibility and recovery state. SQLite full-text tables are introduced only for explicitly selected tracker/resume fields, and remain inside SQLCipher.

## Record choices

- Resume/workspace/snapshot payloads are JSON validated against the canonical generated schema before write and after read.
- Company, title, job text, resume content, URLs, and activity details are treated as sensitive even though SQLCipher encrypts the whole database.
- The published master is immutable in place but zero-or-one per profile. Publishing creates a new row/value and replaces the previous association atomically; it does not create a hidden master-history library. A workspace temporarily copies the published baseline it needs and records its revision so later publishing cannot change an in-progress application silently. Tracker material snapshots remain append-only.
- Qualification alerts store requirement text/span, mandatory-classification evidence, mapped resume field IDs/evidence, result type, confidence metadata for diagnostics, and presentation state. They do not store inferred sensitive personal status.
- A partial unique constraint permits at most one non-finished current workspace per profile. Finishing/discarding it removes temporary qualification alerts rather than copying them into tracker snapshots.
- Stage/tab values use constrained codes: Stage 1 capture/review; Stage 2 `resume`, `cover_letter`, or `answers`. A current answer capture/draft may reset independently without changing job text or other artifacts.
- Activity clearing deletes ledger rows by approved date range according to retention rules but never deletes guardrail periods/reservations or changes cap arithmetic.

## Transactions and concurrency

Required atomic units:

- draft update plus revision check;
- creation/replacement of the single published snapshot;
- AI attempt creation plus direct-cap reservation;
- attempt finalization plus usage/cost rows plus reservation settlement;
- tailoring result plus change summary plus alerts;
- `Finish Application` plus tracker entry plus immutable material snapshots plus workspace state;
- settings/policy change plus policy revision;
- restore database replacement after validation.

Repositories expose domain operations, not generic SQL execution. Expected revision is mandatory for user-editable aggregates. `SQLITE_BUSY` receives a short bounded retry; it never causes an unbounded UI hang.

## AI recovery and accounting

Before external dispatch, persist an attempt with a unique client correlation ID and reservation. State progression is:

```text
prepared -> dispatching -> streaming -> validating -> completed
                                   \-> cancelled
                                   \-> failed
                                   \-> outcome_unknown
```

On startup, attempts whose lease expired in `dispatching` or `streaming` become `outcome_unknown`. Direct reservations remain charged conservatively. If provider usage can be authoritatively reconciled, settlement records its provenance; otherwise the user can inspect the retained unknown reservation under the approved guardrail behavior.

## Local search and indexes

Initial search is local and scoped to resume headings/entries, company, role, tracker notes, and status. SQLite FTS5 runs only inside the encrypted database. Search documents are rebuilt from canonical records during migration/repair; they are not included as separate backup authority.

Prefix/wildcard query features are constrained and parameterized. Returned snippets are escaped before UI rendering.

## Temporary imports and renders

- Copy the selected source into a private random staging directory only when the parser requires seekable ownership.
- Reject over-limit size before copying and recheck after open to reduce time-of-check/time-of-use risk.
- Never modify the source.
- Delete staging on success/cancel/failure; startup cleanup removes expired entries after verifying ownership and containment.
- Derived previews use content-addressed cache keys over document/template/font/renderer versions, bounded by LRU size and age.
- Resume and cover-letter drag-out copies live only under a random private session directory with recorded owner/workspace/artifact IDs. Finish/discard removes them after commit; startup cleanup removes abandoned owned directories after containment checks.
- User exports and intentionally retained tracker snapshots are not treated as cache.

## Backup container v1

Extension: `.ort-backup`.

The file contains a small clear header followed by a single authenticated encrypted payload:

```text
magic "ORTB" | format major/minor | KDF id/parameters | salt | nonce |
ciphertext length | XChaCha20-Poly1305 ciphertext+tag
```

Argon2id derives a 256-bit key from the user passphrase. Parameters are stored in the header, meet a documented minimum, and are calibrated to approximately 500 ms on a reference supported machine without falling below the memory floor. XChaCha20-Poly1305 encrypts the entire payload so filenames and metadata are not exposed. Cryptographic choices require focused review and test vectors before format freeze.

The decrypted payload is a deterministic archive containing:

- `manifest.json` with format/app/schema versions, created time, content inventory, and hashes;
- portable canonical JSON records rather than a raw live database;
- retained structured snapshots and explicitly selected derived files;
- license/format information needed to inspect compatibility.

Excluded: API keys, database key, native IPC secret, Codex auth/configuration, update state, derived indexes/cache, and diagnostics unless the user separately exports them.

Restore validates authentication before decompression, then entry count, total size, hashes, schema versions, IDs/references, and domain invariants. It never uses archive paths. A successful device restore creates a fresh local database key/vault entry and imports the portable content into a newly encrypted database. The backup passphrase cannot be recovered by ORT. The initial release supports replace-restore only; merge restore is deferred until deterministic conflict semantics are designed.

## Migration strategy

- Forward-only numbered SQL migrations are embedded in the application and checksummed.
- Each migration declares minimum reader/app version, estimated disk requirement, whether it rebuilds indexes, and whether a pre-migration copy is mandatory.
- Startup obtains a migration lock, checks free space, creates a consistent encrypted safety copy for destructive/large changes, migrates in a transaction where SQLite permits, runs integrity/domain checks, then opens the UI.
- An interrupted migration resumes or restores the untouched safety copy according to its recorded phase.
- A newer unsupported schema opens no user content and displays upgrade/recovery guidance.
- Old safety copies are deleted only after a successful subsequent startup and within retention/storage limits.

Downgrades that cannot read the current schema are blocked. The supported rollback mechanism is installing the prior app and restoring its pre-migration backup, not executing untested reverse SQL.

## Portable exports

- Full portable export: canonical JSON plus manifest and optionally user-selected rendered documents; no secrets.
- Tracker CSV: documented columns, UTF-8 with spreadsheet-injection escaping, locale-independent dates.
- AI Monitoring CSV/JSON: selected Week/Month/Year/All time bucket series and aggregate breakdowns with normalized usage/cost and completeness/provenance. Attempt rows remain internal and enter only a separately requested scrubbed diagnostic export; raw prompts/responses never enter either export.
- Diagnostic bundle: separate, reviewable, redacted export; never silently included in backup.

## Deletion, uninstall, and repair

In-app deletion requires an explicit confirmation and closes the database before removing the exact resolved profile directory and vault entries. Material deletion uses platform trash when practical or clearly describes permanence. External exports are never deleted.

Normal uninstall removes application binaries and native-host registration but preserves profile data by default. Documentation and uninstall UI explain how to delete retained data. Repair verifies database integrity, permissions, indexes, vault references, native-host registration, and cache; it never rewrites canonical content without a safety copy.

## Tests

- unit/property: IDs, revisions, dates, cap period math, reservation settlement, retention;
- integration: encrypted open/reopen, WAL recovery, concurrent writes, low disk, vault loss/lock;
- migration: every historical schema to current, interruption at each phase, newer schema refusal;
- backup: deterministic fixtures, wrong passphrase, corruption, truncation, hostile sizes/counts, cross-platform restore;
- deletion: exact-target resolution, symlink/reparse protection, preserved exports, vault cleanup;
- fuzz: SQL-boundary values, JSON payloads, archive header/payload and search queries;
- privacy: seeded sensitive markers absent from clear files, logs, backups, and process arguments.

## Rollout and rollback

M1 initially ships to development profiles only. Preview release requires cross-platform encryption and restore evidence. Schema versions are enabled in stable only after a preview cohort has completed upgrade/restart/backup/restore tests. A failed migration prevents app use but preserves both source and safety copy with a support code.

## Completion criteria

- All canonical records and policy state survive clean restart and injected process termination.
- Database, journal, and indexes expose no seeded plaintext at rest.
- Backup restores across Windows and macOS while excluding all credentials and device-bound secrets.
- Clearing activity, deleting a workspace, and resetting a cap affect only their approved records.
- Low-disk, vault-failure, corruption, and newer-version paths preserve recoverability.
