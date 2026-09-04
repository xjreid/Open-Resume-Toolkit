# ADR 0008: Bounded import source snapshot and envelope preflight

- Status: accepted for the disabled M2 import path
- Target milestone: M2

## Decision

Acquire a native-dialog-selected PDF or DOCX once through a held parent-directory
capability, refuse final-component symlinks and non-regular files, and read at most
10 MiB into a parent-owned snapshot. The file extension selects the expected
format but does not establish it. The parent then validates the same snapshot's
outer envelope before any private staging or worker launch.

PDF preflight accepts only the supported version signatures and requires a
terminal `%%EOF` marker. DOCX preflight walks bounded ZIP central-directory and
local-header metadata without decompressing entries. It rejects ZIP64 and
multidisk archives, unsupported flags/compression, encryption, unsafe or duplicate
names, known active-content parts, missing minimum OPC/Word parts, malformed
header relationships, more than 4,096 entries, names over 512 bytes, and declared
expansion over 100:1. No source path or parser-supplied size controls allocation.

This is format-envelope validation, not document-content parsing. PDF object
processing, ZIP decompression, XML/relationship interpretation, page extraction,
and all other hostile content processing remain inside the future disposable
native worker. The snapshot is never reread from the user path after inspection.

## Consequences

Extension-only claims, selected-file symlinks, read-size growth, obvious archive
bombs, path traversal, encryption, and common executable/embedded DOCX payloads
fail before worker launch. The same fixed byte ceiling is shared by native input
and document inspection, and the source wrapper intentionally has no `Debug`
implementation.

This decision does not provide a private staged handle, OS sandbox, parser,
import command, UI, secure memory erasure, or native containment proof. A malicious
but structurally valid PDF/DOCX remains hostile and must not be parsed in the
desktop process. `IMPORT_ENABLED` remains `false`, and `ort-document-worker`
continues to exit 78.
