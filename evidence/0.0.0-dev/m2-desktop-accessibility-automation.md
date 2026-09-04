# M2 desktop accessibility automation checkpoint

Date: 2026-09-04. Base commit: `28401b0`; implementation changes are
uncommitted. Local verification platform: macOS arm64. Status: automated
semantic checks implemented; native assistive-technology and cross-platform
evidence remain pending.

## Implemented coverage

- The desktop test suite now runs axe-core against complete HTML documents in
  jsdom for the full initial main window and currently reachable overlay route.
  Backup/recovery, storage, and all-local-data deletion are no longer removed
  before the audit.
- A composed control-state fixture covers the PDF preview and quit-decision
  surfaces. This catches missing programmatic names, broken ARIA references,
  invalid landmark structure, and other rules axe-core can determine without
  native layout.
- A populated synthetic published resume verifies the document review heading
  structure and accessible article name.
- A live React/jsdom harness drives the native-command boundary with synthetic
  responses and audits the loaded editor, an invalid required title, and a
  revision-conflict recovery state. It verifies that inline validation is
  programmatically associated, the conflict is announced, and the editor
  remains available for correction.
- The quit dialog records the invoking control, initially focuses **Keep
  editing**, and restores focus to the connected invoking control after
  cancellation. A live component test covers that sequence.
- Exact rollback, safety-copy deletion, backup replacement, and all-local-data
  deletion confirmations expose programmatic invalid state and associated
  correction text after a nonempty mismatch. Each asynchronous form exposes an
  `aria-busy` state while its single native operation is pending.
- Live recovery tests drive synthetic content-free status and safety-copy
  deletion responses. A live all-local-data deletion test covers invalid and
  exact confirmation, announced progress, committed-but-cleanup-pending state,
  callback semantics, and focus placement on the final status/alert.
- The harness includes a deliberately unnamed button as a positive control; the
  test must observe axe-core's `button-name` violation before the passing
  fixtures can be trusted.
- Color contrast is the only disabled axe rule because jsdom does not calculate
  layout or resolved colors. Native contrast, zoom, focus, and platform
  accessibility remain explicit manual gates.

The tests use only synthetic static and live jsdom markup. They do not start
Tauri, open a profile, access the OS vault, invoke a real native command, delete
data, or enable any remote content or document import path.

## Verification run

- `pnpm format:check`: passed.
- `pnpm --filter @ort/desktop lint`: passed.
- `pnpm --filter @ort/desktop test`: 14 files and 62 tests passed, including ten
  accessibility cases.
- `pnpm check:security`: passed; no forbidden remote assets, dynamic HTML/code
  APIs, secret files, or private-key markers were found.
- `cargo fmt --all --check`, strict workspace Clippy, and
  `cargo test --workspace --all-targets --locked`: passed; the one explicit
  OS-vault integration test remained ignored as designed.

The attached GitHub Actions failure occurred only in the npm advisory request;
every preceding contract, format, JavaScript, Rust, security-source, and DOCX
check passed. A direct diagnostic request from the local verification host also
timed out without response. CI now delegates a full `pnpm-lock.yaml` and
`Cargo.lock` scan to the SHA-pinned OSV reusable workflow rather than making the
quality job depend on npm's unavailable advisory POST. See
`m2-ci-dependency-scan.md`; its first GitHub result is pending.

## Deliberate limitations

- Component automation cannot establish complete keyboard order, native modal
  trapping, announcement timing, 200% text zoom, forced colors, reduced motion,
  target geometry, or screen-reader output in WKWebView and WebView2.
- More complex edited content and non-conflict recovery states still need live
  component/native coverage.
- The HIGH-tagged recovery and deletion surfaces now have synthetic semantic and
  state-transition coverage, not native execution evidence. Vault interruption,
  filesystem failure, and actual data-deletion behavior remain governed by their
  separate HIGH evidence.
- Manual VoiceOver/macOS and NVDA/Windows checks, native dialog/interruption
  checks, and the final cross-platform accessibility matrix remain M2 release
  work.
- This checkpoint does not alter or approve the HIGH-tagged all-local-data
  deletion evidence gate, parser containment, vault behavior, or complete M2
  exit journey.
