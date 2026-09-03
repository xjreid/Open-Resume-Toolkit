# M2 retained safety-copy management checkpoint

Date: 2026-09-03. Base commit: `0a706b2`, with the preceding restart-staged
replace-restore checkpoint in the same uncommitted working tree. Platform: macOS
arm64. Status: implemented and verified with synthetic automated fixtures;
native vault/UI inspection, failure injection and cross-platform CI remain
pending. M2 remains underway.

## Implemented behavior

- A generated, main-window-only empty request returns only whether a fixed local
  safety copy exists, a replacement/rollback awaits restart, or confirmed cleanup
  is pending. It scans no external location and exposes no path, profile/vault
  identity, size, record inventory, timestamp or content.
- Rollback requires the exact phrase `ROLL BACK SAVED PROFILE`. The storage layer
  refuses missing, unsafe or overlapping recovery state, opens and verifies the
  retained SQLCipher profile without creating one, and makes a verified encrypted
  checkpoint in the existing staging slot. Only then does it sync a bounded
  `rollback_ready` marker. The active profile remains unchanged until restart.
- Startup removes the redundant original safety directory while retaining its
  key for the staged checkpoint, moves the current profile into the safety slot,
  and promotes the checkpoint through the previously tested restart journal. The
  previously active profile therefore remains available after rollback rather
  than being discarded.
- Permanent cleanup requires `DELETE SAFETY COPY`. The exact retained directory
  is verified and renamed to a fixed deletion-pending slot before mutation. Its
  manifest resolves the one vault key; deletion is idempotent, and only known
  database sidecars/manifest files are removed. Startup completes an interrupted
  confirmed cleanup before opening the active database. The active profile and
  user-controlled exports/backups are never targeted.
- Status, rollback and cleanup share the native file-operation/quit gate where
  mutation occurs. Responses and stable errors contain no filesystem, vault or
  profile details. Accessible UI copy distinguishes reversible rollback from
  permanent safety-copy deletion and requires separate typed confirmations.
- Both destructive paths compare the retained and active vault references before
  staging or deletion. A malformed same-identity safety directory fails closed,
  preserving the active key, active data and retained directory.

## Verification actually run

- Focused strict Clippy passed for the full workspace.
- Focused `ort-domain`, `ort-storage` safety-action and `ort-desktop`
  backup-command tests passed. The storage suite now includes rollback exchange,
  exact cleanup and startup-resumed cleanup tests in addition to the
  replacement/crash cases.
- Contract and desktop Vitest suites passed (17 and 47 tests respectively), and
  the desktop TypeScript/Vite production build passed. Validators reject extra
  path fields and false/malformed action receipts; command-client tests verify
  empty status and confirmation-only action payloads.
- The complete local `just check` gate passed after generation and formatting:
  Prettier, TypeScript lint, all pnpm tests/builds, web-security and secret scans,
  Rustfmt, strict workspace Clippy and all workspace/all-target Rust tests. The
  storage suite passed 28 tests, including the same-vault-identity guard.

## Remaining gates

- Run all four CI jobs. Exercise installed signed macOS and Windows builds
  against Keychain/Credential Manager, including locked vaults, reparse/symlink
  cases, native keyboard/screen-reader operation and quit during preparation or
  cleanup.
- Add deterministic failures at checkpoint, marker, rename, vault-delete,
  known-file-delete and directory-sync boundaries. The restart cleanup test
  covers one observable interrupted phase, not every APFS/NTFS failure.
- Decide bounded automatic retention/age policy and whether the UI should expose
  safety-copy creation time or byte use without weakening the content-free
  boundary. Full active-profile deletion remains a separate destructive M2 task.
- Parser containment/import UI, final templates, historical renderer replay and
  the complete offline journey remain separate M2 work.
