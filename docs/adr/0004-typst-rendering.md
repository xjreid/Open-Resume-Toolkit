# ADR 0004: Pinned Typst rendering boundary

- Status: implemented for the development plain PDF fixture; platform/release gates remain
- Target milestone: M2

## Decision

Use a pinned Typst toolchain and reviewed local fonts/templates for PDF rendering. User content is structured data and never executable Typst source.

## Consequences

`ort-render` embeds Typst/typst-layout/typst-pdf **0.15.1** rather than launching
a CLI. `typst-assets` **0.15.1** supplies only its first six Libertinus Serif
faces to the World. All six are fixed by package checksum and receipt content
hash. No user source, external file/package access, font discovery, current date,
or renderer network interface exists. Content is serialized as typed JSON data
and rendered by `templates/resume/plain_pdf_v1.typ`, never concatenated as source.

PDF.js **6.3.289**, bundled with the frontend and an explicit local module worker,
displays the exact generated PDF bytes on canvas. Fonts render as embedded glyph
paths. The Rust export command consumes an expiring in-memory ticket for those
same bytes. No second rendering or arbitrary webview-provided PDF/path is accepted.

This keeps a small fixed rendering interface but adds a substantial transitive
compiler dependency graph. The renderer is in-process, serialized, document-bound
and has post-layout/output caps; those are not hard CPU/memory/OS isolation.
Import remains disabled. Arbitrary templates, cover letters, images, scripts,
packages, historical replay and release template categories are not supported.

The plain fixture is original repository work (US Letter, 1 in margins, 11 pt
Libertinus Serif, 18 pt name, 12/11 pt headings, English layout language). It is
not a promise of identical DOCX pagination. Unsupported glyphs and clipped or
over-limit layouts fail explicitly. Session receipts identify the document hash
and schema, exact PDF hash/size/page count, engine version, template/font hashes
and generation time; no historical receipt storage migration is claimed.

See [checkpoint evidence](../../evidence/0.0.0-dev/m2-pdf-preview.md) and
[dependency/font provenance](../dependencies/pdf/README.md). Golden and native
platform CI, WKWebView/WebView2, document readers and screen-reader verification
remain distinct gates.
