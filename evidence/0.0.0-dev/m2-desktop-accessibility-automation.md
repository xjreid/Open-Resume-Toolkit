# M2 desktop accessibility automation checkpoint

Date: 2026-09-03. Base commit: `91e7e54`; implementation changes are
uncommitted. Local verification platform: macOS arm64. Status: automated
semantic checks implemented; native assistive-technology and cross-platform
evidence remain pending.

## Implemented coverage

- The desktop test suite now runs axe-core against complete HTML documents in
  jsdom for the medium-routed portion of the initial main window and the full
  currently reachable overlay route.
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
- The harness includes a deliberately unnamed button as a positive control; the
  test must observe axe-core's `button-name` violation before the passing
  fixtures can be trusted.
- Color contrast is the only disabled axe rule because jsdom does not calculate
  layout or resolved colors. Native contrast, zoom, focus, and platform
  accessibility remain explicit manual gates.

The tests use only synthetic static and live jsdom markup. They do not start
Tauri, open a profile, access the OS vault, invoke a real native command, or
enable any remote content or document import path.

The main-route audit asserts that exactly one `.backup-panel` and one
`.storage-panel` exist, then removes them before axe is run. Backup/recovery and
all-local-data deletion are HIGH-tagged roadmap work; this medium-reasoning
checkpoint does not design tests for or extend evidence about those surfaces.

## Verification run

- `pnpm format:check`: passed.
- `pnpm --filter @ort/desktop lint`: passed.
- `pnpm --filter @ort/desktop test`: 14 files and 60 tests passed, including
  eight accessibility cases.
- `pnpm check:security`: passed; no forbidden remote assets, dynamic HTML/code
  APIs, secret files, or private-key markers were found.
- `pnpm audit --audit-level high`: no known vulnerabilities found, including
  the new test-only packages.
- `cargo fmt --all --check`, strict workspace Clippy, and
  `cargo test --workspace --all-targets --locked`: passed; the one explicit
  OS-vault integration test remained ignored as designed.

The attached GitHub Actions failure from the same date occurred only in
`pnpm audit --audit-level high --prod`: npm's advisory endpoint timed out after
the package manager's own retries. Every preceding contract, format, JavaScript,
Rust, and DOCX verification step in that job passed. A later local request to
the same advisory endpoint completed successfully with no known production
vulnerabilities, so no vulnerability remediation or source change was justified.

## Deliberate limitations

- Component automation cannot establish complete keyboard order, native modal
  trapping, announcement timing, 200% text zoom, forced colors, reduced motion,
  target geometry, or screen-reader output in WKWebView and WebView2.
- More complex edited content and non-conflict recovery states still need live
  component/native coverage.
- The HIGH-tagged backup/recovery and all-local-data deletion panels are
  deliberately excluded and retain their existing evidence status.
- Manual VoiceOver/macOS and NVDA/Windows checks, native dialog checks, and the
  final cross-platform accessibility matrix remain M2 release work.
- This checkpoint does not alter or approve the HIGH-tagged all-local-data
  deletion evidence gate, parser containment, vault behavior, or complete M2
  exit journey.
