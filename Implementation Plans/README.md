# Technical implementation plans

## Status

These plans select the architecture, module boundaries, durable records, protocols, security controls, test strategy, and release workflow. M0–M2 are complete for the macOS Apple Silicon development scope; see the [M2 acceptance closure](../evidence/0.0.0-dev/m2-acceptance-closure.md). The next milestone is M2.5: usable desktop layout, approved aesthetic and professional resume designs, before M3. The approved visual direction remains owned by `../Aesthetic/`; exact component polish and the two non-default document-template layouts may be tested and refined during development without weakening their release gates.

Implementation may refine library versions and internal names without changing product behavior. Any change to privacy promises, supported AI modes, data ownership, release channels, or the user-visible lifecycle must first be approved in `../Product Plans/`.

## Active platform scope

M0-M2 development and qualification currently target macOS Apple Silicon only.
The architecture continues to isolate operating-system code so Windows and Intel
Mac can be added later without forking contracts, schemas, or product behavior.
Existing non-arm64 CI builds remain useful portability regression signals, but
they are not current native qualification and do not add Windows or Intel-Mac
work to the M0-M2 exit gates.

## Selected implementation baseline

| Area | Initial decision |
|---|---|
| Desktop shell | Tauri 2 on macOS Apple Silicon now; Windows and Intel Mac retained as later adapters |
| Desktop UI | React, TypeScript, and Vite, rendered entirely from bundled assets |
| Core/application services | Rust workspace; UI commands are thin adapters over testable use cases |
| Local database | SQLite encrypted with SQLCipher Community Edition; key held in the OS credential vault |
| Secrets | macOS Keychain through a narrow Rust vault abstraction now; Windows Credential Manager adapter retained for later qualification |
| Document model | Versioned Rust domain types with generated JSON Schema and TypeScript contracts |
| Preview/PDF | One pinned Typst rendering pipeline; bundled fonts and templates |
| DOCX | Constrained Open XML importer/exporter behind a document adapter |
| Direct AI | Rust HTTPS adapters for OpenAI, Anthropic, and Gemini; no provider SDK in the core contract |
| Codex | Optional, separately installed Codex app-server over `stdio`; never bundled with ORT |
| Browser bridge | Shared Manifest V3 extension plus a Rust native-messaging host and authenticated local IPC |
| Packaging | Tauri macOS-arm64 app/DMG preview initially; NSIS/Windows signing and Intel/universal Mac packages later |
| Updates | Signed ORT update metadata and GitHub Release assets, with channel separation |
| Website | Static Astro/TypeScript site on Cloudflare Pages, documented in the private repository |

Exact dependency versions are chosen and locked when the workspace is bootstrapped. They must pass the license, security, and platform-build gates in the plans below.

## Plan map

### System documentation

- [`System Documentation/Architecture.md`](System%20Documentation/Architecture.md) — component boundaries, execution model, dependency direction, data flows, and failure domains.
- [`System Documentation/Repository_and_Build.md`](System%20Documentation/Repository_and_Build.md) — proposed source tree, contracts, local development, dependency policy, and CI layers.
- [`System Documentation/Development_and_Deployment_Outline.md`](System%20Documentation/Development_and_Deployment_Outline.md) — shared-code ownership, build targets, environments, artifact matrix, CI/deployment flow, gates, rollback, and M0 readiness checklist.
- [`System Documentation/Security_and_Threat_Model.md`](System%20Documentation/Security_and_Threat_Model.md) — trust boundaries, controls, abuse cases, and release-blocking security gates.
- [`System Documentation/Delivery_Roadmap.md`](System%20Documentation/Delivery_Roadmap.md) — vertical milestones, dependencies, evidence, and explicit deferrals.
- [`Next_Milestones.md`](Next_Milestones.md) — M3 starting route and ordered navigation for M4–M8; it summarizes and does not supersede the delivery roadmap.
- [`System Documentation/Requirement_Traceability.md`](System%20Documentation/Requirement_Traceability.md) — stable requirement IDs for issues, tests, and release evidence.

### Component plans

- [`Desktop Application/Desktop_Application_Plan.md`](Desktop%20Application/Desktop_Application_Plan.md)
- [`AI and Document Processing/AI_and_Document_Processing_Plan.md`](AI%20and%20Document%20Processing/AI_and_Document_Processing_Plan.md)
- [`Local Data and Migration/Local_Data_and_Migration_Plan.md`](Local%20Data%20and%20Migration/Local_Data_and_Migration_Plan.md)
- [`Browser Extensions/Browser_Extension_and_IPC_Plan.md`](Browser%20Extensions/Browser_Extension_and_IPC_Plan.md)
- [`Distribution and Updates/Distribution_and_Updates_Plan.md`](Distribution%20and%20Updates/Distribution_and_Updates_Plan.md)

## Delivery order

1. Bootstrap contracts, Rust workspace, desktop shell, test fixtures, and CI.
2. Implement encrypted storage, schema migrations, backup primitives, and the structured resume model.
3. Deliver the offline editor/publish/preview/export path before connecting any AI service.
4. Add direct-provider adapters, operation accounting, guardrails, tailoring, and Required Qualification Alerts.
5. Add the overlay-owned Stage 1/Stage 2 application workflow, tracker, PDF Download/drag handoff, and browser native messaging.
6. Add external Codex support only after the containment proof passes on both supported operating systems.
7. Harden packaging, signing, update, recovery, accessibility, and release evidence.
8. Build and publish the static project website after download channels have real artifacts.

For the active handoff, begin with [`Next_Milestones.md`](Next_Milestones.md).

## Feature safeguards

Testing depth follows the [focused quality policy](../Product%20Plans/Quality_Accessibility_and_Verification.md).
The following are implementation safeguards, not requirements for exhaustive test campaigns.

The following are not invitations to silently weaken the product requirements. If a gate fails, the affected feature remains disabled or the release channel changes as already allowed by the product plans.

- **SQLCipher packaging:** reproducible macOS-arm64 builds, encrypted WAL/temp behavior, and license attribution must pass before real user data is stored. Each later platform repeats the gate.
- **Vault boundary:** exact macOS credential behavior, cross-account and desktop/native-host access separation, development/preview identity, move/update/repair behavior, and no-plaintext fallback must pass before real secrets are stored. Windows proof is deferred with Windows qualification.
- **Hostile-document isolation:** PDF/DOCX import must run only in the disposable OS-sandboxed worker. If a platform/package cannot deny user files, secrets, network, and subprocesses and kill the worker tree reliably, import remains disabled there.
- **Codex containment:** ORT must prove that an app-server child cannot use tools, read arbitrary files, execute commands, or make network requests outside the approved Codex service path. During the active phase, macOS-arm64 proof gates the feature; later platforms must repeat it before enabling Codex there.
- **SignPath approval:** signed GitHub distribution is the preferred Windows stable channel. Until approval, Windows artifacts are clearly labeled preview; the Microsoft Store path remains the fallback.
- **Native host registration:** direct and Store-style Windows installation paths must each prove install, repair, update, and uninstall behavior before their extension integration is advertised.
- **Document fidelity:** the renderer/importer must pass the golden corpus and accessibility checks before PDF/DOCX formats are called supported.

## Definition of implementation-ready

A work item needs a clear outcome, relevant data/permission impact and the roadmap's focused checks. Close a milestone with a brief build/commit-linked result note and
known limitations; a tagged candidate or elaborate evidence package is not required.
