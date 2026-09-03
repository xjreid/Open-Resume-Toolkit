# M2 portable PDF render-history checkpoint

Date: 2026-09-03. Base commit: `cb72c85`. Platform: macOS arm64. Status:
implemented and locally verified; cross-platform CI and native backup UI remain
pending. M2 remains underway.

This checkpoint fixes the `Contracts, tests, and security checks` CI drift from
`cb72c85` and extends the portable-backup prototype so encrypted PDF render
history survives backup and restore. It does not persist PDF bytes, expose the
backup prototype in the UI, enable import, or advance to M3.

## CI diagnosis and fix

The failing Ubuntu job stopped at its first command after setup:

`cargo run --locked -p ort-contract-generator && git diff --exit-code -- packages/contracts/generated`

The committed `pdf.ts` output had been formatted, but
`crates/ort-contract-generator/src/pdf.ts.template` still contained the
unformatted expression. The generator copies that template verbatim, so Linux
recreated a six-line Prettier diff and the contract drift check exited 1. The
template is now the canonical Prettier form. Regenerating contracts produces no
diff, and both the template and generated file pass Prettier.

## Portable history behavior

- The writer advances the authenticated container minor version from 1.0 to
  1.1 and declares database schema 2. The reader accepts both 1.0 and 1.1,
  requires the clear authenticated header and encrypted manifest versions to
  agree, and requires 1.0 payloads to contain no render history.
- Version 1.1 includes at most 100 content-free render manifests in deterministic
  newest-first order. Each record keeps only its UUIDv7 identity, source and
  revision, generation times/count, and the already bounded render receipt.
  PDF bytes, resume content, paths, preview tickets, database keys, vault
  references, diagnostics and credentials remain excluded.
- Export validates canonical IDs, ordering, uniqueness, identity uniqueness,
  JavaScript-safe revisions/times, hashes, identifiers, page/byte bounds and
  counters before deriving the backup key.
- Restore authenticates and validates the complete payload before opening its
  destination transaction. Draft, published records, settings and render
  manifests are then inserted atomically into an otherwise empty profile. The
  destination retains its independently generated SQLCipher key and vault
  identity.
- Empty render-history fields are omitted from canonical JSON so the published
  1.0 deterministic vector remains byte-for-byte reproducible and readable.
  The new 1.1 vector intentionally has its own reviewed SHA-256.

## Verification actually run

- The exact CI contract-generation command passed with no generated diff.
- Focused `ort-backup` and `ort-storage` suites passed. They verify the new 1.1
  deterministic vector, the unchanged 1.0 vector, encrypted round trip, wrong
  passphrase/tamper/truncation handling, separately keyed restore, and exact
  restored render-history identity.
- Full local `just check` passed: Prettier; TypeScript; Node and Vitest suites;
  frontend/extension builds; web and secret scans; Rust formatting; workspace
  Clippy with all targets/features and warnings denied; and all workspace tests.
  The pre-existing opt-in OS-vault test remained ignored and was not newly
  skipped.

## Remaining gates

- Re-run all four CI jobs. In particular, the repaired contract drift command
  needs confirmation on Ubuntu and backup format 1.1 needs Windows/macOS CI.
- Portable backup remains a backend prototype. Atomic user-selected file export,
  recovery UX, cancellation/memory-pressure/hostile-input coverage and native
  cross-device restore verification remain release gates.
- Historical binary replay is not implemented: manifests do not retain old PDF
  bytes, structured draft history, or renderer bundles.
- Import stays disabled (`IMPORT_ENABLED=false`; worker exit 78). Storage
  management/deletion controls, final templates, broader native PDF/DOCX checks,
  Windows containment, and the complete offline journey remain M2 work.

Suggested commit: `feat(backup): preserve PDF render history`
