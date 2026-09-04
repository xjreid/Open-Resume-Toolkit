# ADR 0011: Pinned PDFium text adapter

- Status: accepted for disabled M2 implementation; runtime enablement deferred
- Date: 2026-09-04

## Context

Initial import promises text-based PDF and DOCX, while OCR is explicitly out of
scope. PDF is a hostile binary format and its mature parser is native code. The
desktop process must not parse it, discover a library from the host, render it,
or accept a parser result without the existing worker protocol limits.

`pdfium-render` offers memory-backed text extraction and explicit API-version
features, but its default feature set follows the latest API and includes image
and thread-safe layers that are unnecessary here. It does not ship the native
library. A separately packaged `PDFium` build therefore needs an exact,
auditable identity for every supported target.

## Decision

Use `pdfium-render` 0.9.3 with default features disabled and only the
`pdfium_7881` API feature. Pair it with the non-V8, non-XFA `PDFium`
151.0.7881.0 artifacts from the immutable `bblanchon/pdfium-binaries`
`chromium/7881` release. `pdfium-manifest.json` pins the release metadata,
archive sizes/digests and extracted dynamic-library sizes/digests for macOS
ARM64/x64 and Windows ARM64/x64. Runtime binding accepts only an absolute path
with the target filename, a regular non-symlink file, exact byte count and exact
extracted-library SHA-256. There is no system-library or latest-release fallback.

The adapter loads the complete, already bounded PDF from memory without a
password. It rejects zero or more than 10 pages, more than 20,000 top-level
objects per page, more than 50,000 extracted characters, malformed page/object
access and any extraction-wire violation. Only text is returned. Rendering,
JavaScript, XFA, form interaction, attachment traversal, URI fetching and OCR
are not used.

Text is normalized only from CRLF/CR to LF and emitted in page order as
line-level blocks. Exact known section aliases become heading hints and explicit
`-`, `*`, or bullet prefixes become list hints; everything else remains literal
paragraph text for user review. An image/form-bearing page with fewer than 16
non-whitespace characters is treated as image-dominant. Entirely unreadable
documents return no-readable-text; mixed readable/image-dominant documents
return partially-scanned. Both are fail-closed OCR-unavailable outcomes.

The public import flag remains false and the worker executable continues to
exit 78. The adapter is linked only into the worker crate and permits one parse
per disposable process because `PDFium` bindings are process-global.

## Consequences

- PDF parser vulnerabilities remain behind, and do not replace, the required
  native containment and supervision boundary.
- Release packaging must reproduce archive verification, safe extraction,
  packaged-library hashing, license notices and signed-bundle verification.
- Updating `PDFium` or `pdfium-render` requires a new reviewed manifest and
  adversarial/native test pass; changing a URL or filename is insufficient.
- The 16-character scanned threshold is conservative and visible as an
  unsupported outcome. Real corpus work may tune it only through a new
  versioned decision without silently enabling OCR.
- Definition order from `PDFium` can differ from visual reading order in complex
  layouts. The review UI must show the original extraction and may not imply
  layout fidelity.

## Verification still required

- Windows ARM64/x64 compile, package, hash, load and hostile-corpus runs.
- macOS x64 package/hash/load and signed/notarized package runs; the current
  native smoke is macOS ARM64 only.
- GitHub build-provenance attestation verification and retained license notices
  in the reproducible packaging pipeline.
- Fuzz/differential and real-world corpus coverage, parser crash/OOM/timeout
  injection, and full native sandbox denial/kill/reap/parent-death evidence.
