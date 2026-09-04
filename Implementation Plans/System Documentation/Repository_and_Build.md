# Repository, contracts, and build plan

## Status

- Status: approved baseline
- Owner: core maintainers
- Milestone: M0
- Non-goal: visual component design or aesthetic assets

## Proposed source tree

The existing planning folders remain at the repository root. Implementation adds:

```text
apps/
  desktop/                    React/TypeScript UI and Tauri configuration
    src/features/             feature-owned views, view models, and tests
    src/shared/               typed command client and accessible primitives
    src/platform/             narrow UI-only platform presentation adapters
    src-tauri/                shared entry point, commands, capabilities, packaging
      platform/windows/       Windows composition and packaging hooks
      platform/macos/         macOS composition and packaging hooks
  extension/                  shared Manifest V3 Chrome/Edge source
    src/                      shared capture, messaging, status, and validation
    manifest/                 Chrome/Edge and channel-specific manifest templates
crates/
  ort-domain/                 entities, value objects, policies, state machines
  ort-application/            use cases, coordinator, ports, error model
  ort-backup/                 portable authenticated backup format and validation
  ort-storage/                SQLCipher repositories and migrations
  ort-vault/                  OS credential-vault adapter
  ort-ai/                     provider and Codex adapters, accounting, guardrails
  ort-documents/              import, normalization, Typst/DOCX/plain-text output
  ort-document-worker/        disposable sandboxed PDF/DOCX extraction executable
  ort-ipc/                    authenticated local protocol and framing
  ort-native-host/            browser native-messaging executable
  ort-platform/               paths, permissions, process, update, single-instance
packages/
  contracts/                  source schemas and generated TypeScript bindings
  catalog/                    provider/model/pricing catalog schema and verifier
templates/
  resume/                     versioned Typst templates; aesthetics added later
  fonts/                      approved redistributable font files and licenses
fixtures/
  documents/                  synthetic PDF/DOCX/text corpus
  ai/                         synthetic provider and prompt-injection fixtures
  ipc/                        valid and hostile protocol corpus
tools/                        generation, verification, and release helper commands
packaging/                    Windows/macOS/Chrome/Edge package configuration
.github/workflows/            CI, preview packaging, and protected release workflows
```

Early milestones keep a deliberately plain functional fixture in `templates/resume/` to prove layout and accessibility before the approved document styles are finalized through development testing. The fixture is not a shipped product theme.

## Workspace tooling

- Rust stable, pinned by `rust-toolchain.toml`.
- Cargo workspace with one lockfile committed.
- Node LTS, Corepack, and pnpm with an exact package-manager version.
- TypeScript strict mode, ESLint, Prettier, and Vitest.
- Tauri 2 with the smallest required official plugins.
- `justfile` or `cargo xtask` exposes platform-neutral developer commands; scripts contain no embedded credentials.

The bootstrap pull request records the exact versions selected after compatibility and license verification. Dependabot or Renovate may propose updates, but security-sensitive libraries and Tauri major/minor updates require a human-reviewed compatibility run.

## Contract ownership

Rust domain types are canonical for application records. The build generates:

- JSON Schema for persisted document records and AI structured outputs;
- TypeScript types and runtime validators for UI commands/events;
- JSON Schema for native-messaging and catalog payloads;
- a machine-readable compatibility manifest used by desktop, host, extension, and updater tests.

Generated output is checked in so extension and website builds do not require Rust. CI regenerates into a temporary tree and fails on a diff. Hand editing generated files is forbidden.

Cross-language integer counters use bounded integers whose JSON range is safe in JavaScript; currency uses `{currency, micros}` or provider-native decimal strings, never floating point. Timestamps are RFC 3339 UTC plus an IANA time-zone identifier when calendar boundaries matter.

## Dependency policy

The M0 license gate is implemented through
`../../config/dependency-license-policy.json` and
`../../tools/check-licenses.mjs`. It checks the complete locked Cargo metadata
graph plus every pnpm lock package, rejects missing/unknown licenses, and writes
the deterministic inventory under `target/licenses/`. Platform-specific
JavaScript packages are rechecked on each supported CI target. The policy has no
package exceptions as of 2026-09-04. Binary-asset notices and the release SBOM
remain separate packaging gates.

Before adding a runtime dependency, record:

1. purpose and why standard-library/current dependencies are insufficient;
2. upstream repository and release cadence;
3. SPDX license and required attribution;
4. transitive native binaries or network behavior;
5. maintenance and security history;
6. whether it handles secrets, parsing, cryptography, updates, or rendering;
7. removal/replacement boundary.

Forbidden without an explicit architecture decision: remote UI/CDN assets, telemetry SDKs, advertising SDKs, provider SDKs that bypass the common adapter contract, arbitrary shell plugins, and dependencies that upload crash reports by default.

Binary inputs such as PDFium and fonts are pinned by digest, mirrored only when licensing permits, listed in the SBOM, and verified before build use.

## Local developer workflow

Initial commands will provide these stable intents:

```text
just bootstrap       validate toolchains and install locked JS dependencies
just generate        regenerate schemas and bindings
just check           format, lint, unit test, license and contract drift checks
just dev             run desktop with an isolated synthetic-data profile
just test-integration run storage, renderer, IPC, and mocked-provider tests
just package-preview build local preview packages (explicitly ad-hoc signed on macOS)
```

Development uses an app-data suffix such as `OpenResumeToolkit-Dev`; it must never open a stable user database. Test profiles live in per-test temporary directories. Networked tests are opt-in and never use personal provider credentials in CI.

## Build profiles

| Profile | Purpose | Behavior |
|---|---|---|
| test | unit/integration | test-only adapters, temporary vault/database, no real update/signing |
| development | local UI work | dev identity, verbose local logs, extension dev IDs |
| preview | public pre-release | production optimizations and preview identity/channel; local macOS artifacts are ad-hoc signed, while distributed previews require the milestone's trusted-signing policy |
| stable-direct | GitHub distribution | production identity, direct updater, signed Windows requirement |
| stable-store | fallback Windows Store | Store identity and update ownership; direct updater disabled |

Feature flags cannot weaken encryption, validation, or permission checks. Codex, browser bridge, and updater can be compiled or runtime-disabled when their platform gate has not passed.
Document import is runtime-disabled on a platform/package whose parser-worker sandbox or termination proof has not passed; it never falls back to in-process parsing.

## CI layers

### Pull request

- formatting and static analysis;
- Rust and TypeScript unit tests;
- generated-contract drift;
- dependency license allowlist and vulnerability scan;
- secret scan;
- synthetic import/render golden tests;
- extension lint/package validation;
- accessibility automated checks for reachable UI routes;
- builds on current supported Windows x64 and macOS Intel/Apple Silicon targets.

### Nightly or scheduled

- fuzz corpora for backup, IPC, PDF, DOCX, URL, and AI-output parsers;
- current provider contract probes using project-owned low-limit test credentials;
- Codex supported-version matrix;
- update/install/repair virtual-machine matrix;
- deterministic/reproducibility comparison where toolchains permit.

### Protected release

- build from a signed tag or immutable commit;
- produce packages, updater artifacts, checksums, SBOM, licenses, provenance, and compatibility manifest;
- perform signing in a protected environment;
- verify signatures and install packages on clean machines;
- publish a draft GitHub Release only after gates pass;
- promote the already-tested artifacts rather than rebuilding them.

## Test boundaries and fixtures

All committed resumes, job descriptions, provider responses, and imported documents are synthetic and clearly labeled. Golden-output updates require reviewer approval and a semantic report; a changed PDF hash alone is not sufficient evidence of correctness.

Provider adapters use recorded contract fixtures with secrets and identifying text removed. Live probes test only small synthetic requests and are separated from deterministic CI.

## Evidence layout

CI attaches a machine-readable evidence bundle:

```text
evidence/<version>/
  tests/
  accessibility/
  security/
  licenses/
  sbom/
  provenance/
  install-matrix/
  renderer/
  compatibility.json
```

Release notes link to durable public evidence where safe. Sensitive security-review details remain private, while the public repository records the control tested and pass/fail result.

## Completion criteria

- A clean checkout can bootstrap and run all offline tests using documented commands.
- Windows and macOS builds use the same generated contracts and domain test suite.
- CI rejects schema drift, forbidden licenses, unpinned package-manager state, and accidental remote UI assets.
- Development data cannot be confused with stable-profile data.
- Every shipped dependency and binary asset appears in the license inventory and SBOM.
