# ADR 0010: Constrained DOCX parsing stays inside the disposable worker

- Status: accepted as disabled production parser code
- Target milestone: M2

## Decision

Implement the first content parser in `ort-document-worker`, a crate that the
desktop and application layers do not depend on. The parser accepts an already
open `Read` handle, takes at most the shared 10 MiB source ceiling, independently
re-runs the DOCX envelope check, and never accepts a path, URL, command, output
location or caller-selected package part.

The worker recognizes ZIP32 store and deflate entries only. Before parsing it
checks central/local metadata, names, sizes, CRC-32 and optional data descriptors.
It inflates only `[Content_Types].xml`, `_rels/.rels`, `word/document.xml`, and an
optional `word/_rels/document.xml.rels`, each under a fixed part ceiling. The
package must declare the non-macro DOCX main content type and a single internal
root relationship to `word/document.xml`.

XML processing is streaming and bounded by depth, event, relationship, block,
character and final-message limits. DTDs, processing instructions, CDATA,
unknown entity references, active WordprocessingML elements, active relationship
types, unsafe internal targets and external targets other than visible
`http`/`https`/`mailto` hyperlinks fail closed. Relationship targets are checked
as inert metadata and are never resolved or fetched. Deleted text is ignored;
paragraphs, the top-level built-in heading style, numbering/list hints, explicit
breaks and tabs are emitted in source order through the independently checked extraction-v1
builder.

DOCX has no trustworthy fixed pagination without layout. This parser therefore
reports one logical page and makes no pagination-fidelity claim. Image-only or
whitespace-only documents return the existing no-readable-text/OCR-unavailable
result.

## Consequences

This provides real DOCX parsing code and exporter-to-parser compatibility, but
does not make it safe to invoke outside OS containment. The shipping worker
entry point still exits 78 and `IMPORT_ENABLED` remains false. Production use
still requires native macOS and Windows containment adapters, bounded pipe
drivers, resource and lifecycle proof, real-format/fuzz corpora, UI review and
supported-platform qualification.

PDF parsing remains separately gated. ADR 0011 subsequently selects the exact
PDFium build and parser limits; provenance-aware packaging, cross-platform native
verification and containment are still required. There is no in-process PDF
fallback.
