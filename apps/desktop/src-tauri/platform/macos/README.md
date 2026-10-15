# macOS boundary

macOS Apple Silicon is the active development target. Native composition lives
in the Tauri modules and `ort-platform`/`ort-macos-lifecycle`; this directory
remains a platform documentation boundary.

Reviewed PDF/DOCX import is implemented with the packaged, signed and
identity-pinned `ort-parser-helper`, running metered-Wasm guests. The earlier
native-parser XPC/whole-process hard-limit design is not the active importer and
must not be enabled as a fallback. A bare desktop build without helper pins
fails closed.

See [ADR 0012](../../../../../docs/adr/0012-metered-wasm-document-parsers.md),
[development packaging](../../../../../DEVELOPMENT.md), and
[the current implementation map](../../../../../docs/development/current-state.md).
Production Developer ID signing, notarization, host identity and release
qualification remain M7 work; passing synthetic XPC probes does not establish
those gates.
