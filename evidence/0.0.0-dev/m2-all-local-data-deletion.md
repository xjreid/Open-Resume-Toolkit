# M2 all-local-data deletion checkpoint

Date: 2026-09-03. Base commit: `104d8aa`. Implementation changes are
uncommitted. Local verification platform: macOS arm64. All fixtures are
synthetic. Status: implementation and synthetic automated verification pass;
native destructive-path and cross-platform evidence remain pending. M2 remains
underway.

## Implemented behavior

- A generated, main-window-only command requires the exact phrase `DELETE ALL
  LOCAL ORT DATA`. Its payload accepts no path, profile identity, category,
  external-backup flag or safety-copy exception. Its response exposes only a
  completed/fresh-profile boolean or committed-cleanup/restart state.
- The shared native-operation gate prevents overlap with export, backup,
  validation, restore, PDF rendering/export or safety-copy actions. Desktop
  storage is mutex-owned so save/publish/load calls finish or fail before the
  command takes the store offline and closes SQLCipher/WAL handles.
- Before commitment, the storage layer validates the active root, every fixed
  recovery directory, every manifest and a closed allowlist of exact regular
  filenames. Unknown entries, symlinks, invalid manifests and unsafe reserved
  root names fail before any key or file is removed.
- A private, synced fixed-name intent marker commits deletion before vault or
  profile mutation. Recovery resolves all distinct manifest-derived vault
  references before removing any manifest, deletes credentials idempotently,
  removes only recognized profile/database/recovery files, removes a pending
  restore marker, and deletes the intent marker last. Startup runs this recovery
  before safety-copy recovery or profile creation.
- A vault/filesystem failure after commitment produces `cleanup_pending` and
  blocks storage until restart. A pre-commit failure attempts to reopen the
  unchanged profile and is never reported as completed. A successful operation
  creates a fresh encrypted profile with new installation/profile identities.
- The frontend clears old editor state and native PDF bytes after commitment.
  Its accessible danger section names included M2 data, unsaved-edit loss, lack
  of ORT recovery, preserved external exports/backups, and the fact that the
  application is not uninstalled.

## Verification actually run

- Strict workspace Clippy with all targets/features passed after the desktop
  integration.
- Focused `ort-domain`, `ort-storage` and `ort-desktop` Rust tests passed. Storage
  cases cover active/safety/staged profiles with distinct vault keys, restore
  marker removal, reset identities, empty fresh data, external backup and
  unrelated-sibling preservation, simulated vault-delete failure followed by
  startup resumption, malformed deletion markers, reserved-target symlinks and
  unknown profile-entry refusal.
- Contract generation emits separate deletion request/response schemas. Rust and
  TypeScript validators enforce the exact phrase and exact outcomes and reject
  paths, profile IDs, category controls and extra response fields.
- Contract Vitest passed 18 tests; desktop Vitest passed 50 tests. UI/client
  cases verify the confirmation-only request, disclosures, destructive styling,
  committed-versus-unstarted/unknown feedback and quit waiting while deletion is
  active. The desktop TypeScript/Vite production build passed.

## Retention and deletion boundary

- Included now: active M2 profile records, settings, diagnostics, render
  manifests, unsaved UI state, in-memory PDF preview, fixed staged restore,
  retained safety profile, pending safety cleanup, restore/deletion markers and
  every database key identified by those profile manifests.
- Never targeted: user-selected `.ort-backup`, PDF, DOCX or text exports;
  unrelated sibling files/directories; application binaries.
- No provider credential, Codex session, native-IPC secret, workspace/import or
  drag-out file exists in the current M2 build. Each later feature must extend
  the exact deletion inventory and its adversarial tests before release.
- Crash-abandoned export staging under a user-selected destination is not scanned
  or automatically deleted because ORT cannot safely re-establish ownership.

## Remaining release gates

- Run all four CI jobs and native installed-app tests on supported macOS and
  Windows builds, including Credential Manager/Keychain denial or lock, NTFS
  reparse points, APFS symlinks, process termination after marker/key/file
  phases, low disk, directory durability, and fresh-profile creation failure.
- Verify keyboard and screen-reader operation and that old content disappears
  immediately from the WebView and process-owned PDF cache.
- Re-run the complete offline journey after parser containment/import is enabled.
  This checkpoint does not enable import (`IMPORT_ENABLED=false`) and makes no
  stable-release claim.
