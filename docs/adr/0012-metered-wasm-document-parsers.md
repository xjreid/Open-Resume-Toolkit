# ADR 0012: Metered WebAssembly document parsers

Date: 2026-09-07
Status: implemented and locally qualified in the signed macOS arm64 candidate; final native/user acceptance pending.
User authorization: broaden containment architecture and revise nonviable plans
rather than waive missing implementation or claim M2 complete.

## Decision

Replace the assumption that an ordinary macOS native parser can be constrained
by hard process-memory and CPU rlimits with a pinned WebAssembly parser running
inside a metered interpreter. Retain a minimal signed sandboxed helper as defense
in depth and an independently owned wall-clock watchdog. The helper runs trusted
runtime/adapter code; PDF/DOCX parsing executes inside guest linear memory.
No JIT, WASI runtime, general filesystem adapter, host environment, clock,
network, native IPC, credentials or process-launch functions are exposed.
Only explicitly reviewed bounded guest-memory operations may be implemented.

Wasmi 1.1.0 is the first implementation candidate. It is already in the locked
Typst dependency graph. The pinned PDFium 151.0.7881.0 release also provides a
Wasm module; preliminary synthetic extraction works with denied OS imports.
Port the constrained DOCX parser to a guest module with a fixed memory ABI.
Document bytes are data, never WebAssembly code. Only bundled modules whose
bytes and import/export signatures match the pinned manifest can be compiled.

## Resource policy changes

The prior macOS requirement of a kernel-enforced 512 MiB whole-process limit and
30 CPU seconds is superseded for this new architecture, not marked passed.
The already implemented native launch-receipt format remains unchanged until
its replacement is implemented; do not manufacture an old receipt for Wasm.

- Parser-accessible linear memory is capped at 512 MiB total, with exactly one
  memory and one module instance per job. This is an enforced guest limit, not
  a total resident-memory claim about the host interpreter.
- Bound runtime stack depth, value stack, table count/elements and module size.
  Module compilation uses only pinned trusted bytes, independent of input data.
  Record host memory overhead and peak behavior separately during qualification.
- Replace CPU-second accounting with a fixed fuel budget selected against the
  supported corpus. An initial evaluation uses 50 million units; this is not
  yet the qualified shipping value or a conversion into CPU seconds. Guest code
  cannot refill fuel. Every host callback must have bounded work and consume
  an appropriate part of the same budget. Never retry parsing with extra fuel.
- Keep the 60-second absolute operation deadline, cancellation, bounded output
  and verified helper cleanup. Fuel exhaustion, traps, oversized output or
  failed cleanup withhold all extraction. No partial import is accepted.

## Why this direction

Local adversarial measurements rejected the native memory setter and showed
ignored SIGXCPU outliving the requested hard CPU ceiling. Those remain valid
negative evidence. Interpreter metering acts below guest code, and the guest
has no system API with which to relax limits or create a descendant.

A small Linux VM is a fallback, not the selected first implementation. Apple's
Virtualization framework supplies configurable guest memory but requires a
kernel/init filesystem and additional packaging/update/licensing work. Guest
memory is also distinct from total host-process overhead. Avoid introducing
that distribution surface unless the interpreter candidate proves unsuitable.

## Initial evaluation evidence (historical)

Five synthetic Wasmi tests verify ordinary execution, allowed growth, rejected
oversized growth/initial memory, fuel exhaustion, stack and bounds traps, and
refusal of missing host functions. A hash-verified PDFium Wasm initialized and
extracted expected text from one synthetic PDF without host file access.
These experiments are not production containment signoff.

Required before enabling import: hardened fixed-module runner, exact import ABI
allowlist, bounded/fueled callback tests, guest DOCX build, supported PDF corpus,
text-order/Unicode parity, packaging/provenance/license records, isolated helper
integration, parent-death and cleanup evidence, resource/cancellation stress,
review/audit/save integration and final native qualification. Existing
`IMPORT_ENABLED` remains false until the revised gate is met.

Sources: [Wasmi runtime](https://github.com/wasmi-labs/wasmi),
[Wasmi metering](https://wasmi-labs.github.io/blog/posts/wasmi-v1.0/),
[pinned PDFium release](https://github.com/bblanchon/pdfium-binaries/releases/tag/chromium/7881),
[Apple Linux VM setup](https://developer.apple.com/documentation/virtualization/running-linux-in-a-virtual-machine).

## Initial 2026-09-07 integration update (historical)

The DOCX memory-only WASI adapter and exact-ABI PDF adapter now exist in
`ort-parser-runtime`; `ort-parser-helper` embeds their bytes. A direct child with
App Sandbox, guest capability denial, a trusted internal watchdog and owned
parent pipes is the implemented replacement candidate for the prior XPC/native
parser adapter. It remains unqualified. PDF corpus evaluation uses fixed 500M
fuel after 50M refused font-heavy fixtures; DOCX remains 50M. Native host overhead
and deadlines still require measurement. No existing native rlimit receipt or
XPC probe is being relabeled as proof for this implementation.

See [integration evidence](../../evidence/0.0.0-dev/m2-wasm-import-integration.md).
The native outside-sandbox verification was blocked by automatic approval review's
usage limit. `IMPORT_ENABLED` remains false until remaining gates are satisfied.

## Implementation completion

The selected direct-child App Sandbox helper has now passed running-code identity,
wrong-identity denial, repeated jobs, cancellation during parsing, malformed input,
parent death and real 60-second input/output-stall tests. Packaging pins both the
signed executable SHA-256 and running CodeDirectory hash before document transfer.
The new desktop path enables import only with both compiled identity pins. The old
native-parser `IMPORT_ENABLED` flag remains false and is not the new path's gate.
The initial 50M PDF budget was replaced by a fixed 500M budget validated against
96 schema/style fixtures; DOCX uses 50M. No request retries with additional fuel.
See [completion evidence](../../evidence/0.0.0-dev/m2-implementation-completion.md).
Final Step 6 native-reader/accessibility/account acceptance is still pending.
