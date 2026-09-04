# M2 retained immutable-published PDF replay

Date: 2026-09-04. Base commit: `9a14985`; implementation changes are
uncommitted. Local verification platform: macOS arm64. Status: implemented and
synthetically verified; native installed-app and cross-platform evidence remain
pending.

## Implemented boundary

- Storage now loads one exact positive immutable publication revision by
  `(profile_id, published_revision)`. Unknown and invalid revisions are not
  replaced by the latest publication.
- The replay request remains one canonical UUIDv7 manifest ID. The WebView
  still cannot submit content, source/revision, renderer choices, PDF bytes, or
  a path.
- Native replay resolves an exact current draft or exact retained immutable
  publication. The renderer must reproduce every stored receipt field before
  any bytes are cached or returned.
- Replay returns bounded deterministic plain text from that same exact source.
  This preserves an accessible review for older publications no longer loaded
  in the editor; the generated client rejects empty, over-256-KiB, malformed,
  or extra-field responses.
- A successful older-publication replay uses the existing ten-minute,
  identity-bound preview cache and no-clobber native PDF export. Export
  rechecks that exact immutable revision rather than requiring it to be latest.
- Draft bodies are still not archived. An obsolete draft receipt remains
  inspection-only, as do manifests whose source was not carried by portable
  backup 1.1. No superseded renderer/template/font binary is bundled.

## Verification

- Focused contract tests: 19 passing, including malformed accessible replay
  payloads.
- Focused desktop frontend tests: 62 passing, including retained-publication
  availability and complete receipt mismatch rejection.
- Focused storage tests: 35 passing, including exact first/second publication
  lookup and no-substitution behavior.
- Desktop Rust tests cover exact older-publication resolution and refusal of a
  superseded draft. The application PDF/storage integration test reproduces an
  older publication after a newer publication becomes current.
- Full `just check` passed after the combined dependency-policy and replay
  changes: formatting, TypeScript, all JavaScript/component tests, production
  builds, source security checks, strict workspace Clippy, and all Rust tests.
