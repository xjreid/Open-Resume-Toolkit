# Step 5 macOS export crash cleanup

Date: 2026-09-07. Development source based on M1-complete `65518eb`.
This is focused implementation evidence; Step 6 has not begun.

## Implementation

All macOS export formats use the same one-use held-directory capability.
The publisher creates an empty mode-0600 file inside a mode-0700 sibling
directory, unlinks it, verifies zero links, syncs the staging directory,
removes that directory and syncs the selected parent. Only then may it write
document bytes through the retained descriptor. After syncing those bytes,
`fclonefileat` publishes the complete file under the selected new name. The
kernel releases the unnamed staging inode when its last descriptor closes,
including on process termination. No startup scan or ownership guess is added.

Publication still refuses existing files, links and directories, including a
target created after the last absence check. The parent capability remains
attached to the selected directory if its ambient path is renamed/replaced.
No copy, overwrite or named-plaintext fallback is permitted. Post-publication
directory-sync failure retains the existing durability-unconfirmed receipt;
successful publication has no named staging payload left to clean up.

The safe API comes from the already locked macOS rustix 1.1.4 dependency and
its existing `fs` feature. No new package, FFI exception, entitlement, credential
access or installed-build replacement is involved. Apple documents descriptor
cloning and its destination-exists/unsupported-filesystem behavior in the
[clonefile manual](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/man/man2/clonefile.2).

## Focused regression coverage

- Separate test processes receive SIGKILL at five exact boundaries: empty
  named staging, completed unlink/sync, partial payload, flushed payload and
  publication. The parent checks actual directory contents after child death.
  Empty staging contains zero bytes; the next three boundaries leave no named
  files; publication leaves only the exact complete synthetic export.
- A last-moment competing target is preserved and the unnamed payload released.
- Removing the held parent before publication produces a bounded failure with
  no fallback output elsewhere.
- Existing shared tests cover exact PDF/DOCX/backup/text bytes, mode 0600,
  format-specific size bounds, invalid names, symlink refusal, cleanup and
  parent-path replacement.

Full workspace Rust tests passed: 215 passed, 9 explicitly gated native tests
ignored, zero failures. Strict workspace/all-target/all-feature Clippy passed.
Logs: `target/m2-unlinked-export-workspace-tests.log` and
`target/m2-unlinked-export-clippy.log`. The canonical `CI=true pnpm check` was
initially blocked before execution by automatic approval review reporting the
account's usage limit. On continuation, the retry passed formatting, lint,
tests, builds, security checks and the 728 Rust / 167 JavaScript dependency
license inventory. Log: `target/m2-unlinked-export-web-check.log`.

## Limits

Local tests use macOS arm64 and the local APFS temporary volume. macOS export
now requires file cloning and directory sync; external, network or other
filesystems lacking either fail closed. This compatibility restriction must
be included in final filesystem qualification. File mode is preserved, but
cloning inherits destination-directory ACLs; this is not a claim to override
permissions deliberately attached to the user's chosen export folder.

SIGKILL evidence does not simulate power loss, disk exhaustion, journal
recovery, arbitrary filesystem behavior or native save-dialog interactions.
Unlinking is not secure erasure. A crash before unlinking can leave empty
staging names; old-version plaintext leftovers are neither discovered nor
removed. Other platforms retain the previous named staging implementation.
The installed M1 app and previously staged native editor candidate predate
this change. Full Step 6 native qualification remains pending.
