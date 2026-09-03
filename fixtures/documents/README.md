# Synthetic document fixtures

All future PDF, DOCX, and text fixtures must be synthetic, contain no personal data, and include their generation provenance.

## DOCX output corpus

The M2 output-only corpus is defined in
`crates/ort-documents/tests/support/mod.rs`: standard, sparse, Unicode,
code-like literal text, and dense multi-page resumes. Generate it with the
`docx_fixtures` example into a **new** directory, then run
`tools/verify-docx-fixtures.py`. Only built-in synthetic content is used.

`docx-v1.sha256.json` pins deterministic package bytes independently of entity
IDs. Updating the fixture or generator requires explicit review of those
digests and semantic/rendered evidence, not automatic baseline acceptance.
No real resume or hostile input parser is involved.
