# M2 implementation completion — Step 5

Date: 2026-09-07. Scope: macOS Apple Silicon. **Step 5 implementation is
complete. Step 6 final native/user acceptance and full M2 milestone signoff
remain pending.** M0 and M1 remain complete and successful.

## Delivered implementation

The complete local path includes manual starting profiles, structured focused
editing with dates/links and validation navigation, exact PDF preview, three
selectable PDF/DOCX styles, text export, publication and retained-source replay,
portable backup/recovery/deletion integration, and guarded quit. Earlier evidence
records those components; this completion closes the missing import integration.

PDF/DOCX import now runs through the native picker, a bounded source snapshot,
the signed disposable metered-Wasm helper, native proposal/review ownership,
explicit per-block decisions, and a transactional draft save with a content-free
audit summary. Empty-profile import creates no placeholder draft before explicit
acceptance. Cancellation, stale revisions, invalid source/output and failed
cleanup cannot apply partial changes. Review source never becomes file/process
or storage authority in the renderer.

The final package enables the new import path only on macOS arm64 with compiled
SHA-256 and CodeDirectory-hash pins for its helper. Ordinary unbundled builds
remain unavailable. The old `ort_documents::IMPORT_ENABLED` flag still disables
only the superseded native-parser path; it is no longer the new desktop path's
gate. No native rlimit or XPC-probe result was relabeled as Wasm containment proof.

## Native containment and packaging evidence

The signed helper carries only App Sandbox entitlement and embedded guest bytes.
Guest code receives no general filesystem, network, credential, IPC, environment,
clock or process API. One bounded memory/instance/table, recursion/value-stack
limits and fixed fuel apply; callbacks charge fuel before bounded work. Source
and extraction cross private pipes without plaintext staging files. Native
runtime/adapter code remains trusted; 512 MiB is the guest memory limit, not a
whole-native-process resident-memory claim.

Before any source bytes are sent, the parent verifies the packaged executable's
SHA-256/signature, then asks macOS for the running child's code identity and checks
its exact pinned CodeDirectory hash plus sandbox entitlement presence. The child
remains unreaped during that check, preventing PID reuse. The exact pinned
signature binds the entitlement value and executable. Wrong running-code hash
is refused. See Apple's [requirement language](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/RequirementLang/RequirementLang.html)
for the underlying requirement semantics.

The exact helper nested in the completed candidate passed:

- PDF and DOCX extraction through the parent supervisor, with both EOFs and
  OS-observed reaping before any extraction is accepted;
- wrong running-code-hash denial and cancellation during PDF parsing;
- repeated jobs and malformed, truncated, trailing and oversized input;
- parent-death output closure;
- real 60-second incomplete-input and blocked-output deadlines. The incomplete
  input run exited after 60.045 seconds. Peak child RSS in this synthetic matrix
  was 39,288,832 bytes; this is a measured workload result, not a global RSS bound.

Raw reports: `target/m2-bundled-helper-smoke.log`,
`target/m2-parser-helper-native.json`, `target/m2-parser-helper-native.log`.
The earlier approval-service usage-limit block was resolved by the normal approval
route; no alternate execution path bypassed it.

`tools/package-m2-macos.py` embeds both helper identities in the desktop build,
copies the helper below `Contents/Helpers`, signs only the outer bundle with the
existing ORT Local Test Signing identity, and verifies nested signatures without
re-signing the pinned helper. `tools/stage-m2-acceptance.py` creates a fresh Shared
folder and account-guarded launcher. Neither installs or launches the desktop app.

## Final checks

- Full Rust workspace/all-target tests: 246 passed, zero failed, nine explicitly
  opt-in native/mutation tests skipped.
- Strict workspace Clippy with all targets/features: passed.
- Full `pnpm check`: passed, including 113 desktop tests, 26 contract tests,
  repository tool tests, type checks, builds, formatting, web/secret checks and
  dependency license checks (730 Rust / 167 JavaScript packages).
- 96 PDF/DOCX fixtures spanning schema v1/v2 and all three styles: extraction,
  normalized content inclusion and heading/bullet order passed. Normalization
  deliberately excludes punctuation/whitespace; native-reader visual checks are
  separate Step 6 observations.
- Real PDFium guest rejects image-only/image-dominant pages and malformed PDF,
  while accepting readable text with an ordinary image/logo.
- Runtime callback tests, encrypted audit rollback, empty-profile explicit commit,
  job-slot teardown and frontend cancellation/race tests passed.

No hosted CI run or final human accessibility/native-reader acceptance is claimed.
The installed M1 app and existing account data were not replaced. Changes remain
uncommitted for the user's normal review/CI workflow.

## Candidate and remaining acceptance

The signed candidate and exact hashes are recorded in
`target/m2-candidate/manifest.json`; the new Shared transfer location is recorded
in `target/m2-candidate/transfer.json`. Use the [Step 6 checklist](m2-final-native-acceptance.md)
for the real `orttest` login. Final native editor/import/output/recovery,
VoiceOver/keyboard, Keychain and filesystem fault observations remain acceptance
work. Marking implementation complete does not mark those observations passed.

### Final transfer verification note

Both the candidate and Shared transfer desktop/helper SHA-256 values match their
recorded identities. Nested signature verification passed during packaging and
Shared staging. A redundant later signature check failed inside the tool sandbox
because local certificate trust was unavailable; its outside-sandbox retry was
rejected when the approval service again reached its usage limit. No binary bytes
changed after the successful signing/staging verification. That redundant repeat
is not represented as passed, and no alternate route bypassed its rejection.

Shared acceptance folder: `/Users/Shared/ORT-M2-acceptance-jov1muwi`.
