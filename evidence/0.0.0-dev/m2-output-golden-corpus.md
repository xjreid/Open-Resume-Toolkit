# M2 expanded output-only golden corpus

Date: 2026-09-03 local. Platform: macOS arm64. Status: implemented and locally
verified; cross-platform CI and native Word/assistive-technology qualification
remain pending. M2 remains underway.

This medium-reasoning checkpoint expands the output-only regression corpus shared
by the constrained DOCX, pinned PDF, and plain-text renderers. It does not parse
files, access a real profile or vault, enable import, change storage/recovery, or
exercise any HIGH-tagged roadmap item.

## Coverage added

The shared Rust fixture builder now defines eight synthetic cases:

- `standard`: representative contact, entry, field, bullet, and HTTPS links;
- `sparse`: contact-only output;
- `unicode`: supported accented Latin, Greek, and Cyrillic glyph coverage;
- `hostile`: Typst-like directives and OOXML-like markup preserved as literals;
- `dense`: an exact four-page stress document;
- `optional`: omitted contact and entry values plus an empty section;
- `structured`: multiple sections/entries, an unlabeled custom field, and
  HTTP/HTTPS/mailto relationship ordering with meaningful labels;
- `paginated`: an exact two-page page-boundary case.

The DOCX example emits `.docx`, canonical source `.json`, and `.txt` files. The
PDF example emits `.pdf`, receipt `.json`, canonical source `.source.json`, and
the same `.txt` representation. Reviewed SHA-256 manifests pin all eight DOCX
packages, all eight PDFs, and one shared set of eight plain-text outputs. Entity
identifiers remain deliberately absent from output bytes.

## Automated verification

- Rust determinism tests render every case twice. PDF tests also require the
  exact one/two/four-page counts, while the existing invalid-content, missing-
  glyph, overflow, and literal-data controls remain active.
- The standard-library Python verifier requires exact DOCX and plain-text hashes,
  a 256 KiB text bound, UTF-8/LF canonical form, one terminal newline, exact
  source semantics, omission of internal title/ID, fixed six-part ZIP32 packages,
  CRCs, deterministic metadata, XML allowlists, headings, lists, relationships,
  US Letter geometry, and seven rejecting mutation controls.
- The independent PDF.js verifier requires exact PDF and shared-text hashes,
  receipt identity, fixed page counts, visible text/order, safe text bounds,
  tagged `Document`, heading, list, and link roles as applicable, exact safe URL
  order, and no JavaScript, attachments, forms, open action, or active content.
- The optional pypdf audit covers every case and requires embedded subset
  Libertinus fonts, ToUnicode maps, tagged structure, and no personal/date
  metadata.
- CI runs the DOCX/text and PDF/text verifiers on Linux and on the macOS arm64,
  macOS Intel, and Windows desktop matrix. Results for this uncommitted checkpoint
  are pending.

## Accessibility and rendered-page review

The Documents skill accessibility audit ran over all eight generated DOCX files
after replacing one raw-URL display label with meaningful text. Final result:
zero high, medium, or low findings for every fixture.

The Documents skill's LibreOffice workflow rendered all 12 DOCX pages. Poppler
rendered all 12 PDF pages. Every resulting page image was inspected at readable
resolution for missing glyphs, clipping, overlap, ordering, margins, and page-
boundary loss. The one-page cases, two-page pagination case, and four-page dense
case were legible and complete in both formats.

These checks are synthetic local evidence, not proof of identical layout in
Microsoft Word, ATS behavior, screen-reader navigation, native save dialogs, or
Windows/macOS reader interoperability. The plain layouts are not the final three
template categories. Those native, cross-platform, final-template, and manual
assistive-technology gates remain open.

Reproduce with new output directories:

```sh
cargo run --locked -p ort-documents --example docx_fixtures -- target/docx-review-fixtures
python3 tools/verify-docx-fixtures.py target/docx-review-fixtures
cargo run --locked -p ort-render --example pdf_fixtures -- target/pdf-review-fixtures
node tools/verify-pdf-fixtures.mjs target/pdf-review-fixtures
python3 tools/verify-pdf-fonts.py target/pdf-review-fixtures
```

## CI follow-up

The first pushed run for commit `52fa408` and the bounded-retry follow-up at
`28401b0` reached the quality job after every contract, formatting, JavaScript,
Rust, storage, and DOCX/text check passed. The jobs then failed because npm's
bulk-advisory endpoint returned no report before each process bound; neither log
contained a vulnerability finding. The quality job no longer depends on that
endpoint. A separate SHA-pinned OSV job performs a blocking full scan of the
JavaScript and Rust lockfiles. Diagnosis, authority, and remaining verification
are recorded in `m2-ci-dependency-scan.md`; the repaired CI result is pending.

The user handles commit and push.
