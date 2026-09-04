# M2 bounded import source-envelope checkpoint

Date: 2026-09-04. Local verification platform: macOS arm64. All documents and
package entries used by tests are synthetic.

## Implemented bounded slice

`ort-platform::read_native_document` now acquires only an absolute,
native-dialog-selected `.pdf` or `.docx` regular file through a held
parent-directory capability. It opens the final component without following a
symlink, compares bounded metadata around the open/read, and returns one
parent-owned snapshot of at most 10 MiB. Its source wrapper does not implement
`Debug`. The extension supplies an expected format, not proof of content.

`ort-documents::import_source::inspect_source` validates that exact snapshot
without decompression. PDF input requires a supported `%PDF-1.0` through
`%PDF-1.7` or `%PDF-2.0` header plus terminal EOF. DOCX input requires a bounded,
single-disk, non-ZIP64 ZIP envelope with matching local/central names, flags and
compression methods; safe unique UTF-8 package paths; required content-types,
root-relationships and Word-document parts; no encryption or known macro,
ActiveX, embedding or ink parts; at most 4,096 entries; 512-byte names; and a
declared aggregate expansion ratio no greater than 100:1. Compressed payloads
must end before the central directory. The source byte ceiling is shared from
`ort-domain` so native acquisition and inspection cannot drift.

The existing eight-case deterministic DOCX output corpus passes this same source
preflight. Adversarial tests cover wrong signatures, unsupported PDF versions,
missing/trailing PDF EOF data, empty/oversized sources, selected symlinks,
unsupported/double extensions, path traversal, absolute/backslash/empty/dot
components, duplicate names, encryption, active parts, expansion bombs, missing
required parts, truncation, multidisk metadata and ZIP64 sentinels.

## Validation

Targeted strict Clippy and all `ort-documents`/`ort-platform` tests passed.
`just check` then passed Prettier, TypeScript lint, all frontend/extension
production builds, static web/secret checks, workspace Rustfmt and Clippy with
warnings denied, 103 JavaScript tests and 155 Rust tests. One explicitly gated
OS-vault test remained ignored. The inert-worker regression passed with exit 78
and no import path enabled. Contract regeneration completed with no generated
file drift; `git diff --check` passed.

## Import remains disabled

This code performs bounded acquisition and outer-envelope validation only. It
does not decompress DOCX, walk relationships, parse XML/PDF objects, extract a
page, launch a worker, create private staging, expose a desktop command, or
mutate a profile. The inspected snapshot remains sensitive ordinary memory and
is not claimed to be securely erased. `IMPORT_ENABLED=false`; the worker remains
inert and exits 78.

## Remaining native and parser gates

- Create the randomized private import staging object and pass only its held,
  read-only descriptor/handle to the worker; prove ACL/mode, ownership,
  symlink/reparse resistance, cleanup on every outcome/startup, parent death and
  no cross-operation source/result leakage on installed macOS and Windows.
- Implement the release-signed macOS XPC/App Sandbox and Windows
  AppContainer/Job adapters plus bounded cancellable pipe drivers. Complete all
  filesystem, credential, IPC, broker, network, child, resource, death and
  process-tree verification matrices in release packages.
- Add pinned PDFium and constrained DOCX decompression/XML parsers only inside
  that boundary. Enforce page, relationship, nesting, image/object,
  decompression, memory, CPU, thread/handle and wall limits with hostile/fuzz
  corpora on every supported OS/architecture.
- Revalidate the worker extraction in the parent, build import review UI, and
  prove cancellation/failure cannot mutate canonical state. Run the complete
  offline import journey, accessibility and native interaction matrices.

Until every applicable gate passes, import and release advertising remain
disabled.
