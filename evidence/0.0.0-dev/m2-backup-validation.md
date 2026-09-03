# M2 authenticated backup-validation checkpoint

Date: 2026-09-03. Base commit: `a3ad66e`, with the preceding uncommitted M2
backup-export and storage-usage checkpoints in the same working tree. Platform:
macOS arm64. Status: implemented and locally verified with automated synthetic
fixtures; cross-platform CI and native dialog/assistive-technology inspection
remain pending. M2 remains underway.

This checkpoint advances replace-restore only through safe intake and read-only
validation. It does not create a staging profile, close the active database,
write a vault item, replace data, delete data, or retain the selected file.

## Implemented behavior

- A generated main-window-only request accepts bounded request metadata and one
  passphrase. Paths, filenames, profile IDs, destinations, replacement flags and
  unknown fields are rejected before a dialog opens. The visible field clears at
  dispatch, and Rust owns the passphrase for the single blocking operation.
- The native Open dialog is covered by the existing one-file-operation lease, so
  it cannot overlap export/render dialogs and normal quit waits for completion.
  Cancellation succeeds without reading or changing any profile data.
- The platform adapter accepts only an absolute `.ort-backup` selection, opens
  its parent as a held capability, checks the final entry without following
  symlinks, and requires a regular nonempty file. It rejects size above the fixed
  64 MiB payload/container allowance before allocation, caps the actual read,
  and rejects a length change instead of returning partial bytes.
- Validation uses the existing format-1 reader: header/version/reserved/KDF/size
  policy precedes Argon2id work; XChaCha20-Poly1305 authenticates the entire
  container; bounded JSON, schema, hash, inventory and domain checks follow.
  Wrong passphrases, tampering, truncation and malformed encrypted content use
  the same non-oracular command failure.
- A success response contains only the authenticated container byte/version,
  application/database/document versions, creation time, and bounded counts for
  drafts, published snapshots, settings and render manifests. It contains no
  path, filename, passphrase, document text, setting value, hash, vault identity,
  decrypted payload, or native error detail. The TypeScript validator independently
  enforces exact fields, supported 1.0/1.1 schema pairings and count/byte bounds.
- The accessible summary explicitly states that authentication is read-only and
  that replace-restore remains disabled. Legacy 1.0 validation remains supported
  and cannot report render history.

`cap-primitives` 4.0.3 is now declared directly by `ort-platform` solely for its
existing no-follow open policy type. The package and version were already locked
transitively through `cap-std`; lock regeneration ran offline and downloaded no
new dependency.

## Verification actually run

- Full local `just check` passed: Prettier, TypeScript lint, Node/Vitest suites,
  frontend/extension builds, web-security and secret scans, Rust formatting,
  strict workspace Clippy with all targets/features, and all workspace tests.
  The opt-in OS-vault mutation test remained ignored as designed.
- Platform tests cover valid bounded reads, wrong extensions, directories, empty
  files, a sparse over-limit file rejected before reading, and final symlink
  refusal through the same capability used by the native command.
- Backup command tests create an authenticated synthetic archive, return its
  content-free inventory, and verify that a wrong passphrase maps to the uniform
  invalid result without details.
- Domain/contract tests reject renderer-supplied paths and replacement flags;
  accept exact current and legacy summaries; and reject extra path content,
  unsupported version/schema pairs, invalid counts and oversized byte totals.
- Desktop tests verify the passphrase-only command, runtime response validation,
  truthful read-only/cancellation copy, and indistinguishable passphrase/file
  failure language. Contract regeneration is byte-stable after formatting.

## Remaining gates

- Exercise the native Open dialog on signed macOS and Windows builds, including
  aliases/symlinks, Windows reparse points, remote/removable filesystems,
  cancellation timing, inaccessible files, replacement races and assistive
  technologies. Re-run all four CI jobs.
- Harden all decrypted parsed record allocations according to the locked-memory
  and zeroization plan; the serialized plaintext buffer and derived key are
  already zeroized, but ordinary domain strings created during validation are
  not yet guaranteed to be locked or wiped on drop.
- Implement replace-restore only with a freshly keyed staging profile, integrity
  verification, sufficient-space check, encrypted safety copy, atomic profile and
  vault handoff, restart journal/recovery, and failure-injection tests that prove
  the current profile remains usable after every interrupted phase.
- Destructive local-data deletion, parser containment/import UI, historical
  renderer replay, final templates and complete offline journey evidence remain
  separate M2 work.
