# Development and deployment outline

## Status and purpose

- Status: approved implementation handoff baseline
- Owner: core maintainers, platform maintainers, and release maintainer
- Starts at: M0
- Product authority: `../../Plan_Index.md` and all files under `../../Product Plans/`

This document turns the architecture, repository, roadmap, security, and distribution plans into one build-and-release outline. It does not create four independent applications. ORT uses one shared desktop codebase and one shared browser-extension codebase, producing platform- and Store-specific artifacts through narrow adapters and packaging configuration.

For the current M0-M2 phase, the only qualified native target is macOS Apple
Silicon. Windows and Intel-Mac outputs below are retained future artifact designs;
their CI builds are portability signals and their native/package matrices are
not current milestone blockers.

## Source and artifact model

```text
Shared desktop source (React/TypeScript + Tauri/Rust)
  +-- macOS Apple Silicon desktop/native-host/document-worker build [active]
  +-- Windows desktop/native-host/document-worker build [later]
  `-- macOS Intel/universal desktop/native-host/document-worker build [later]

Shared Manifest V3 extension source
  +-- Chrome package with Chrome ID/manifest/Store metadata
  `-- Edge package with Edge ID/manifest/Store metadata
```

Windows and macOS packages are builds of the same product version and canonical domain/contracts, not separately designed products. Chrome and Edge packages likewise share capture, validation, messaging, and UI code. Platform- or browser-specific code must remain behind explicit interfaces and may not fork product behavior.

## Implementation repository shape

```text
apps/
  desktop/
    src/features/                 shared React feature views and tests
    src/shared/                   generated client, accessible primitives, utilities
    src/platform/                 UI-only platform presentation adapters when required
    src-tauri/
      src/                        shared Tauri commands and composition root
      capabilities/               least-privilege capability files per window
      platform/windows/           Windows composition and packaging hooks
      platform/macos/             macOS composition and packaging hooks
      tauri.conf.json             shared safe defaults
  extension/
    src/                          shared Manifest V3 extension implementation
    manifest/chrome.json          Chrome/channel identifiers and permissions
    manifest/edge.json            Edge/channel identifiers and permissions
    assets/shared/                common icons/assets
    assets/chrome/                Chrome-only Store/package assets if required
    assets/edge/                  Edge-only Store/package assets if required
crates/
  ort-domain/                     records, validation, policies, state machines
  ort-application/                use cases, ports, operations, recovery
  ort-backup/                     portable authenticated backup format and validation
  ort-storage/                    SQLCipher repositories and migrations
  ort-vault/                      shared interface plus Windows/macOS adapters
  ort-documents/                  mapping, rendering, DOCX/text output
  ort-document-worker/            disposable sandboxed PDF/DOCX parser
  ort-ai/                         direct providers, Codex, accounting, guardrails
  ort-ipc/                        authenticated local protocol and framing
  ort-native-host/                shared native-messaging host with OS transport adapters
  ort-platform/                   OS paths, permissions, processes, dialogs, updater
packages/
  contracts/                      checked-in generated schemas/TypeScript bindings
  catalog/                        signed model/pricing catalog schema and verifier
packaging/
  windows/nsis/                   per-user direct installer configuration
  windows/store/                  fallback Store packaging spike/configuration
  macos/dmg/                      DMG layout and preview guidance
  extension/chrome/               Chrome package/listing metadata
  extension/edge/                 Edge package/listing metadata
```

Platform folders contain operating-system integration, entitlements, manifests, installer hooks, and tests only. Business rules, persisted schemas, provider behavior, document rules, and user-visible workflows remain shared.

## Build-target matrix

| Target | Initial architecture | Output | Includes | Distribution state |
|---|---|---|---|---|
| macOS desktop | Apple Silicon (`arm64`) | app bundle/DMG; later updater archive | desktop, native host, document worker, renderer/assets | active development and unsigned preview; stable requires Developer ID and notarization |
| Windows desktop direct | architecture to revalidate later | per-user NSIS installer and updater payload | desktop, native host, document worker, renderer/assets | deferred; preview until trusted signature, stable after signing |
| Windows Store fallback | architecture to revalidate later | approved Store/MSIX or documented packaged-Win32 form | same components, Store-owned updater | deferred; stable only after native-host/package proof |
| macOS Intel/universal | `x86_64` or universal | DMG; later updater archive | same macOS components | deferred pending dependency and native-matrix qualification |
| Chrome extension | current compatibility window | Store ZIP/package | shared extension plus Chrome manifest/ID | release coordinated after compatible desktop/host |
| Edge extension | current compatibility window | Store ZIP/package | shared extension plus Edge manifest/ID | release coordinated after compatible desktop/host |

When Intel-Mac work resumes, a universal build is preferred only if every native
dependency supports it; otherwise separate `arm64` and `x86_64` artifacts come
from the same commit and version. Architecture differences never permit schema
or behavior differences.

## Identities and environments

| Environment | Identity/data | External services | Intended use |
|---|---|---|---|
| test | temporary per-test profile and fake vault | mocked; no updater/signing | unit, property, integration, fuzz |
| development | dev app ID, dev native-host name, dev extension IDs, isolated data | mocked by default; opt-in synthetic live probes | daily development |
| preview | separate preview app/data/channel and preview extension IDs | production endpoints only when explicitly tested | release-candidate and public preview |
| stable direct | stable direct app/data/update identity | signed catalog/update endpoints | trusted GitHub direct release |
| stable Store | Store app/data/update identity | Store update ownership | Windows fallback stable release |

No environment may open another environment's database or vault namespace. Moving data between preview, direct, and Store identities requires the documented backup/import path.

## Local development contract

The bootstrap implementation supplies these stable commands:

```text
just bootstrap                 verify/install pinned workspace tools
just generate                  regenerate schemas and bindings
just check                     format, lint, unit, license, vulnerability, drift checks
just dev                       run desktop with synthetic isolated dev data
just dev-extension chrome      build/load Chrome development package
just dev-extension edge        build/load Edge development package
just test-integration          storage, renderer, worker, IPC, mocked-provider tests
just test-platform             current-OS vault, sandbox, drag, native-host tests
just package-preview           current-OS unsigned preview artifacts
just verify-artifacts          signatures, hashes, manifests, SBOM, provenance
```

Commands are platform-neutral intents even when their underlying implementation differs. Unsupported current-host operations return a clear instruction to use the matching CI runner rather than silently skipping evidence.

## CI and deployment workflows

### Pull-request workflow

`ci.yml` runs without release secrets:

1. validate pinned Rust/Node/pnpm toolchains and lockfiles;
2. regenerate contracts into a temporary tree and fail on drift;
3. format, lint, static-analyze, test, and scan dependencies/licenses/secrets;
4. build the macOS-arm64 qualification target and retained Windows/Intel-Mac shared-source portability targets;
5. build Chrome and Edge extension variants and compare shared bundles for unintended behavior drift;
6. run synthetic storage, document, No-AI import, IPC, AI, and accessibility suites;
7. verify production Tauri capabilities/CSP contain no broad privilege or remote asset;
8. publish non-release test evidence only.

### Scheduled security/platform workflow

Nightly or scheduled jobs run fuzz corpora, parser-worker sandbox tests, vault access matrices, provider contract probes, Codex compatibility/containment probes, clean-machine native-host registration, and reproducibility comparisons. A scheduled result can block promotion even when an earlier pull request passed.

### Preview workflow

`preview.yml` builds one immutable artifact set from an approved commit:

1. compile the macOS-arm64 preview and Chrome/Edge targets; deferred platform artifacts may be attached as non-qualified CI evidence but are not published as supported previews;
2. create checksums, SBOMs, license inventory, provenance, compatibility manifest, and preview update metadata where allowed;
3. run clean-account install/launch/repair/uninstall and extension interoperability tests on the exact macOS-arm64 artifacts;
4. label unsigned macOS outputs prominently and prevent stable-channel update ownership;
5. publish only to a draft/prerelease after required evidence passes.

### Stable release workflow

`release.yml` starts from an approved signed version tag and protected environment:

1. build once from the immutable source revision using pinned inputs;
2. generate the complete artifact/evidence set;
3. sign eligible Windows binaries/installer through the approved SignPath policy or route the fallback package through the Store;
4. when macOS stable signing exists, sign nested code, apply hardened runtime, notarize, and staple; otherwise retain preview classification;
5. verify every code signature, updater signature, digest, provenance statement, channel identity, and compatibility field;
6. install and test the exact signed artifacts without rebuilding;
7. package Chrome and Edge variants against the released desktop/native-host compatibility range;
8. create a draft GitHub Release and Store submissions;
9. require human release approval, then promote the already-tested artifacts;
10. update the website from the signed release/compatibility manifest.

## Deployment and Store order

For a compatible protocol change, deployment order is:

1. release desktop and native host with support for both the current and next additive protocol;
2. verify adoption/availability of the desktop package;
3. submit Chrome and Edge extension updates using only behavior the released desktop accepts;
4. keep the previous protocol during the documented compatibility window;
5. remove old protocol support only in a later coordinated desktop release.

An extension must never require an unreleased desktop. Emergency capability disablement occurs in the desktop/host without causing the extension to capture passively or retain content.

## Version ownership

One release manifest records independently visible versions for desktop application, native host, extension protocol, Chrome package, Edge package, database, backup, document schema, renderer/templates/fonts, AI prompts/schemas, provider catalog, updater security floor, and Codex compatibility range.

The desktop product version may be shared across Windows and macOS even when packaging revisions differ. Store resubmissions that do not change product behavior still record their artifact/package revision and exact digest. Generated contracts are the interoperability authority; handwritten duplicate protocol types are forbidden.

## Mandatory gates before feature enablement

- SQLCipher/vault gate before storing real user content or secrets.
- Hostile-document worker sandbox gate before enabling PDF/DOCX import on a target.
- Native-host registration and IPC/vault-sharing gate before advertising browser integration on a package/channel.
- Direct-provider contract, privacy, accounting, and guardrail gates before enabling each provider/model preset.
- Codex executable-identity and OS-containment gate before showing Codex as available.
- Renderer/DOCX fidelity, license, and accessibility gates before calling an export format supported.
- Trusted-signing/channel gate before labeling macOS artifacts stable; later platforms repeat their own gate.

A failed optional-feature gate disables only that feature where the product plans allow it. It does not authorize a weaker fallback.

## Rollback and recovery

- Preview and stable identities remain separate, preventing preview migrations from corrupting stable data.
- Database migrations create required safety copies and use forward fixes or compatible restoration rather than reverse SQL.
- A bad release is withdrawn without deleting evidence; recovery uses newly signed metadata and the last verified compatible artifact or a forward-fix release.
- Chrome/Edge rollback respects their Store controls and protocol compatibility window.
- Key compromise follows the independent updater, catalog, code-signing, Store, and website recovery paths in the distribution plan.

## Readiness to begin implementation

The project is ready to begin M0. Product scope, authority, architecture, source ownership, shared-versus-platform boundaries, initial target matrix, contracts, security gates, milestone order, CI layers, evidence layout, preview/stable identities, and rollback principles are defined.

M0 may start without SignPath approval, final Store acceptance, final parser library selection, or proven Codex containment because those are later gated deliverables. M0 must not claim those gates have passed or enable affected production capabilities.

### M0 entry checklist

- [x] Product and technical authority order documented.
- [x] Shared desktop and extension ownership decided.
- [x] Active macOS-arm64 and deferred platform responsibilities separated; Chrome/Edge responsibilities separated.
- [x] Repository tree and dependency direction proposed.
- [x] Security/privacy threat model and fail-closed feature gates documented.
- [x] Versioned contract generation and drift policy documented.
- [x] Development/preview/stable identities separated.
- [x] CI, release evidence, update ownership, and rollback model planned.
- [ ] Exact Rust, Node, pnpm, Tauri, SQLCipher, PDFium, Typst, and supporting dependency versions pinned by the M0 bootstrap change.
- [ ] Actual workspace, commands, CI workflows, synthetic fixtures, and health-command skeleton implemented and passing.

The unchecked items are the work of M0, not missing product decisions. After they pass, M1 begins under the delivery roadmap's encrypted-core gate.

## First implementation change

The first implementation pull request should contain only the M0 skeleton:

1. pinned toolchain and package-manager files;
2. Cargo and pnpm workspaces matching the approved source tree;
3. Tauri main/overlay shells with narrow development capabilities;
4. one generated `health` request/response contract exercised by both windows;
5. Chrome/Edge manifest-generation skeletons with development IDs and no broad host permission;
6. temporary/synthetic profile isolation;
7. `just bootstrap`, `just generate`, `just check`, and `just dev` intents;
8. macOS-arm64 qualification CI plus Windows/Intel-Mac portability builds, tests, contract drift, license/vulnerability, secret, and remote-asset checks;
9. initial ADRs and an empty versioned evidence manifest.

It should not yet implement real credentials, parsing, AI, updater installation, signing, or production native-host registration. Those enter only with their milestone-specific controls and evidence.
