# M2 No-AI import core checkpoint

Date: 2026-09-02. Base commit: `7577e56`. Implementation changes are uncommitted.
Local verification platform: macOS arm64. All fixtures are synthetic.

## Implemented scope

- Parent-side extraction wire v1 decoding with explicit format/version, page,
  collection, character, byte, control-character, and unknown-field boundaries.
  Empty/whitespace-only extraction returns no-readable-text. These checks are
  not proof of the parser's sandbox, content extraction, pagination, or type.
- Deterministic mapper v1 with a conservative multilingual heading alias table,
  explicit contact-label detection before the first section, custom headings,
  and literal text/list fallbacks. All original source blocks remain immutable
  and indexed; blank, ambiguous, unknown, and overlong blocks are accounted for.
- In-memory review with no default acceptances. Explicit decisions can edit or
  reclassify content, create/merge sections, keep both sections, move text, keep
  or replace conflicting contact values, or reject content. Rejecting a parent
  cannot silently discard accepted children.
- Whole-candidate validation and exact base revision/document checks before
  preparation of a save payload. No review method writes storage or publishes.
  Existing content/IDs survive merges; accepted additions get fresh UUIDv7 IDs.
- Redacted debug/error output and bounded retained decision text. No new external
  package/version was introduced: lockfile changes only connect existing crates.

## Automated evidence

`cargo test --locked -p ort-documents -p ort-application -p ort-document-worker`
passes locally, including:

- Golden multilingual/unknown-section mapping and exact source-block accounting;
  repeat mapping is deterministic (new entity IDs are generated only at candidate
  preparation). Literal script/path/shell/prompt-like text has no side effects.
- Malformed UTF-8/JSON, unexpected fields/types, version/format mismatch,
  invalid/out-of-order pages, oversize messages/blocks/collections/total text,
  control characters, empty extraction, and exact text-limit boundaries.
- Mandatory decisions, non-mutating suggestions, invalid item indices, reset to
  pending, keep-both versus explicit merge, preserved existing IDs/headings,
  rejected-parent handling, and deliberate child relocation.
- Existing and within-import contact conflicts, explicit keep/replace, invalid
  destinations, source retention after over-limit/invalid edits, and reject-all
  returning an unchanged candidate document.
- Stale revision/document identity/content refusal. A real temporary SQLCipher
  profile then proves that preparing alone changes nothing, committing saves
  reviewed content, replay is refused, publication stays unchanged, and content
  survives database reopen. A separate test races a later edit after preparation
  and confirms the storage transaction refuses to overwrite it.
- The storage integration uses `MemoryDatabaseKeyVault`, never macOS Keychain
  or Windows Credential Manager and never the user's development profile.
- An inert-worker regression launches the placeholder with an input argument;
  it still returns exit 78, no extraction stdout, and the fixed disabled message.
  No actual document parser is run by this test.

The full local `just check` suite and deterministic contract regeneration are
also checked for this checkpoint; results are recorded in `manifest.json`.
No native UI smoke test is claimed: no frontend/command/capability/installer
change was introduced. The previous text-export preview and its recorded hash
remain the last packaged artifact; they were not rebuilt for this core work.

The macOS arm64/Intel and Windows CI jobs now include these core and inert-worker
tests. The user reported all tests passed for the previous push; the current
changes have not yet run remotely.

Follow-up 2026-09-02: the user reported only windows-2025 failed on the subsequent
push. Its `import_storage` executable aborted with `STATUS_STACK_OVERFLOW`.
See `m2-import-transport.md` for diagnosis and follow-up probes; no Windows fix
is claimed yet. The later bounded transport policy also supersedes the absence
of collection policy below, but there is still no native pipe driver/supervisor.

## Remaining gates and limitations

- PDF/DOCX import is still unavailable in the desktop app. Native file picking,
  cross-platform private staging, supported sandbox primitives, resource-limited
  process supervision, packaged PDFium, production parser invocation, and real
  hostile document fixtures remain prerequisites. Constrained DOCX and pinned
  PDFium text adapters now exist only behind the inert worker. A bounded decoder
  or parser is not a sandbox.
- Native pipe drivers must enforce the transport ceiling before buffer growth;
  only the common supervision policy exists. PDF image-dominant detection now
  uses a documented conservative threshold; real corpus validation remains open.
  Reliable DOCX pagination is not implemented.
- Mapping does not yet infer/split employment or education entries, date ranges,
  skill items, or link destinations. Ambiguous multi-line contact blocks stay
  literal. Unlabeled names are not guessed. This is not a complete importer or a
  claim of comprehensive multilingual support.
- The source/proposal UI, window/session-bound IPC, editable splitting, detailed
  duplicate detection, asynchronous save outcome/replay lifecycle, cancellation/
  expiry, and binary cleanup are not connected. Preparation retains the review;
  the eventual caller must use its exact expected revision in storage and retire
  the session only after confirmed commit/cancellation. A reject-all/no-change
  decision need not create another saved revision.
- Rust/JavaScript core tests do not replace native Windows/macOS containment,
  accessibility, adversarial parser, signed-package, or offline-journey evidence.
  M2, the earlier platform/quit gates, PDF output, and release hardening remain
  unfinished. The application is still development-only and not release-eligible.
