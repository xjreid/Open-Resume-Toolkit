# M2 document-schema v2 foundation

Baseline: M1-qualified `65518eb9d434a5ff6158810000b8798261d9227c`.
Status: Step 5 local foundation; not installed or natively qualified. M1 remains
complete for its recorded macOS-arm64 development scope. M2 remains incomplete.

## Representation and upgrade

Readers accept document schemas 1 and 2. New empty resumes still start as schema 1
until the structured-date editor and explicit upgrade action are integrated.
Loading never upgrades a document. The Rust `upgraded_v2` and frontend
`upgradeDocumentV2` helpers create a copy for a subsequent ordinary edit/save.
The Rust helper validates before and after upgrade, including serialized bounds.

Schema 2 requires UUIDv7 IDs and canonical zero-based order on contact and entry
links, plus an explicit `dates` array on every entry. IDs are unique across the
whole document. Changing labels and reordering retain identity. A date record
represents one labeled date or range with its own ID/order and optional start/end.
Calendar values require a year (1–9999), optional month (1–12), and explicit
expected flag. An end may instead be Present. Missing months remain missing;
missing endpoints do not generate separators or invented dates. Completely empty
dates, including their labels, are omitted from output. There are at most 200
date records per document, with labels charged to existing character bounds.

Legacy free-text `dateRange` stays byte-for-byte intact during upgrade. It may
remain in a v2 entry while its structured dates array is empty. The validator
rejects simultaneous nonempty legacy text and structured dates, preventing two
conflicting date representations. Reversed known ranges produce a nonblocking
warning; ambiguous mixed-precision ranges are not declared reversed. Invalid
calendar numbers fail validation with field-specific UI feedback.

Optional v2 fields are omitted when serializing v1; explicit null identity/date
fields are rejected instead of being silently discarded. Existing v1 serialized
content, source hashes and immutable publications are preserved. Ordinary draft
updates cannot downgrade the stored schema, enforced in the atomic SQL update
alongside expected revision. No database table migration is needed: the existing
schema/version columns and encrypted JSON support both versions.

## Backup, render and reader compatibility

- Profiles containing only v1 documents still write backup 1.1. The existing 1.0
  and 1.1 vectors remain readable and their reviewed hashes are unchanged.
- Any retained v2 draft or publication requires backup 1.2, with database schema
  2 and document schema 2 in authenticated metadata. Mixed v1/v2 archives retain
  each source's original version. The writer refuses to label v2 content as an
  older archive; the reader checks header, manifest and actual document versions.
  Existing older apps reject the new minor version before decryption.
- The compatibility manifest now advertises document schema 2 reader support.
  Command contract 2, database schema behavior, backup major version, renderer
  version and original templates remain unchanged. Backup cryptography, KDF
  policy, keys and file-selection/write boundaries were not changed.
- PDF, DOCX and text render the same explicit date values in order. PDF receipts
  identify the actual document schema. New fields in v1 remain absent, preserving
  all eight legacy PDF/DOCX/text goldens and exact historical receipts.
- Frontend readers accept both supported shapes, reject v2 duplicate/missing IDs,
  inconsistent ordering, invalid date precision, mixed schema fields and unknown
  future versions. Normalization preserves existing v2 IDs and assigns IDs only
  to newly added links. Current accessible previews show structured dates.

This is not complete forward-unknown-field recovery: unsupported future fields
and schemas are refused, not rewritten. A recovery/export experience for future
unknown content remains product work. No automatic interpretation of legacy date
text, persisted style preference or silent publication migration was added.

## Validation

- `cargo test --workspace --all-targets --locked`: 202 passed, 9 explicitly ignored
  native/opt-in cases (`target/m2-schema-workspace.log`). All-target/all-feature
  Clippy passed (`target/m2-schema-clippy.log`).
- `CI=true pnpm check` passed: 79 desktop tests, 23 contract tests, TypeScript,
  formatting, builds, static security and dependency licenses
  (`target/m2-schema-web-check.log`).
- Synthetic encrypted-store integration upgrades a draft, refuses a downgrade,
  reopens it, round-trips a mixed-version backup into a fresh store, and verifies
  the old publication and exact original PDF receipt. No OS vault is accessed.
- Domain tests cover lossless/idempotent upgrade, null/mixed-shape rejection,
  duplicate IDs, order, calendar bounds, free-text conflicts, expected/present,
  omitted endpoints and nonblocking reversed ranges. Backup tests refuse legacy
  format mislabeling and inconsistent authenticated schema metadata.
- Independent PDF.js and Python ZIP/XML inspection checked v2 date precision,
  ordering and omission across all four PDF/DOCX styles
  (`target/m2-schema-independent.log`, `target/m2-schema-fixtures/`). Generate the
  synthetic corpus using `cargo run --locked -p ort-render --example
  schema_fixtures -- NEW_DIRECTORY`.
- All eight existing plain golden pairs passed independent audits unchanged
  (`target/m2-schema-golden.log`). Generated contracts were regenerated and
  compared byte-for-byte with the working tree (`target/m2-schema-generated.json`).
  The usual `just verify-contracts` compares against Git and will show the
  intentional uncommitted generated changes; this is not a clean-tree CI claim.

The installed app remains the M1-qualified build. No account switch, native
Keychain test, installation, commit or push was performed.

The subsequent Medium date/link editor work is implemented locally and recorded
in `m2-date-link-editor.md`: explicit draft upgrade, date fields and range warnings,
legacy-text replacement confirmation, and stable link/date ordering. Empty
workspaces continue to start at v1 until the user explicitly enables these
controls. The complete focused editor, final style layouts, historical
regeneration, sandboxed import, lifecycle qualification and M2 native acceptance
remain open.
