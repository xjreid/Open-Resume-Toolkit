# M2 encrypted-profile storage-usage checkpoint

Date: 2026-09-03. Base commit: `a3ad66e`, with the preceding uncommitted
portable-backup export checkpoint in the same working tree. Platform: macOS
arm64. Status: implemented and locally verified through automated tests;
cross-platform CI and native assistive-technology inspection remain pending.
M2 remains underway.

This checkpoint adds visibility into active local storage without adding a
destructive operation. It does not delete, vacuum, restore, enumerate arbitrary
directories, or change backup format bytes.

## Implemented behavior

- A generated main-window-only command accepts an empty payload. Paths, profile
  identifiers, content selectors, cleanup/vacuum flags and unknown fields are
  rejected before storage access.
- One SQLCipher read returns content-free counts for master drafts, immutable
  published snapshots, portable settings, PDF render manifests and diagnostic
  events. Counts are checked into unsigned bounded response fields.
- Fixed-name metadata reads report the encrypted database, encrypted WAL, SQLite
  shared-memory, non-secret profile manifest, and transient exact-manifest
  recovery files. Missing optional sidecars count as zero. Required missing files,
  symlinks, directories, special entries, arithmetic overflow, and values beyond
  JavaScript's safe-integer range fail closed with an existing safe storage error.
- `totalProfileBytes` must equal the checked sum of all returned file categories.
  The TypeScript response validator independently enforces exact fields, positive
  required files, draft cardinality, unsigned counts, safe integers, and the same
  total invariant.
- The UI uses a definition-list layout with exact byte counts plus readable IEC
  units. It identifies external exports/backups, OS-vault items and in-memory PDF
  preview bytes as exclusions, and explains transient WAL/SQLite behavior and
  diagnostic-record backup exclusion. It refreshes when storage becomes idle and
  offers a keyboard-accessible manual refresh.

## Verification actually run

- Full local `just check` passed after implementation: Prettier; TypeScript;
  Node and Vitest suites; frontend/extension builds; web and secret scans; Rust
  formatting; strict workspace Clippy with all targets/features; and all
  workspace tests. The opt-in OS-vault mutation test remained ignored as designed
  and was not newly skipped.
- Contract tests accept only exact content-free inventories and reject extra
  content, invalid draft/count bounds, unsafe byte values, and inconsistent totals.
- Desktop tests verify the empty path-free request, malformed-response refusal,
  exact byte/IEC formatting, and the pre-data accessibility/exclusion copy.
- Encrypted-storage tests verify empty and populated counts, two immutable
  snapshots, settings, diagnostics and render manifests; exact known-file sum;
  schema reporting; at-rest marker absence; and refusal of an unexpected recovery
  metadata symlink.
- Contract regeneration was rerun and remained byte-identical after formatting
  the source template.

## Remaining gates

- Re-run all four CI jobs and inspect the panel in native WKWebView/WebView2 with
  keyboard, VoiceOver/NVDA, forced colors, and 200% text scaling.
- Add deliberate deletion only after exact-target resolution, confirmation copy,
  database closure, vault cleanup, platform trash/permanence behavior and
  interruption recovery are implemented and verified. External exports and
  backups must remain untouched.
- Define bounded cleanup/VACUUM policy with low-disk and crash tests before
  presenting any bytes as reclaimable. Current measurements are usage, not a
  cleanup estimate or quota.
- Replace-restore, parser containment/import UI, renderer replay, final document
  templates and the complete offline journey remain M2 work.
