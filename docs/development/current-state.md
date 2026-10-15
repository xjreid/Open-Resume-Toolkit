# Current implementation map

Reviewed against the local source on October 15, 2026. This describes the
working implementation, including the audit fixes in the current change. It is
not hosted CI, production-release, or milestone acceptance evidence. The
[roadmap](../../Implementation%20Plans/System%20Documentation/Delivery_Roadmap.md)
retains authority over milestone closure.

## Features and boundaries

| Area | Implemented today | Remaining boundary |
| --- | --- | --- |
| Platform | Tauri 2, React/TypeScript, Rust; macOS Apple Silicon development | Intel Mac and Windows have portability CI, not native support qualification |
| Resume | Structured editing, autosave, publications, reviewed PDF/DOCX import, preview and PDF/DOCX/text export | Packaged import requires pinned metered-Wasm parser helpers; no OCR |
| Documents | Technical, Professional, Modern; internal Plain fixtures and replay compatibility | No guarantee of universal ATS behavior or exact submitted-file archival |
| Application | Overlay-owned capture/review, tailoring, qualification alerts, letters, answers, structured edits, PDF/DOCX download and macOS drag | Browser capture and AI remain explicit; no form submission |
| Direct AI | Multiple vault-backed keys; one active key; explicit catalog model IDs; local accounting and per-key/general spending caps | Price/model/account availability needs release-time review; no hosted ORT inference |
| Codex | Verified external runtime, explicit installer, Start/Stop server, managed memory-only login, model/reasoning selection, reserve checks and Data activity | macOS arm64 only; account-specific live acceptance is separate; no inherited Codex credentials |
| Tracker | Atomic Finish, retained snapshots, inline metadata edits, background search, filters, 100-row pages, visible-page CSV | Retained materials are read-only; no calendar or job-board automation |
| Chrome | 0.3.0 source/package; desktop-authorized rectangle capture; bounded native transport | Development host/extension setup required; Store publication and signed production bridge remain M7 |
| Edge | Shared source and a capture-disabled scaffold | Functional capture and its browser walkthrough remain M5 work |
| Local data | SQLCipher, OS-vault database/API keys, profile ownership lock, immutable publications, durable AI accounting | No cloud recovery; exports are outside profile deletion |
| Backup/recovery | Container 1.9; restore into a fresh identity; retained safety copy and rollback; retryable credential retirement | Export has byte/count safety bounds; no merge restore |
| Distribution | Local development app/helper packaging and preview integrity checks | No production-qualified release/updater workflow; Developer ID/notarization and final identities remain M7 |

The app UI uses Open Folio and Hanken Grotesk. Old Quiet Navy/Precision Workbench
references describe the earlier direction. Exported resumes do not inherit app
branding.

## Persistence facts

- One editable master draft and one current published master are exposed to the
  user. Earlier immutable publication revisions remain stored for replay and
  backup. Publication does not delete them or create another editable master.
- The encrypted database is schema 9. Its tracker search projection is derived
  from canonical entries and updated atomically by database triggers.
- Portable backup remains format 1.9 with database-content schema 8: schema 9
  adds a derived projection, not a new portable record. The reader accepts
  1.0–1.9 and rebuilds derived search data on restore.
- Backup collection checks aggregate bytes/counts in the same database snapshot
  before loading rows, then counts exact canonical bytes as records are collected.
  Encryption and key derivation occur after the desktop storage lock is released.
- The process owns a stable sibling lock before recovery opens any profile. A
  second desktop launch refuses access until that owner exits; automatic
  activation/forwarding to an existing window is not implemented.
- Deleting a safety copy or all local profiles inventories provider addresses
  durably before deleting their credentials, database keys, and roster files.
  Vault failures remain retryable; credentials survive while rollback is offered.
- New drag roots are private and hold an OS lease. Startup removes abandoned
  roots older than 24 hours while preserving live drags. Legacy roots without a
  lease require a prior boot plus strict owner/content validation.
- Codex authorization is memory-only. Every application launch starts stopped;
  model, reasoning, reserve settings, and activity remain, while server permission
  and account identity reset. Direct-provider API keys remain in the OS vault.

## Reading order

[Development setup](../../DEVELOPMENT.md), [CI checks](ci.md),
[application behavior](../application-behavior/README.md), then the owning
component plan. Historical milestone evidence and dated audits retain their
original results; do not interpret old test counts as the current suite size.
