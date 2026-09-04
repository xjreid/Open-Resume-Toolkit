# M2 pinned PDF worker parser

Date: 2026-09-04. Implementation changes are uncommitted. Import remains
disabled. Most validation is synthetic on macOS ARM64; the one native smoke test
uses the pinned macOS ARM64 library and a generated non-sensitive PDF.

## Implemented boundary

`ort-document-worker::extract_pdf` accepts an already-open PDF reader and one
explicit packaged-library path. It repeats the 10 MiB source envelope check,
binds no system library, accepts no password and returns extraction wire v1.
Before loading, the target-specific `PDFium` dynamic library must be an absolute,
regular non-symlink file whose filename, byte length and SHA-256 match the
compiled `chromium/7881` identity. macOS ARM64/x64 and Windows ARM64/x64 archive
and extracted-library identities are recorded in the checked-in manifest.

The adapter uses `pdfium-render` 0.9.3 with default features disabled and only
the `pdfium_7881` API surface. The selected `PDFium` 151.0.7881.0 artifacts have
V8 and XFA disabled. Loading is memory-backed. The adapter rejects zero/more
than 10 pages, over 20,000 top-level objects per page, malformed page/object/text
access, over 50,000 extracted characters and any producer protocol violation.
It does not render, execute script, traverse attachments/forms, fetch URIs or
perform OCR.

Extracted text is preserved in page/definition order, with only line-ending
normalization. Exact known headings and explicit list prefixes receive hints;
all other content remains literal for review. Image/form-bearing pages below 16
non-whitespace characters return a partially-scanned unsupported state. Fully
unreadable input returns no-readable-text. No partial output survives any error,
and diagnostics contain neither extracted text nor a library path.

## Local validation

Focused tests and strict Clippy pass. Adversarial cases cover source/read errors,
engine failure, image-only and partially scanned PDFs, an ordinary logo positive
control, page/object/text/protocol bounds, impossible image/object counts,
Unicode, page ordering, CRLF/CR normalization, headings/lists, relative/wrong
library paths, symlinks, wrong-size/wrong-digest libraries and redacted errors.

The immutable GitHub release metadata supplied exact archive sizes/digests. All
four target archives were downloaded to temporary storage, their SHA-256 values
matched, and the extracted dynamic-library hashes/sizes were recorded. The
macOS ARM64 native opt-in test then verified the library again at runtime and
successfully extracted both expected strings from a synthetic PDF. The test is
ignored by default because native assets are not yet part of normal development
or packaging.

## Remaining gates

- The executable still exits 78, `IMPORT_ENABLED` is false and no application
  crate invokes either parser.
- The release pipeline does not yet fetch/verify the archive attestation, safely
  extract and package the selected asset, retain notices, or verify the final
  signed bundle's library digest.
- macOS x64 and Windows ARM64/x64 native load/parser runs remain pending. Windows
  private staging and AppContainer/Job implementation remain pending.
- Production macOS XPC/App Sandbox invocation, native pipe ownership and the
  full denial/resource/kill/reap/parent-death matrix remain pending.
- Real/fuzz/differential PDF corpora, malformed/encrypted/object-stream cases,
  crash/OOM/timeout fault injection, scanned-threshold product copy, and import
  review accessibility remain release gates.
