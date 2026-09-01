# Technical implementation plans

## Status

These plans are **implementation-ready for the first development milestone**. They select the initial architecture, module boundaries, durable records, protocols, security controls, test strategy, and release workflow. Visual language, styling, layout polish, iconography, and other aesthetic decisions are deliberately excluded and remain owned by `../Aesthetic/`.

Implementation may refine library versions and internal names without changing product behavior. Any change to privacy promises, supported AI modes, data ownership, release channels, or the user-visible lifecycle must first be approved in `../Product Plans/`.

## Selected implementation baseline

| Area | Initial decision |
|---|---|
| Desktop shell | Tauri 2 on Windows and macOS |
| Desktop UI | React, TypeScript, and Vite, rendered entirely from bundled assets |
| Core/application services | Rust workspace; UI commands are thin adapters over testable use cases |
| Local database | SQLite encrypted with SQLCipher Community Edition; key held in the OS credential vault |
| Secrets | Windows Credential Manager and macOS Keychain through a narrow Rust vault abstraction |
| Document model | Versioned Rust domain types with generated JSON Schema and TypeScript contracts |
| Preview/PDF | One pinned Typst rendering pipeline; bundled fonts and templates |
| DOCX | Constrained Open XML importer/exporter behind a document adapter |
| Direct AI | Rust HTTPS adapters for OpenAI, Anthropic, and Gemini; no provider SDK in the core contract |
| Codex | Optional, separately installed Codex app-server over `stdio`; never bundled with ORT |
| Browser bridge | Shared Manifest V3 extension plus a Rust native-messaging host and authenticated local IPC |
| Packaging | Tauri NSIS/DMG packages; SignPath-first Windows signing; unsigned macOS preview initially |
| Updates | Signed ORT update metadata and GitHub Release assets, with channel separation |
| Website | Static Astro/TypeScript site on Cloudflare Pages, documented in the private repository |

Exact dependency versions are chosen and locked when the workspace is bootstrapped. They must pass the license, security, and platform-build gates in the plans below.

## Plan map

### System documentation

- [`System Documentation/Architecture.md`](System%20Documentation/Architecture.md) — component boundaries, execution model, dependency direction, data flows, and failure domains.
- [`System Documentation/Repository_and_Build.md`](System%20Documentation/Repository_and_Build.md) — proposed source tree, contracts, local development, dependency policy, and CI layers.
- [`System Documentation/Security_and_Threat_Model.md`](System%20Documentation/Security_and_Threat_Model.md) — trust boundaries, controls, abuse cases, and release-blocking security gates.
- [`System Documentation/Delivery_Roadmap.md`](System%20Documentation/Delivery_Roadmap.md) — vertical milestones, dependencies, evidence, and explicit deferrals.
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
5. Add tracker/workspace workflows and browser native messaging.
6. Add external Codex support only after the containment proof passes on both supported operating systems.
7. Harden packaging, signing, update, recovery, accessibility, and release evidence.
8. Build and publish the static project website after download channels have real artifacts.

## Blocking feasibility gates

The following are not invitations to silently weaken the product requirements. If a gate fails, the affected feature remains disabled or the release channel changes as already allowed by the product plans.

- **SQLCipher packaging:** reproducible Windows/macOS builds, encrypted WAL/temp behavior, and license attribution must pass before real user data is stored.
- **Codex containment:** ORT must prove that an app-server child cannot use tools, read arbitrary files, execute commands, or make network requests outside the approved Codex service path. Codex mode remains disabled in stable builds until this is demonstrated on both platforms.
- **SignPath approval:** signed GitHub distribution is the preferred Windows stable channel. Until approval, Windows artifacts are clearly labeled preview; the Microsoft Store path remains the fallback.
- **Native host registration:** direct and Store-style Windows installation paths must each prove install, repair, update, and uninstall behavior before their extension integration is advertised.
- **Document fidelity:** the renderer/importer must pass the golden corpus and accessibility checks before PDF/DOCX formats are called supported.

## Definition of implementation-ready

A milestone may enter coding when its plan identifies its inputs, records, interfaces, state transitions, errors, permissions, tests, rollout behavior, and evidence location. A milestone is complete only when its automated tests and required manual verification are attached to a tagged build or release candidate.
