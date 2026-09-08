# M2 metered WebAssembly containment candidate

Date: 2026-09-07. User-authorized architecture revision; see ADR 0012.

Wasmi 1.1.0 and SHA-256 are added only as explicit test/example dependencies of
ort-documents. Both were already locked; no third-party version was added.
The normal production document crate still does not instantiate a Wasm parser.
Wasmi defaults are disabled, so this addition does not request WAT parsing,
WASI, hash collections or new std-related packages. Existing workspace feature
unification may enable features already used by Typst; neither feature presence
nor dependency reuse is an independent runtime security audit.

Five checked-in first-party binary-module regressions pass: normal execution and
permitted growth; runtime memory growth/initial-allocation denial; unrefillable
fuel exhaustion; recursion and out-of-bounds memory traps; unresolved host
function refusal. The 512 MiB over-limit attempt is rejected before allocation;
it does not allocate/touch 512 MiB. These are real interpreter tests rather than
mock control receipts. Failures assert trap codes where available.

## Pinned PDFium feasibility

The immutable chromium/7881 release metadata identifies `pdfium-wasm.tgz`:
2,543,846 bytes, SHA-256
`added6e8ac024f71cb61cf2b77a205d178e2bdde2e4048fbcd916f68b7264d56`.
The archive was downloaded to ignored target/ and hash checked before manually
copying only regular JS/Wasm members to that evaluation folder. No archive path
was extracted into the workspace or user profile. The module is 5,233,982 bytes,
SHA-256 `3283857c1d26d4b11c64743deb41390c98733892515efea4e4426cc06496d512`.
Both inspection/execution examples verify the module hash before compilation.
The supplied JavaScript glue is not executed.

The module has 33 function imports. The example implements denied filesystem
syscalls, bad-descriptor WASI replies, an empty environment, a deterministic zero
clock and a bounds-checked guest-to-guest memory copy charged by length against
fuel. Unsupported callbacks trap. Heap expansion returns failure in this
initial probe; the runtime also enforces a 512 MiB memory ceiling, one memory,
one instance, one bounded table, 256 recursion depth and 50 million fuel units.
This generic prototype import wiring must become an exact manifest/signature
allowlist before production use. No network or native host authority is linked.

PDFium initialization, a one-page synthetic PDF load, text extraction, matching
expected text, handle closure and runtime destruction succeeded. The run left
44,645,526 fuel units from the initial 50 million. This is a feasibility result,
not corpus timing qualification or a CPU-seconds measurement. No real document
was parsed and no production/installed app was changed.

Logs: `target/m2-wasm-containment-tests.log`,
`target/m2-pdfium-wasm-inventory.log`, `target/m2-pdfium-wasm-candidate.log`.
The generated artifact remains ignored and is not automatically downloaded by
normal tests. The examples require the exact evaluation artifact separately.

Strict workspace/all-target/all-feature Clippy and the full workspace Rust
tests passed after the examples were refactored. The canonical `CI=true pnpm
check` passed with 111 desktop tests, 26 contract tests and the unchanged
728 Rust / 167 JavaScript license inventory. Logs:
`target/m2-wasm-containment-clippy.log`,
`target/m2-wasm-containment-workspace-tests.log`, and
`target/m2-wasm-containment-web-check.log`. Documentation was added afterward;
`git diff --check` passed.

Remaining: production runtime/ABI, callback authority and fuel audit, DOCX guest,
PDF corpus and output bounds, trusted-host overhead, signed helper/watchdog and
parent-death integration, packaged module verification, complete license/source
bundle, review/audit path and final native qualification. Import remains disabled.
