# M2 constrained DOCX export checkpoint

Date: 2026-09-03 UTC (2026-09-02 local). Base commit: `e978cfe`.
Local platform: macOS arm64. Status: implemented and locally verified;
cross-platform CI and native reader/dialog checks pending. M2 remains underway.

Follow-up: the user reported three of four jobs passing for commit `e349856`
and supplied the Windows failure log. Rust tests passed; the independent DOCX
golden check failed because embedded XML was checked out with CRLF. The local
reproduction, scoped checkout fix, stronger regression checks and Node-action
runtime update are recorded in `windows-docx-checkout.md`. The repaired CI result
is pending; the original verification below is preserved as checkpoint history.

Before implementation, the user confirmed all four jobs passed for `e978cfe`.
That confirmation was not independently retrieved. This checkpoint groups the
output generator, command/contracts, editor controls, native write policy,
encrypted-storage integration, independent package tests and documentation.
No production import, M3 work, real profile access or OS-vault test was enabled.

## What works

- The main window can select plain text or Word document output, then export
  an exact saved draft revision or latest immutable published snapshot. DOCX
  uses its own fixed `export_resume_docx` command; no path, bytes, arbitrary
  format, template or overwrite authority crosses the renderer request boundary.
- Both formats use one shared native export lease and quit wait. Saved content
  is captured before the native dialog; further edits cannot replace it.
  Stale/missing revisions fail before selection. Cancellation and failures do
  not modify the draft/snapshot or pause autosave. Uncertain results never retry.
- The native destination capability now fixes the extension and byte limit by
  command. Text stays at 256 KiB; DOCX uses 2 MiB. New `.docx` files use the same
  held-directory, private sibling staging and atomic no-clobber link publication.
  Existing files, directories, symlinks, reserved names and raced targets remain
  refused. Post-commit cleanup/durability warnings remain distinct from failure.
- DOCX v1 is the plain `plain_docx_v1` layout: six fixed OPC parts, ZIP32/store,
  CRC32, deterministic timestamps/order, semantic heading styles and real bullet
  numbering. XML text/attribute escaping prevents markup/field injection.
  HTTP/HTTPS/mailto links show their destination and have explicit hyperlink
  relationships. No link is fetched or opened by the exporter.
- Normalized professional text, ordering, line breaks, tabs and custom fields
  are preserved. Internal title/IDs, author metadata, macros, field codes,
  embedded files, images and external templates are absent. Invalid documents,
  XML-invalid characters, unsupported/whitespace-containing links, empty output,
  per-buffer XML expansion over 1 MiB and oversized packages fail closed.

This generator writes fixed OPC parts directly; it is not a ZIP/XML reader or
hostile-file parser. Microsoft documents the underlying
[WordprocessingML structure](https://learn.microsoft.com/en-us/office/open-xml/word/structure-of-a-wordprocessingml-document);
the output-only store encoder follows the local/central/end record layouts in
[PKWARE APPNOTE 6.3.10](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT).

## Verification actually run

- Full local `just check`: Prettier, TypeScript, Node/Vitest tests, desktop and
  extension frontend builds, web/secret checks, Rust formatting, workspace
  Clippy with all targets/features and warnings denied, and workspace tests.
  The pre-existing opt-in native vault test remains ignored as designed; no
  test was removed or weakened. Contract regeneration was also checked.
- Focused native-command tests cover shared lease release, stale/missing
  revisions and both formats rendering only their captured saved document.
- Rust DOCX tests cover the five synthetic fixtures, deterministic bytes,
  well-formed XML, fixed parts, escaped hostile text/relationship injection,
  semantic headings/lists, LF/tab preservation, invalid controls/noncharacters,
  unsafe schemes, canonical validation and expansion/package ceilings.
- Native filesystem tests cover DOCX's exact bound, wrong/special extensions,
  target races and cleanup, alongside the existing no-clobber/symlink/held-parent
  tests. A synthetic encrypted-profile restart-to-DOCX test exports distinct
  saved/published versions and proves revisions/content are unchanged afterwards.
- Independent Python standard-library ZIP/CRC/XML verification checks exact
  source paragraph order and text, heading/list semantics, relationship targets
  and IDs, package allowlist and page geometry. Six altered-package controls are
  rejected. Golden SHA-256 values are retained in
  `../../fixtures/documents/docx-v1.sha256.json` and required on every CI runner.
- Five final DOCX fixtures were rendered with the Documents skill's headless
  LibreOffice renderer, then all page images inspected. Standard, sparse,
  Unicode and hostile-text fixtures each use one page; dense output uses four.
  Layout is plain and readable, without clipping/overlap. It is not proof of
  identical layout in Word or a complete language/accessibility matrix.
- The standard fixture's structural accessibility audit reported no findings.
  Actual VoiceOver/NVDA and document-reader navigation remain manual gates.

Generated synthetic DOCX/source files and QA pages are ignored under
`target/docx-final-fixtures`. The first render exposed a missing CJK fallback in
the headless runtime. Final output uses a SimSun East Asian font reference;
render QA explicitly points `FONTCONFIG_FILE` to a task-local configuration
listing `/System/Library/Fonts` and its `Supplemental` directory with a writable
task-local cache. The final Unicode page visibly includes Chinese/Japanese,
Greek and accented Latin text. No font was installed, copied into ORT or embedded
in DOCX. Reader font substitution is still disclosed in the UI.

Reproduce structural checks without a desktop/credential prompt:

```sh
cargo run --locked -p ort-documents --example docx_fixtures -- target/docx-review-fixtures
python3 tools/verify-docx-fixtures.py target/docx-review-fixtures
```

The output directory must be new. All four CI jobs now run the independent
verification; their result for this checkpoint is pending. These CI checks do
not run the headless visual inspection or Windows-native UI/vault matrix.

## Dependency and authority review

`crc32fast =1.5.1` (MIT OR Apache-2.0) is now a direct output dependency;
`quick-xml =0.41.0` (MIT) is test-only. Both versions/checksums already existed
in Cargo.lock; no new registry package or version was introduced. SPDX values
were checked in downloaded registry manifests. The small ZIP writer supports
only stored output and accepts only internal fixed part names. There is no
compression, encryption, ZIP64, arbitrary output path or archive input surface.
The generator/layout is original repository code, without copied upstream
resume template source/assets. Fonts are references, not distributed assets.

No Tauri capability, SQLCipher setting, native containment probe, signing
entitlement or application identity changed. `.cargo/config.toml` still enforces
`SQLCIPHER_OMIT_LOG` and `SQLCIPHER_OMIT_DEFAULT_LOGGING`; encryption and memory
protection remain enabled. No desktop, Word, Keychain or Credential Manager was
launched for this checkpoint. LibreOffice was used headlessly for synthetic QA
only, not installed or used by the product. RustSec/transitive license/SBOM and
signed-package review remain release work; this is not a security certification.

## Still gated / manual follow-up

1. Confirm the four CI jobs after committing/pushing this checkpoint.
2. In the development app, select DOCX, export saved and published variants to
   new names, cancel once, and attempt an existing filename. Confirm accurate
   notices and unchanged editor content; test guarded quit during the dialog.
   This new DOCX-specific native UI smoke check was not performed locally.
3. Check the generated DOCX in native Word/readers on macOS and Windows, including
   links, CJK/RTL/fallback fonts, pagination, heading/list navigation and screen
   readers. Windows-native dialog/ACL/reparse/filesystem and vault proofs remain
   separate from CI compilation and synthetic filesystem tests.
4. Final template categories, Typst preview/PDF, historical renderer records,
   confirmed replacement, export crash recovery, storage UI, and the complete
   offline journey remain M2 work. A plain DOCX receipt is not renderer history.
5. Hostile-file import stays disabled (`IMPORT_ENABLED=false`; worker exits 78).
   Supervisor/client-death, inherited authority, full-tree/resource/credential/
   broker and Windows containment gates remain unchanged.

No migration is needed. Rollback can remove the DOCX command/selector and generator
without touching saved records or user exports. The user handles commit/push.
