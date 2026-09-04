# M2 constrained DOCX worker parser

Date: 2026-09-04. Implementation changes are uncommitted. Verification uses
synthetic documents only on macOS arm64.

## Implemented boundary

`ort-document-worker::extract_docx` is a real parser over an already-open handle,
but the executable remains inert. It reads at most 10 MiB, independently repeats
the source-envelope check, validates ZIP local/central records, CRC-32 and data
descriptors, and inflates only four fixed package metadata/content parts under
separate ceilings. ZIP64, encryption, unsafe names, excessive declared expansion,
active parts and unsupported methods remain rejected by the shared envelope.

The parser requires the standard non-macro DOCX main content type and exact root
document relationship. The streaming XML layer rejects DTD/PI/CDATA, unknown
entities, excessive depth/events, active elements and relationship types,
unsafe internal targets, duplicate relationship IDs and non-hyperlink external
relationships. Allowed hyperlink targets are treated as inert visible metadata
and never fetched. Deleted text is excluded. Top-level headings, paragraphs,
list hints, Unicode, entities, breaks and tabs pass through the bounded extraction-v1 builder
and independent parent decoder.

Because DOCX pagination depends on a layout engine, the adapter reports one
logical page. Empty/image-only content returns no-readable-text and makes no OCR
claim. Parser and error Debug output contain no extracted content.

## Local validation

Focused strict Clippy passes for every document-worker target. Unit/adversarial
coverage includes stored and deflated packages, semantic ordering, Unicode and
entity decoding, deleted text, allowed hyperlinks, file targets, active/internal
relationships, macro content types, redirected root relationships, active XML,
DTDs, excessive nesting, CRC corruption, oversized input, whitespace-only input
and redacted diagnostics. An integration test renders the shipping fixed DOCX
export and parses it back through extraction wire v1.

`just check` passes Prettier, TypeScript lint, frontend/extension production
builds, static web/secret checks, workspace Rustfmt, strict Clippy, 103
JavaScript tests and 176 Rust tests. One explicitly gated OS-vault test remains
ignored. The inert-worker regression still passes by observing exit 78.

## Remaining gates

- The worker executable still exits 78 and import remains false. No desktop or
  application crate invokes this parser.
- Production macOS XPC/App Sandbox and Windows AppContainer/Job adapters,
  bounded native pipe readers and full kill/reap/parent-death proof remain open.
- Windows private staging ACL/reparse implementation and native verification are
  still required.
- Real third-party DOCX fixtures, differential/fuzz corpora, memory/CPU/handle
  fault injection, accessibility review and both-platform signed-package runs
  remain release gates.
- PDFium selection, pin/hash/license review and the bounded text adapter are now
  recorded separately in `m2-pdf-worker-parser.md`; packaging and native
  cross-platform verification remain open.
