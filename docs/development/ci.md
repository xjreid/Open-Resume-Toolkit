# CI and local verification

The executable definitions are [CI](../../.github/workflows/ci.yml),
[dependency scanning](../../.github/workflows/dependency-scan.yml),
[package scripts](../../package.json), and [justfile](../../justfile).
This map was reviewed on October 16, 2026. It describes checks, not a claim that
the hosted runs for an uncommitted working tree have passed.

## Gates

| Gate | Command or coverage | Where it runs |
| --- | --- | --- |
| Embedded source bytes | `node --test tools/tests/docx-checkout.test.mjs` | Before dependency installation on every runner; LF/catalog-signature positive and negative controls |
| JavaScript | `pnpm format:check`, `pnpm lint`, `pnpm test`, `pnpm build` | Ubuntu quality job; all workspace packages and extension packaging regressions |
| Documentation links | `pnpm check:docs` | Entry-point, community, product/implementation plan, component README, and `docs/` local destinations; excludes external URLs/heading validation |
| Local security | `pnpm check:security` | Current source: forbidden web APIs/remote assets and forbidden secret files/private-key markers |
| Licenses | `pnpm check:licenses`; target JS-only checks | Cargo/pnpm locked graph and explicit platform policies; inventory under `target/licenses/` |
| Generated contracts | `cargo run --locked -p ort-contract-generator` then `git diff --exit-code -- packages/contracts/generated` | Ubuntu; Rust-owned schemas, types, decoders, wire fixtures |
| Rust | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo test --workspace --all-targets --locked` | Ubuntu quality job |
| Desktop portability | Frontend build and `cargo check --locked -p ort-desktop` | macOS 15 arm64, macOS 15 Intel, Windows 2025 |
| Native storage | Separate `native_startup` test plus installer/domain/document/render/desktop/worker/application/platform/storage suites | Each desktop runner |
| Development browser bridge | `node tools/check-dev-browser-bridge.mjs` | Both macOS runners and macOS `just check`; explicit feature-enabled IPC/host and desktop disconnect tests |
| Independent documents | DOCX/PDF fixtures, independent parsers, field-preservation checks, all three selectable styles with schema v1/v2 | Ubuntu and desktop runners; field-preservation pair additionally in Ubuntu |
| macOS native probes | Legacy parser sandbox/hard-limit measurements, child lifecycle, bounded output with sanitizers, AppKit termination | Both macOS runners; isolated synthetic programs |
| Vulnerability scan | SHA-pinned OSV reusable workflow; `osv-scanner.toml`, `Cargo.lock`, `pnpm-lock.yaml` | Hosted PR/main/scheduled/manual workflow; separate from local source/license checks |

The October 16 dependency repair updates rustls to 0.23.46 and source-map-js to
1.2.2. The exact RUSTSEC-2024-0449 exception expires December 4, 2026: it is an
[informational unmaintained notice](https://rustsec.org/advisories/RUSTSEC-2024-0449.html)
with no patched release, inherited through Typst's serde_yaml dependencies.
The renderer uses fixed templates and denies external YAML/bibliography files;
regression tests exercise both paths. This exception does not suppress other
advisories against that package or either lockfile.

The October 19 CI repair adds exact RUSTSEC-2026-0370, RUSTSEC-2026-0371, and
RUSTSEC-2026-0372 exceptions for `zvariant`, `zbus_names`, and `zvariant_derive`.
They enter only through `keyring`'s Linux Secret Service backend; no Linux app is
distributed. The OSV policy test checks all-feature workspace dependency graphs
for Apple Silicon macOS, Intel macOS, Windows, and Linux: these packages must be
absent on both Mac architectures and Windows, and present in the Linux graph.
The exceptions expire December 4, 2026 and must be reviewed before distributing
the Linux backend. Both lockfiles and other advisories remain subject to the
full hosted scan.

The DOCX parser round-trip test checks every exported header line in reading
order, heading/list kinds, and section associations in all four styles. It also
checks body-only Skills content and excludes empty table anchors from extracted
content. Its former tab-joined header expectation predates the independent
header columns and no longer describes the exporter.

The default build's fail-closed native-host tests are preserved. Feature-enabled
macOS tests exercise the current-user development capability; they do not enable
or qualify the production bridge. Non-macOS builds cannot run that adapter.

The native XPC probes measure the older native-parser architecture. Their result
cannot enable the current metered-Wasm importer or certify whole-process parser
containment. The packaged importer has its own pinned helper/guest checks.

## Local commands

After installing the pinned prerequisites and dependencies, run:

```sh
COREPACK_ENABLE_NETWORK=0 pnpm_config_verify_deps_before_run=false \
  pnpm_config_update_notifier=false CARGO_NET_OFFLINE=true just check
```

CI also independently checks exported output:

```sh
cargo run --locked -p ort-documents --example docx_fixtures -- target/docx-ci-fixtures
python3 tools/verify-docx-fixtures.py target/docx-ci-fixtures
cargo run --locked -p ort-render --example pdf_fixtures -- target/pdf-ci-fixtures
node tools/verify-pdf-fixtures.mjs target/pdf-ci-fixtures
cargo run --locked -p ort-render --example field_preservation -- target/field-preservation-ci
node tools/verify-field-preservation.mjs target/field-preservation-ci
python3 tools/verify-field-preservation.py target/field-preservation-ci
python3 tools/check-style-output.py
```

Set `CARGO_NET_OFFLINE=true` for an offline run. Use fresh output directories
when diagnosing stale fixtures. The style script always makes a fresh directory.
A golden hash change requires inspecting rendered semantics and intended design;
do not regenerate goldens merely to hide a failing assertion.

## Current regression coverage

- The Codex installer enforces administrator access on Apple Silicon macOS and
  rejects installation before inspecting arguments on unsupported platforms. Its
  suite runs in Ubuntu quality and every desktop matrix job, including Windows
  and Intel macOS; Unix filesystem safety tests remain active on Unix runners.

- Profile ownership excludes another process and is released after forced process
  death, even if the profile directory is removed.
- Restore/safety/delete-all cleanup retains credentials for rollback and retries
  their removal before destroying database keys; malformed rosters fail closed.
- Drag roots use private permissions, preserve live leases, and clean abandoned
  or narrowly validated legacy directories.
- Tracker search runs in bounded background work. Search changes coalesce and
  never fetch another page automatically; projection migration/update/delete
  and profile boundaries are checked.
- Backup export rejects oversized histories before materializing rows, stops
  bounded serialization, and round-trips a valid profile above 95% of the payload
  limit. Historical format vectors remain readable.
- Current AI fixtures cover explicit model selection, key states, durable cap
  accounting after history clearing, structured tailoring/refinement, errors,
  cancellation, and Codex memory-only sessions and shutdown.

## Interpretation and limits

Local checks use synthetic content and temporary/in-memory vaults. Real Keychain,
installed/signed app, live provider/account, runtime download, and destructive
failure campaigns remain explicit opt-in tests. Ordinary runs do not mark these
ignored tests as passed.

A sandbox may deny local sockets, AppleScript, XPC, or AppKit. Rerun an authorized
synthetic check in a suitable native session or report it unverified. Preserve
its assertions; do not add skips, retries, or `continue-on-error` for a product
failure.

A local arm64 run does not prove hosted Linux/Intel/Windows execution, OSV
results, branch protection, production signing, or Store qualification. YAML
parsing is not full GitHub Actions validation; use `actionlint` when installed.
The current secret scanner is intentionally narrow and does not prove that
outgoing Git history is free of tokens or personal data.

## Local review result — October 15, 2026

Reviewed the uncommitted candidate based on `63f5c1a` on `main`, including the
profile/cleanup/tracker/backup audit fixes and this CI/documentation update.
The cached `origin/main` reference matched HEAD; no remote state was fetched.

- Workspace Rust tests: 539 passed, 19 explicit opt-in tests ignored.
- macOS development bridge: 15 tests passed across IPC, host, and desktop.
- JavaScript: desktop, contracts, extension, and tooling suites passed; source
  formatting, types, builds, local security and locked license inventory passed.
- Contracts regenerated without drift; all-feature workspace Clippy and Rust
  formatting passed.
- Independent DOCX/PDF/field-preservation checks and all 48 reviewed v1/v2 style
  output pairs passed.
- All four synthetic macOS sandbox/lifecycle/output/AppKit probes completed with
  their documented limited claims. Native tests requiring local sockets,
  AppleScript and XPC ran outside the restricted tool sandbox.
- Repository documentation links passed. `actionlint` is unavailable locally;
  workflow YAML is parsed and commands inspected, without claiming full Actions
  semantic validation.

Hosted Linux/Intel/Windows execution, OSV advisory results, production
signing/Store qualification, and ignored real-vault/live-account tests remain
unverified by this local run. Results apply to the candidate, not a pushed commit.
