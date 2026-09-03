# M2 restart-staged replace-restore checkpoint

Date: 2026-09-03. Base commit: `0a706b2`. Platform: macOS arm64. Status:
implemented and locally verified with synthetic automated fixtures; native
dialog/vault interaction, cross-platform CI and failure-injected filesystem
evidence remain pending. M2 remains underway.

## Implemented behavior

- A generated main-window-only restore request accepts bounded request metadata,
  one passphrase, and the exact phrase `REPLACE SAVED PROFILE`. It rejects paths,
  merge/destination controls, profile IDs and unknown fields before opening the
  native picker. Passphrase fields clear at dispatch and Rust zeroizes its owned
  value after the one operation.
- Restore shares the single native file-operation/quit gate and the existing
  held-parent, final-no-follow `.ort-backup` reader. Cancellation changes nothing;
  unreadable files, wrong passphrases and malformed authenticated content return
  bounded responses without paths, content, vault references or native errors.
- A selected archive is fully authenticated and imported into a new private
  sibling SQLCipher profile with an independently generated database key and OS-
  vault identity. Domain/schema checks and cipher integrity complete before a
  fixed, content-free restart marker is synced. The open active profile is never
  replaced in place and remains usable until restart.
- Startup detects the marker before opening the active profile. Exact sibling
  directory renames retain the previous encrypted profile as a safety copy and
  promote the staged profile. The marker is removed only after the promoted
  profile opens successfully. Recognized crash states finish promotion, accept an
  already-promoted profile, or restore the old safety directory when staging is
  absent. Symlinks, unexpected entry types, and malformed/oversized markers fail
  closed.
- The renderer receives only cancellation or an exact receipt stating that
  restart is required and a safety copy will be retained. Merge restore is not
  offered. A fixed existing safety/staging/marker slot blocks repeat restore
  rather than overwriting recovery material.

## Verification actually run

- Full local `just check` passed: Prettier, TypeScript lint, Node/Vitest suites,
  frontend/extension builds, web-security and secret scans, Rust formatting,
  strict workspace Clippy with all targets/features, and every workspace test.
  The opt-in OS-vault mutation test remained ignored as designed.
- `cargo test --locked -p ort-domain -p ort-storage -p ort-desktop` passed. New
  storage integration tests prove that staging leaves the live draft unchanged,
  activation restores backup records under a different vault reference, and the
  previous profile remains reopenable from the retained encrypted safety copy.
- A recovery test simulates interruption after the old profile moved and staging
  disappeared; startup restores the previous profile and removes the marker.
  Unix tests reject a broken marker symlink and a marker over the fixed bound
  without moving the active directory.
- Domain/native tests cover the exact confirmation phrase, refusal of renderer-
  supplied paths/merge controls, and stable detail-free error mapping.
- Contract and desktop Vitest suites passed. They enforce the exact staged receipt,
  reject false/extra fields, verify the passphrase/confirmation-only invocation,
  and check restart/safety-copy disclosure. The desktop TypeScript/Vite build
  also passed.

## Remaining gates

- Run all four CI jobs, then exercise installed signed macOS and Windows builds
  with native Open dialogs, Keychain/Credential Manager, aliases/reparse points,
  locked vaults, removable/network filesystems, and assistive technology.
- Add deterministic fault injection around marker creation, directory sync and
  both renames; exercise low/free-space exhaustion and process termination at
  every phase on APFS and NTFS. Current synthetic tests cover representative
  observable crash states, not every OS failure.
- Design visible safety-copy inventory, explicit rollback and bounded retention/
  deletion. Until then one retained safety slot blocks a second restore. Failed
  promoted-profile inspection can leave a separately keyed staged directory for
  support rather than deleting recovery evidence automatically.
- Expand locked/zeroized handling for parsed plaintext domain allocations. Parser
  containment/import UI, destructive all-local-data deletion, final templates and
  the complete offline journey remain separate M2 work.
