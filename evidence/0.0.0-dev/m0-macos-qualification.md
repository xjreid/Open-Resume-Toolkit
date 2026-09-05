# Step 3: M0 macOS Apple Silicon qualification

Date: 2026-09-05. Source baseline: `0a455e961870b35396d35003e5e4faca6209c574`.
The containing repository commit is authoritative; these changes were tested
before committing. Scope is the development channel on macOS arm64 only.

## Implementation and CI repair

- Replaced the placeholder `just test-platform` with a native-arm64 preflight,
  exact development configuration checks, and real synthetic isolation tests.
- The desktop refuses a non-development bundle identifier before resolving
  profile paths or touching the vault. This composition root does not yet
  implement stable or preview channels.
- A storage regression test checks every mismatched pair of dev, preview,
  stable, and test channels: refusal preserves the synthetic database and
  manifest bytes, preserves the key, and leaves the original profile reopenable.
  The vault in this test is in memory; this is not native Keychain ACL evidence.
- Added repeatable clean-clone bootstrap/check and local-certificate
  release-build/installed-app verification commands. Reports never mark native
  GUI success, notarization, or milestone completion automatically.
- Both user-attached CI logs from 2026-09-04 fail for the same reason:
  `fsevents@2.3.3` is absent on Linux/Windows and had no policy for a single-OS
  optional dependency without an installed sibling. The new exact MIT metadata
  record pins its lockfile integrity and macOS restriction. On macOS, installed
  metadata must match; absence is allowed only on Linux/Windows. There are no
  new license exceptions or allowlisted families.
- Negative tests cover changed integrity/version/OS/license, absent macOS
  installation, duplicate records, unsupported OS, broadened capabilities,
  remote entrypoints/assets, weakened CSP, wrong signer, ad-hoc signing,
  missing hardened runtime, and added entitlements. A regression test also
  ensures the signing requirement is compared independently of executable
  location and codesign output-stream order.

## Installed artifact and native observations

- Platform: macOS 26.6.2, build 25G83, native arm64; Node 24.16.0, pnpm 11.19.0,
  Rust 1.98.0. Interactive developer account; no test-account switch required.
- Implementation SHA-256:
  `faabea8e6127f369e5dbd79a6d9e6b2b985088e2fa06577d26ad94e66432aada`.
  The harness hashes tracked and unignored new implementation files, excluding
  documentation/evidence and ignored build outputs. It records the baseline
  commit and dirty status separately.
- Built and installed `/Applications/Open Resume Toolkit Dev.app` from the
  release-profile Tauri build. Bundle identifier: `com.openresumetoolkit.dev`.
- Installed executable SHA-256:
  `9ab48ab4e234fedfc7708472be10a88907c1ae95a9337cde6cbbedc91e0bd51e`.
- Local signer: `ORT Local Test Signing`. Public certificate SHA-256:
  `d4fdc787da71f95d9f5455c1a733e9d942d76409a72b92a27dff48da66c2cc49`.
- Designated requirement:
  `identifier "com.openresumetoolkit.dev" and certificate root = H"314932fe09cb08143ddc66fed25ba732fd20cfe5"`.
- Both built and installed copies passed `codesign --verify --deep --strict`;
  arm64-only architecture, hardened runtime, and empty entitlements verified.
  The installed executable, signer, and requirement match the build report.
- The exact reviewed source policy requires local main/overlay routes, the
  production CSP, only the named main/overlay `core:default` capabilities,
  no remote capability targets, no additional sidecars/resources, and no
  dangerous CSP overrides. Both production HTML entrypoints reference local
  assets that exist; all eight generated frontend assets were inventoried.
  This evidence links the reviewed inputs to a freshly built sealed app; it is
  not an independent extraction of every embedded setting from an arbitrary
  executable or a network-containment test.
- After installing the final verified build, native accessibility inspection
  observed **Encrypted storage ready** in `Open Resume Toolkit Overlay Dev`
  at `tauri://localhost/overlay.html`, then in `Open Resume Toolkit Dev` at
  `tauri://localhost` after selecting it from the Window menu. Both routes use
  the validated typed health-command response. The main window showed the
  existing synthetic draft as **Saved**. No editor, backup, restore, deletion,
  or profile-reset actions were performed.
- The previous installation is retained locally at
  `target/m0-qualification/previous-installed.app`. No key was exported, no
  Keychain trust or account configuration was changed, and no profile data was
  deleted. The installed app remains a local development build, not a
  Developer-ID-signed/notarized distribution. The earlier DMG was not refreshed.

Machine-generated local reports: `target/m0-qualification/preflight.json`,
`build.json`, `verified-app.json`, and `clean-checkout.json`. These ignored
reports include paths and public digests, not private keys or resume contents.
The build/installed reports intentionally leave GUI checks pending; the manual
observations above supplement them without overwriting their machine results.

## Test results and remaining gates

The full repository gate passed locally during implementation and again on the
final implementation in a fresh clone. Final run completed at
`2026-09-05T16:00:03.901Z` with temporary snapshot commit
`68d561616ab354222baca4c3c500de7e3cc447a9`. That commit exists only in the
disposable clone, not in the user's project history. Its implementation digest
matches the built/installed artifact's source digest above. The clone remains
at `/var/folders/5j/9n49kjzs3y389ph5w432lkd40000gn/T/ort-m0-checkout-2OUWYm/source`.
Documentation/evidence was finalized afterward and format/diff checked.

Passed:

- Fresh `just bootstrap`: frozen lockfile installation, no copied project
  `node_modules` or Cargo `target`; shared system/download caches only.
- `just check`: formatting, TypeScript, repository/extension/contracts/desktop
  tests, frontend builds, static security checks, dependency-license inventory
  (727 Rust and 167 JavaScript/workspace packages), clean contract regeneration,
  Rust formatting, strict workspace Clippy, and the workspace Rust test suite.
- `just test-platform`: final source configuration preflight and both targeted
  Rust isolation tests. The clone remained clean and the original implementation
  digest was unchanged throughout the successful final run.
- Final local-certificate packaging, installed-app verification, and the two
  installed-window observations recorded above.

The opt-in native OS-vault and PDFium-library tests remain intentionally ignored
by the ordinary workspace gate; they are not counted as passed. Earlier verifier
iterations exposed certificate-extraction syntax and signing-output path
comparison errors; both were corrected before the successful final run. An
intermediate clone run correctly refused current-source evidence when the
verifier source changed during testing. Only the final matching-digest report
is used here.

M0 formal signoff still requires green hosted CI on the containing commit.
The attached failures are diagnosed and repaired locally, not yet confirmed
fixed by a new hosted run. Windows/Intel CI remains a portability signal;
Windows/Intel native qualification is deferred.

The installed health check does not close M1's cross-account/cross-process
Keychain, moved/updated identity, recovery, migration, low-disk, or hostile
backup matrices. It does not close M2's remaining implementation, native
interaction, accessibility, document-reader, containment, or performance gates.
See `DEVELOPMENT.md` for rerun commands and account preparation. Production
signing/notarization/distribution evidence remains a separate future gate.
