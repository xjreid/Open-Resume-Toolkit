# Development

The active target is macOS on Apple Silicon. The current app includes the offline
resume workflows, direct AI, application overlay/tracker, Chrome development
capture, and optional Codex. See the [implementation map](docs/development/current-state.md)
for availability and the [roadmap](<Implementation Plans/System Documentation/Delivery_Roadmap.md>)
for acceptance and remaining release work. Windows/Intel CI is portability
coverage, not a support claim.

## Prerequisites

Use the checked-in pins:

- Node.js 24.16.0 (`.nvmrc`).
- pnpm 11.19.0 through Corepack (`package.json`).
- Rust 1.98.0 with Clippy/rustfmt (`rust-toolchain.toml`). Packaging parser guests
  also needs the configured LLVM tools and guest toolchain prerequisites.
- `just` 1.x, Python 3, and Xcode Command Line Tools on macOS.

Install prerequisites deliberately, then from the repository root:

```sh
just bootstrap
just check
just dev
```

Bootstrap installs locked JavaScript dependencies. `just dev` checks the isolated
`com.openresumetoolkit.dev` identity before launching. Use synthetic resumes,
jobs, provider fixtures, and disposable profiles for development and tests.
Never reset a real profile to make a check pass.

Bootstrap also installs the checksum-pinned secret scanner and enables the local
pre-commit/pre-push hooks. For an existing checkout run `pnpm security:setup`.
See [secret protection](docs/development/secret-protection.md) for source,
history, client-build, packaging, and GitHub enforcement details. Missing scanner
tools fail the security/build gates; setup requires network, subsequent scans do not.

## Current development builds

`just dev` runs the frontend and Tauri shell. Import requires the signed parser
helper and its executable/running-code pins. Codex installation requires its
packaged helper and digest. A bare shell build does not establish that either
helper is available.

For a complete local development app, including verified helpers and the
explicit development browser feature:

```sh
python3 tools/build-development-app.py
```

This builds and verifies an ad-hoc signed candidate and records its location in
`target/latest-codex-installer-update.txt`. It does not install/launch the app,
install the external Codex runtime, connect an account, or perform production
signing. Follow [Chrome development setup](packaging/extension/chrome/development-testing.md)
for native-host registration and matching extension identities. Edge capture
is not implemented. The [extension README](apps/extension/README.md) documents
builds and packaging.

Codex is optional and separately installed. Its page checks installation
without starting the runtime. Use **Start server**, then managed ChatGPT sign-in;
**Stop server**, quitting, and profile retirement end memory-only authorization.
Relaunch starts stopped. See the [runtime and installer guide](docs/ai/codex-plan-runtime.md).
Live provider calls and runtime installation are explicit user actions, not
ordinary CI tests.

`just package-preview` builds a local app/DMG integrity-test artifact. It is
not a substitute for the helper-complete development build or M7 production
signing/notarization. See [macOS packaging](packaging/macos/dmg/README.md).

## Local checks and CI

After dependencies/toolchains have been installed, the local gate can run with
network and automatic dependency checks disabled:

```sh
COREPACK_ENABLE_NETWORK=0 pnpm_config_verify_deps_before_run=false \
  pnpm_config_update_notifier=false CARGO_NET_OFFLINE=true just check
```

`just check` runs JavaScript formatting/types/tests/builds, security/license
checks, local documentation links, generated-contract drift, Rust formatting/Clippy/workspace tests, and
macOS development-bridge tests. Offline license checks need cached metadata for
all locked targets. Missing caches are an incomplete check, not a reason to
weaken the license policy.

The [CI guide](docs/development/ci.md) lists additional independent DOCX/PDF/field/
style audits, native macOS probes, and the hosted dependency scan. Hosted
Linux/Intel/Windows execution and real signing/OS-vault qualification cannot be
inferred from a local arm64 pass.

Useful focused commands:

```sh
pnpm --filter @ort/desktop test
pnpm --filter @ort/extension test
cargo test --locked -p ort-storage -p ort-backup -p ort-vault
just test-dev-browser-bridge
python3 tools/check-style-output.py
```

Default native-host tests verify that the production transport fails closed.
The separate macOS feature test exercises authenticated development sockets,
replay/timeout rejection, cleanup, and responsive desktop disconnect. Preserve
both sets when changing the bridge.

Tests normally use synthetic data and in-memory vaults. Ignored tests identify
explicit network, account, signed-artifact, Keychain, containment, or failure-
injection qualification. They remain separate from ordinary test runs; their
absence is not a pass. Sandbox restrictions can block loopback sockets,
AppleScript, XPC, or AppKit, so record those limitations separately from product
failures.

## Contracts, state, and documentation

Rust domain/wire types own serialized contracts. Run `just generate` after
changing them, inspect generated changes, and run `just verify-contracts`.
Do not hand-edit generated schemas or TypeScript bindings.

Before changing a user-visible workflow, read and update its
[application behavior reference](docs/application-behavior/README.md), owning
product plan, and implementation plan. The [plan index](Plan_Index.md) defines
precedence. Keep implemented behavior distinct from planned release requirements.

Database migrations are forward-only. Preserve profile ownership across storage
retries and replacements, and extend backup/deletion inventories when persistent
records or credentials are added. Exported files stay outside profile deletion.
See [backup format and lifecycle](docs/backup/backup-v1.md).

## Historical qualification

M0–M2 setup experiments, test counts, and checkpoint walkthroughs are retained in
[the development history](docs/development/m0-m2-development-history.md) and
[acceptance evidence](evidence/0.0.0-dev/m2-acceptance-closure.md). They describe
their original builds. They do not mean AI, browser capture, import, or the
current UI is still at that earlier checkpoint.
