# M2 editor checkpoint — native macOS smoke check

Date: 2026-09-02. Platform: macOS arm64. Identity:
`com.openresumetoolkit.dev`. Artifact: the local ad-hoc-signed preview `.app`
listed in `manifest.json`, not a stable release or refreshed DMG.

Only synthetic content was entered through the native desktop UI. Computer-use
access worked after the permission change/restart. No real resume or credential
value was inspected or included in this record.

## Native checks

- Baseline Save/Publish/restart: draft revision 2 and published snapshot 1
  returned independently, with synthetic contact/entry/bullet content intact.
- Debounced autosave: editing the title without pressing Save advanced the draft
  and changed the status from Unsaved to Saved.
- Validation: a blank required title and an executable-scheme URL produced
  inline errors and disabled saving/publication. Waiting did not save the
  invalid URL. Correcting it to a synthetic HTTPS link restored saving.
- Named fields: a synthetic skill label/value and its checked skill flag were
  editable and retained while moving their containing section.
- Reordering: the second section moved to the first position; Tab focused its
  move button and Space activated it. Content stayed attached to its section.
- Undo: removing the skill field and choosing Undo restored its label, value,
  and skill flag. Undo of the invalid title returned to the saved state.
- Reload protection: an invalid unsaved title triggered an explicit discard
  confirmation; Keep editing preserved it until Undo was chosen.
- Publication: snapshot 2 displayed ordered sections, bullet, contact link,
  and named skill as read-only text. A newer draft title did not change it.
  Repeat publication of identical content was disabled.
- Layout: the actual WebView screenshot showed the custom-field controls and
  published text panel without clipping/overlap at the default desktop size.
- Final rebuild/restart: draft revision 9, snapshot 2, the draft-only title,
  reordered sections, synthetic link, and checked skill field all returned.
  Session undo/redo reset as intended. The final preview app passed
  `codesign --verify --deep --strict`; its executable hash is in the manifest.

## Automated checks

`just check` passed formatting, TypeScript, frontend builds, contract tests,
static security checks, strict Rust linting, and unit tests. Separately rerunning
`pnpm generate` left every generated contract file byte-for-byte unchanged. New cases
exercise edits arriving during an in-flight save, paused conflict/transport
recovery, bounded undo/redo, validation and Unicode limits, stable-ID ordering,
escaped published text, idempotent publication, and refusal to publish an invalid
persisted draft. Rust remains the authoritative validation/storage boundary.

## Limitations and next gates

- Native close/quit protection is pending: wait for Saved before quitting;
  invalid/in-flight edits and session undo history are not durable recovery.
- Windows native editor/vault tests and remote CI outcomes are not verified by
  this macOS run. Local success does not establish a green GitHub matrix.
- This is text review, not PDF rendering. Import containment, import review,
  renderer/export, storage management, and the full offline journey remain M2.
- No new remote assets, URL-opening action, network capability, parser, AI,
  browser bridge, updater, or production-profile access was added.
- This checkpoint is not release-eligible and is not a complete security audit.
