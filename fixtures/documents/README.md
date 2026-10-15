# Synthetic document fixtures

All future PDF, DOCX, and text fixtures must be synthetic, contain no personal data, and include their generation provenance.

## M2 output-only golden corpus

The M2 output-only corpus is defined in
`crates/ort-documents/tests/support/mod.rs`. Its eight cases cover standard,
sparse, supported multilingual, code-like literal, dense multipage, omitted
optional data, multi-section/field/link ordering, and explicit pagination.
Only built-in synthetic content is used; this is output verification, not an
input parser or hostile-file corpus.

Generate DOCX/source/plain-text fixtures with `docx_fixtures` into a **new**
directory, then run `tools/verify-docx-fixtures.py`. Generate PDF/source/
receipt/plain-text fixtures with `pdf_fixtures`, then run
`tools/verify-pdf-fixtures.mjs`.

`docx-v1.sha256.json` pins deterministic package bytes independently of entity
IDs. `pdf-v1.sha256.json` pins exact renderer output, while
`text-v1.sha256.json` is shared by both generation paths and proves exact
plain-text parity. Updating a fixture or generator requires explicit review of
all affected digests plus semantic, accessibility, and rendered-page evidence;
baselines are never accepted automatically.

## Reviewed style regression audits

`cargo run --locked -p ort-render --example style_fixtures -- NEW_DIRECTORY`
generates the same eight synthetic cases for technical, professional and modern
PDF/DOCX styles. Run `node tools/verify-pdf-fixtures.mjs
NEW_DIRECTORY/technical/pdf technical` and `python3 tools/verify-docx-fixtures.py
NEW_DIRECTORY/technical/docx technical`, then repeat for professional and modern.
The style argument checks the selected bundled template and independent
content/structure constraints. `python3 tools/check-style-output.py` runs all
three styles with v1/v2 sources and verifies 48 reviewed PDF/DOCX/text pairs
against `styles-v1.sha256.json`. Omitting a style argument still enforces the
original Plain fixture hashes. Hashes never replace semantic or native-reader
qualification; changing a golden requires review of the actual output.
