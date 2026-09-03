# M2 encrypted PDF render-manifest history

Date: 2026-09-03 UTC (2026-09-02 local). Base commit: `296610a` plus the
uncommitted installed-app evidence listed in the handoff. Platform: macOS arm64.
Status: implemented and locally verified; cross-platform CI and refreshed native
application verification pending. M2 remains underway.

This checkpoint adds the first forward database migration and makes PDF render
receipts durable and inspectable. It does not add another renderer, retain PDF
bytes, enable import, or advance to M3.

## Implemented behavior

- Encrypted database schema v2 adds `render_manifests`. Every successful native
  preview records its exact saved-draft or published-snapshot revision,
  document/PDF hashes, document schema, renderer/template/font IDs and hashes,
  page/byte counts, first/last generation times and repeat count before generated
  PDF bytes are returned to the webview.
- PDF bytes, resume text, filesystem paths and expiring preview tickets are not
  stored. The history command is main-window-only, takes an empty path-free
  request and returns at most the newest 20 validated manifests.
- Re-rendering an identical source/revision/PDF identity increments one bounded
  counter. The encrypted store retains the newest 100 distinct identities and
  prunes older rows in the same transaction as a new receipt.
- The PDF panel exposes the encrypted history with source, revision, last-render
  time, page count, repeat count and PDF SHA-256. Current preview receipts state
  that their metadata is stored; export still consumes only the expiring native
  preview ticket and exact cached bytes.
- Existing schema-v1 profiles verify their original migration checksum, apply v2
  additively in an immediate transaction, verify both checksums/integrity, and
  then update the non-secret profile manifest. The exact-name manifest handoff
  retains `profile.json.previous` until the new file is durable. Startup restores
  that exact previous file after interruption and rejects symlinks or unexpected
  entry types; it does not scan or delete arbitrary names.
- A render-history write failure prevents the new preview from being exposed.
  Existing saved data and any previous native preview remain unchanged. The
  migration adds no plaintext fallback and does not change SQLCipher keying,
  memory protection or logging policy.

## Verification actually run

- Focused Rust suites passed for `ort-storage`, `ort-domain`, and `ort-desktop`,
  including native SQLCipher startup. Storage tests cover schema-v1 upgrade,
  checksummed v1/v2 receipts, newer-schema refusal, manifest-handoff interruption
  recovery, encrypted reopen, exact-identity deduplication, 100-row retention,
  invalid limits and absence of a seeded resume marker from profile files.
- Contract generation produced separate history request/response schemas.
  Contract tests reject unexpected paths/bytes, malformed UUIDv7 IDs, duplicate
  IDs, invalid times/counters/receipts and more than 20 results.
- Desktop TypeScript checks and all contract/desktop Vitest suites passed. UI
  tests cover the encrypted-history disclosure, and command-client tests confirm
  the request contains only an empty payload.
- The existing encrypted PDF application journey now records both draft and
  published receipts and verifies their order and exact hashes after a second
  encrypted-store restart.
- Full local `just check` passed: Prettier, TypeScript, Node/Vitest, production
  frontend and extension builds, web/secret policy scans, Rust formatting,
  workspace Clippy with all targets/features and warnings denied, and all
  workspace tests. The pre-existing opt-in OS-vault test remained ignored; it
  was not newly skipped. `git diff --check` also passed.

No installed application was replaced or opened for this checkpoint. The
existing `/Applications/Open Resume Toolkit Dev.app` remains the prior `296610a`
build; its saved profile was not migrated by these source-level tests.

## Remaining gates

- Run all four CI jobs, including Windows and macOS Intel migration/SQLCipher
  behavior. Native profile migration and history UI need verification in a
  refreshed app using synthetic data after CI.
- Historical binary replay is not implemented. A receipt identifies prior
  inputs/tooling/output but does not retain the old renderer bundle, structured
  revision or PDF bytes needed to regenerate every old artifact.
- Portable backup format 1.1 now includes render manifests; see
  [the follow-on checkpoint](m2-portable-render-history.md). Final backup/export
  UI semantics, deletion/storage-management UI and bounded retention controls
  remain M2 work.
- Import stays disabled (`IMPORT_ENABLED=false`; worker exit 78). Export
  replacement/crash cleanup, final templates, broader native PDF/DOCX checks and
  Windows containment remain separate gates.

Suggested commit: `feat(storage): persist bounded PDF render history`
