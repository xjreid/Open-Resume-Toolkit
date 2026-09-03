# M2 native portable-backup export checkpoint

Date: 2026-09-03. Base commit: `a3ad66e`. Platform: macOS arm64. Status:
implemented and locally verified through automated tests; native Save-dialog and
cross-platform CI verification remain pending. M2 remains underway.

This checkpoint exposes creation of the existing encrypted portable container in
the main desktop editor. It does not implement current-profile replacement,
enable imports, alter the format-1.1 bytes, or advance to M3.

## Implemented behavior

- The generated command contract accepts only request metadata and a nonempty
  UTF-8 passphrase capped at 1,024 bytes. Paths, overwrite flags, profile content,
  database/vault keys, and unknown fields are rejected. The secret-bearing Rust
  request types do not derive `Debug` or `Clone`.
- The UI requires a matching confirmation, recommends a long unique passphrase,
  clears both controlled fields at dispatch, and truthfully states that ORT
  cannot recover it. It also states the saved-only scope, credential exclusion,
  sync-folder consequence, new-file policy, and unavailable restore flow.
- The native command is main-window-only and runs the dialog, encrypted-profile
  snapshot, Argon2id derivation, XChaCha20-Poly1305 encryption, and file write off
  the async event loop. It shares the single file-operation gate with text, DOCX,
  and PDF so competing dialogs fail and quit cannot approve exit mid-operation.
- Native selection becomes a held-directory, one-use capability before profile
  reading. The fixed `.ort-backup` writer rejects special/wrong/existing names,
  publishes exact bounded bytes through a private sibling directory and
  no-clobber hard link, uses mode 0600 on Unix where supported, and exposes
  cleanup/durability warnings separately from failure.
- Successful responses contain only byte count, fixed format 1.1, and the two
  publication warnings. Cancellation returns no file receipt. Bounded error
  codes contain no selected path, passphrase, content, vault identifier, or raw
  operating-system error.
- The archive remains canonical saved data: draft, published snapshots, portable
  settings, and at most 100 content-free render manifests. It excludes unsaved
  editor state, database/vault keys, credentials, diagnostics, PDFs, and preview
  tickets.

## Verification actually run

- Full local `just check` passed after implementation: Prettier; TypeScript;
  Node and Vitest suites; frontend/extension builds; web and secret scans; Rust
  formatting; strict workspace Clippy with all targets/features; and all
  workspace tests. The opt-in native OS-vault mutation test remained ignored as
  designed and was not newly skipped.
- Contract tests reject unexpected response fields, wrong format versions,
  zero/oversized byte counts and oversized passphrases. Desktop tests verify the
  exact path-free invocation, fixed response identity, no retry after an unknown
  outcome, bounded feedback, cancellation, cleanup/durability warnings, and the
  accessible two-password-field limitation copy.
- Rust command tests verify non-sensitive error mapping. Platform tests verify
  fixed extension, exact maximum size, private publication, no replacement, and
  wrong-extension rejection.
- A new cross-crate synthetic integration starts from encrypted saved/published
  records, creates a backup, proves the plaintext marker is absent, publishes
  exact bytes, refuses a second write to the same target, restores from the file
  into a fresh profile with an independent in-memory vault, and verifies both
  restored integrity and unchanged source revision.
- The existing backup/storage suites still verify deterministic format-1.1 and
  legacy-1.0 vectors, wrong-passphrase/tamper/truncation uniformity, bounded
  content validation, render-history preservation, and separately keyed restore.

## Remaining gates

- Exercise the actual native Save dialog with a synthetic development profile on
  macOS and Windows, including cancel, existing-file refusal, synchronized,
  removable, low-space and hard-link-unsupported destinations, permissions/ACLs,
  concurrent quit, interruption, cleanup, and directory-durability outcomes.
- Re-run all four CI jobs. This checkpoint has not been remotely verified.
- Implement replace-restore by authenticating and validating before mutation,
  creating or offering a safety copy, importing into a newly keyed staged profile,
  atomically switching the exact profile/vault identity, and recovering every
  interrupted phase. Wrong passphrase and hostile files must leave the current
  profile untouched. Merge restore remains deferred by the approved plan.
- Storage usage/deletion controls, automatic staging cleanup, final renderer
  replay, parser containment/import UI, broader native document checks and the
  complete offline journey remain M2 work. No release-eligibility claim is made.
