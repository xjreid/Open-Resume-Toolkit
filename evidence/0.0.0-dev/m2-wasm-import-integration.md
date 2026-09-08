> Latest status: Step 5 implementation and signed candidate assembly are complete.
> Native helper identity/lifecycle checks now pass; the earlier usage-limit block
> below is historical. See `m2-implementation-completion.md` for final results and
> `m2-final-native-acceptance.md` for the separate Step 6 matrix.

# M2 metered parser and desktop import integration

Date: 2026-09-07. Local implementation; **M2 and native import qualification are
not complete**. M0/M1 remain successful; the installed M1 application is unchanged.

## Implemented

- `ort-parser-runtime` executes the first-party constrained DOCX guest and pinned
  chromium/7881 PDFium Wasm with Wasmi 1.1.0, no JIT/general WASI/OS adapter.
  Module bytes are bounded and hashed before compilation. PDFium's 33 import
  signatures have a fixed allowlist, rather than accepting guest-declared types.
- One memory, one instance, one table with at most 20,000 elements, 512 MiB guest
  linear memory, recursion depth 256, value stack 65,536 and bounded module bytes.
  DOCX uses 50 million fuel; PDF uses a fixed 500 million evaluation budget.
  The original 50 million PDF experiment rejected all font-heavy corpus PDFs.
  No production retry refills fuel. Host copies/growth/date conversions consume
  fuel before work. UTC conversion is deterministic and reads no host clock or
  timezone. Native runtime memory/CPU overhead remains to be qualified.
- DOCX receives only job-owned bytes through restricted stdin and emits bounded
  extraction through memory-only stdout. Other descriptors are denied. Stderr
  is counted and discarded. PDF host file operations are denied; unsupported
  exception/mapping imports trap. PDF page/object/character/scanned-page limits
  withhold all output on failure. Guest compilation and extraction do not accept
  modules from a document or renderer.
- `ort-parser-helper` embeds both modules and the build-generated DOCX digest.
  It handles one length-bounded request, disables core dumps, emits only bounded
  extraction, and has a trusted 60-second watchdog checking parent death every
  25 ms. Unpackaged workspace builds contain no guests and exit 78.
- `tools/build-parser-guests.py` verifies the immutable archive/module digests,
  builds DOCX with the pinned Rust target, and retains PDFium's license notices
  and build arguments. It does not execute or distribute JavaScript glue.
  `tools/package-parser-helper.py` creates a development-only ad-hoc App Sandbox
  helper in `target`; it never installs the desktop app or accesses the Keychain.
- `ort-platform::ParserHelper` owns a direct child and nonblocking private pipes,
  shares a single deadline across executable verification and execution, rejects
  output overflow/nonzero exits, and accepts extraction only after both EOFs and
  observed child reaping. Cancellation kills/reaps the owned child. A cleanup
  failure discards extraction. This is a new Wasm policy adapter, not a fabricated
  successful receipt for the superseded native-parser rlimits.
- Native picker/begin/cancel/availability commands, exact draft revision checks,
  exclusive operation/job slots, window teardown cancellation, native-owned
  review creation, and mounted desktop review flow are wired together.
  The helper digest must be compiled into the desktop build and its path is fixed
  below `Contents/Helpers`. The renderer supplies neither executable identity nor
  source path/module authority. `IMPORT_ENABLED` remains false.
- A native empty revision-zero base supports first import without saving a blank
  draft. Explicit review commit performs create-if-absent or revision CAS and a
  content-free accepted/rejected/mapping/revision audit event in one SQLCipher
  transaction. Audit failure rolls back the draft. No source text/path appears
  in the audit. Concurrent draft creation makes an empty-base import stale.

## Local verification

Eight callback-boundary tests cover memory bounds, signature refusal, unavailable
OS descriptors, cumulative stdout/stderr limits, fuel-before-copy, memory growth
refusal and exact UTC writes. Encrypted-storage tests inject an audit failure and
verify rollback; review tests cover empty-profile creation on explicit commit.
The native job-slot test verifies exclusivity, cancellation on teardown and reuse
only after the old lease drops. Frontend tests exercise unavailable import,
picker cancellation, one pending request and cancel/completion races.

- Desktop frontend: 113 tests passed; generated-contract package: 26 passed.
- Workspace strict Clippy passed with all targets/features.
- Dependency policy passed: 730 Rust / 167 JavaScript packages; repository secret
  and web-security checks passed.
- Full workspace all-target tests passed: 246 passed, zero failed, nine explicitly
  opt-in tests ignored (`target/m2-final-workspace-tests.log`).
- All 48 synthetic PDF/DOCX fixtures across the three styles passed extraction and
  normalized content inclusion checks (`target/m2-guest-corpus.log`).
- Repository formatting and frontend TypeScript checks passed.

## Earlier native gate (superseded by completion evidence)

The ad-hoc signed helper aborts with signal 6 when launched inside the tool's
sandbox. The same helper code without App Sandbox succeeded on a synthetic DOCX;
that is not evidence for signed native containment. An outside-tool-sandbox
verification attempt was rejected by automatic approval review because its usage
limit had been reached. It was not retried through another launch route.

Before enabling import, resolve and verify:

1. Real signed App Sandbox launch, inherited-descriptor behavior, executable
   identity binding across verification/launch, and final desktop bundle assembly.
2. Real cancellation during parsing, deadline, stalled input/output, parent death,
   abnormal exit, cleanup failure, repeated jobs and runtime overhead measurements.
3. Wider PDF malformed/scanned/Unicode/order corpus qualification; current content
   checks normalize alphanumeric text and do not prove punctuation/layout order.
4. The final native editor/import/export/recovery/VoiceOver and standard-account
   matrix (Step 6). No new test-account action is requested until a qualified
   candidate is available.

The user authorized architecture changes to address nonviable native rlimits.
The proposed direct-child Wasm helper replaces the need for an XPC-launched
hostile native parser, but its native qualification is still open. Guest code
cannot launch descendants; the trusted interpreter/helper and OS sandbox remain
part of the trusted computing base. This does not claim arbitrary compromised
native runtime code is contained by guest fuel or memory limits.
