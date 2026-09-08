# M2 style parity and regression corpus — 2026-09-06

High-reasoning output qualification work on the M1-complete baseline. Step 5 and
native qualification remain incomplete. No renderer, template, source schema,
receipt format, installed application or production capability changed.

## Expanded verification

The style fixture generator now has an explicit `schema-v2` mode. Eight cases per
style exercise year-only, present, expected month/year, start-only, reversed ranges
and an omitted empty date alongside legacy Unicode, hostile-literal, optional,
dense and paginated content. Across v1 and v2, this provides 48 PDF/DOCX/text pairs.

The independent Python DOCX/text audit now formats structured dates using a separate
oracle with hand-written expected cases. It rejects altered date content in an
otherwise well-formed DOCX. The PDF.js audit accepts an explicit v2 mode and verifies
the structured source hash as well as output text/order, receipt, page geometry,
links, tags and absence of active content. Default audit behavior still enforces
all original plain-text, PDF and DOCX golden hashes; v2 mode does not relabel its
output as a legacy golden pass.

`fixtures/documents/styles-v1.sha256.json` records the reviewed local 48-pair hashes,
page counts and template/font hashes. A second generation with fresh opaque entity
IDs matched all output hashes. `tools/check-style-output.py` generates fresh
synthetic directories, runs independent audits and checks the corpus. That exact
command passed locally and is now included in core and target-OS CI jobs. Hosted
results for these uncommitted changes remain pending. These regression baselines
are not a declaration of final native style or accessibility qualification.

## Visual and reader evidence

All 37 generated v2 PDF pages were rasterized with Poppler and inspected in labeled
sheets. No clipping, overlap, lost fields or unsafe execution was observed. Dense
and structured entries can continue onto later pages; exact continuous paginated
editor integration and further presentation refinement remain open.

LibreOfficeDev 26.8.0.0.alpha0 opened all 24 v2 DOCX files using a disposable reader
profile. Text extracted from its rendered PDFs matched the expected text in every
case. Standard, dense and structured outputs for each style were rasterized and
inspected. The reader used Liberation Serif substitution for the DOCX Times New
Roman declaration; direct PDF output uses the pinned Libertinus Serif bundle.
This demonstrates semantic reader interoperability, not identical font metrics,
page breaks, Word/Pages acceptance or native screen-reader qualification.

## Checks and artifacts

- Independent audits: 24 v2 pairs, 24 styled v1 pairs, and eight legacy plain pairs
  passed; original legacy golden hashes remained unchanged.
- Exact CI command: all 48 pairs passed including reviewed regression hashes.
- `CI=true pnpm check`: 91 desktop, 24 contract, 30 tooling and two extension tests;
  TypeScript, formatting, builds, static security and licenses passed.
- 52 Rust renderer/document tests passed; all-target/all-feature workspace Clippy
  with warnings denied and Rust formatting passed.

Logs: `target/m2-style-v2-independent.log`,
`target/m2-style-parity-legacy-audit.log`, `target/m2-style-parity-repeat.log`,
`target/m2-style-parity-ci-command.log`, `target/m2-style-parity-web-check.log`,
`target/m2-style-parity-rust-tests.log`, `target/m2-style-parity-clippy.log`.
Visual review: `target/m2-style-v2-visual/`. Reader results:
`target/m2-style-v2-docx-reader-report.json`. Synthetic corpora:
`target/m2-style-v2-parity/` and `target/m2-style-parity-repeat-v1/`.

No user profile, OS vault or test account was accessed. Nothing was installed,
committed or pushed. Final style/native and editor pagination gates remain open;
sandboxed import and lifecycle completion remain separate High work.
