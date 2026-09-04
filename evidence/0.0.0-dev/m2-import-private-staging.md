# M2 private import staging and parser-output boundary

Date: 2026-09-04. Base implementation committed in `7a8cda1`. Local
verification platform: macOS arm64. All document content is synthetic.

## Implemented production slice

`ort-platform::ImportStagingRoot` creates a fixed private `imports` root and a
random UUIDv7 directory per prepared operation. On Unix it verifies `0700`
directory and `0600` file modes, writes only a bounded ownership marker and
`source.bin`, syncs them, reopens the source read-only without following the
final component, and exposes no stage or selected-source path. The held input is
transferred once rather than duplicated, preventing another clone from sharing
and changing the parser's seek position.

Explicit cleanup closes the stage-owned handle and deletes only the two fixed
regular files and exact random directory. Drop retries that same exact cleanup
on ordinary early returns. A bounded startup scavenger inspects at most 128
entries and removes only stages at least 24 hours old with an exact UUIDv7 name,
private directory, two-file inventory, valid bounded marker and matching source
length. Fresh, unknown, symlinked, malformed, tampered and extra entries are
preserved; no recursive or ambient cleanup is used.

`ort-application::document_import` composes source-envelope inspection and
staging over the same owned bytes. Its parser launch interface accepts only the
parent operation nonce, independently selected format and transferred read-only
file. The future execution path orders launch, common supervision, adapter
destruction and stage cleanup; stage cleanup failure withholds extraction. The
public path checks `IMPORT_ENABLED` first and currently cleans without calling
the launcher.

`ort-documents::worker_output` adds the symmetric parser-side extraction builder.
It retains no block before validating page ordering, block count, per-block and
aggregate character counts, supported controls and allocation. It emits one
bounded versioned message and passes that message back through the independent
parent decoder before returning bytes. Debug output contains counts only.

## Focused validation

Strict Clippy and all `ort-documents`, `ort-platform`, and `ort-application`
tests pass locally. New adversarial cases cover wrong extension-only content,
private modes, read-only single transfer, duplicate transfer refusal, exact and
Drop cleanup, preservation of fresh/unknown/tampered entries, expired cleanup,
broad roots, symlinked roots, disabled launch refusal, launch failure cleanup,
successful supervision composition, invalid page order/controls, output limits,
whitespace-only extraction and debug redaction.

`just check` then passed Prettier, TypeScript lint, all frontend/extension
production builds, static web/secret checks, workspace Rustfmt, strict Clippy,
103 JavaScript tests and 169 Rust tests. One explicitly gated OS-vault test
remained ignored. The inert worker regression passed with exit 78. Contract
regeneration completed with no generated-file drift; `git diff --check` passed.

## Linux CI failure and repair

The Ubuntu quality job for `7a8cda1` failed four application tests while
creating a private stage. `cap-std 4.0.3` opens capability directories with
Linux `O_PATH`; calling `sync_all` on that descriptor returns `EBADF`, which the
content-free boundary correctly reduced to `Staging(Unavailable)`. macOS lacks
that descriptor behavior, so the same tests passed locally and in the macOS
jobs.

The repair keeps the capability directory for relative operations and also
opens a no-follow, device/inode-matched read-only directory handle solely for
durable directory synchronization. Stage creation, partial cleanup, explicit
cleanup, Drop cleanup, and the expired-stage scavenger use that verified sync
handle. This does not relax modes, symlink refusal, exact inventory, or cleanup
failure semantics. The focused platform/application suites pass locally; an
Ubuntu CI rerun remains required.

## Gates deliberately still open

- Windows staging returns `PlatformSecurityUnavailable` until current-user-only
  ACL creation/verification, reparse defense and installed-package behavior are
  implemented and tested on Windows.
- No desktop command, file-picker token, import state, review UI or startup call
  site is exposed. The all-local-data deletion inventory must include the fixed
  import root before a product call site can create persistent stages.
- Constrained DOCX and pinned PDFium parser libraries now exist behind the inert
  worker; PDFium packaging and native cross-platform verification remain open.
  The worker still exits 78.
- Production macOS XPC/App Sandbox and Windows AppContainer/Job adapters,
  cancellable native pipes, hostile parser corpora, resource/death matrices,
  release signing and supported-platform verification remain mandatory.

`IMPORT_ENABLED=false`; this checkpoint does not make document import available.
