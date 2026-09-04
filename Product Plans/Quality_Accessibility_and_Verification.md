# Quality, accessibility, and verification

## Quality principles

- Critical user actions fail safely and preserve the last valid local state.
- The project makes no durability promise without tested backup and migration behavior.
- The same structured input produces consistent preview and export within a declared renderer/template version.
- Provider outages or API changes degrade AI features without blocking manual editing, rendering, tracking, backup, or export.
- Desktop, extensions, native host, updater, and local schema are versioned and tested as one compatibility system.

## Requirement traceability

Future implementation planning assigns stable identifiers using these categories:

- `PRD` — product-wide principles, scope, terminology, and settings.
- `RES` — master draft, publish, schema, import review, editing, and rendering.
- `APP` — application workspace, tailoring, Finish Application, tracker, and retained materials.
- `AI` — direct-provider and Codex adapters, credentials/authentication, model catalogs, prompts, validation, generation, activity/usage accounting, pricing estimates, spend/quota guardrails, and evaluation.
- `DAT` — local storage, encryption, migrations, retention, backup, export, and deletion.
- `EXT` — browser capture, native messaging, local IPC, installation, and repair.
- `SEC` — privacy, security, import isolation, diagnostics, and supply chain.
- `DST` — packaging, signing, Stores, releases, updates, and rollback.
- `OSS` — license, dependencies, contributions, governance, and release provenance.
- `QLT` — compatibility, performance, accessibility, document quality, and release evidence.

Each traceability record contains the requirement, authoritative file/section, applicable platforms and data categories, owning component, dependencies and central configuration values, verification type, objective pass criterion, evidence location, owner, status, and applicable release gate.

## Accessibility

The desktop app, overlay, and extensions target WCAG 2.2 AA principles where applicable and equivalent native-platform accessibility expectations.

Required coverage includes:

- Complete keyboard operation and logical focus order
- Screen-reader labels, roles, status announcements, and error recovery
- High contrast, scalable text, zoom, and reduced-motion behavior
- No color-only meaning
- Accessible dialogs for destructive, provider-transmission, update, and Finish Application decisions
- Accessible AI Monitoring period controls, token/cost graphs with equivalent textual summaries, aggregate breakdowns, mode/model controls, spending/quota-cap forms, progress warnings, blocked states, exports, and clearing/reset confirmations without color-only cost, quota, or error meaning
- Readable exported documents with selectable text, meaningful ordering, links, and appropriate tagging where supported
- Overlay behavior that does not trap focus or obstruct essential browser/OS controls
- Required Qualification Alerts that are keyboard and screen-reader accessible, do not rely on color alone, expose the requirement and evidence relationship clearly, can be dismissed/ignored/reopened, and never seize focus or block the workflow

## Supported compatibility matrix

The active M0-M2 qualification matrix is macOS Apple Silicon only. Exact minimum macOS versions are chosen before distribution and recorded centrally. During the current phase, test:

- The developer-controlled macOS-arm64 version plus any additional macOS-arm64 versions explicitly added to the matrix
- Current stable Chrome and Edge plus a documented compatibility window
- Development and unsigned preview identities; Developer ID/notarized distribution remains a release gate
- Fresh install, upgrade, repair, uninstall, extension-first installation, and desktop-first installation
- Multiple browser profiles and distinct Chrome/Edge extension identifiers

Windows and Intel-Mac matrices are retained as later qualification work. Shared
CI compilation and deterministic tests on them may detect portability regressions,
but they do not replace native vault, sandbox, installer, WebView, accessibility,
filesystem, lifecycle, and clean-machine testing.

## Critical end-to-end journeys

1. First launch, local-profile creation, backup explanation, and manual resume creation without AI.
2. Import a text-bearing resume in No AI mode, verify lossless local mapping/custom-section review with no network, correct the proposal, save the draft, and publish; repeat with a configured provider and separate transmission confirmation.
3. Edit a published draft without those unpublished changes leaking into tailoring.
4. From overlay Stage 1, capture and review/edit a job description through Chrome and Edge, capture again, and continue, including browser-gesture, desktop-not-running, and repair cases.
5. In overlay Stage 2, tailor, inspect no more than three verified change points and Required Qualification Alerts, dismiss/ignore/reopen alerts, enlarge preview/edit, and require a correction prompt before resume regeneration.
6. Generate and preview/edit a cover-letter PDF; capture/review an application question, generate and edit an answer, copy it, then reset and capture another; refuse a prohibited attestation.
7. Download and drag the current resume and cover-letter PDFs from their overlay cards, verify the browser-rejected-drop fallback, and render/validate PDF, DOCX, and text outputs across representative content lengths and templates.
8. Finish with selected structured materials, recover safely from a failed save, and reset temporary content.
9. Open a historical structured snapshot and render it after application and renderer upgrades.
10. Create an encrypted backup, restore on a clean profile, handle the missing AI credential, and verify integrity.
11. Delete selected data and all local data without affecting unrelated files.
12. Configure each OpenAI, Anthropic, and Gemini direct adapter; complete successful, failed, retried, cancelled, ambiguous, and interrupted calls; verify Week/Month/Year/All time token and estimated-cost graphs/totals, aggregate breakdowns, accessible text equivalents, internal attempt accounting, provenance, export, retention, and date-range clearing.
13. Enable weekly/monthly/yearly/all-time direct spending caps and prove warnings, atomic reservation, boundary reset, fail-closed unknown pricing/usage, crash recovery, credential replacement, and the separation between clearing activity and resetting a cap.
14. Connect and sign out of Codex through browser and device-code paths; discover tested account models; verify isolated no-tool execution, ORT thread tokens where available, account-wide daily/lifetime tokens, exact quota windows, delayed/rounded updates, quota thresholds, bucket changes, and missing telemetry labels.
15. Switch between Direct API, Codex subscription, and No AI without leaking credentials, changing an active operation, or silently falling back.
16. Verify that unsigned macOS previews remain clearly labeled and do not enable
    an unauthenticated automatic updater; later test Developer ID-signed macOS
    updates without losing data/native messaging. SignPath and Microsoft Store
    journeys are deferred with Windows qualification.

## Security and privacy verification

- Static analysis, dependency/license scanning, secret scanning, and malicious import tests run in CI.
- Release checks verify that `LICENSE` remains the unmodified GPLv3 text; required copyright, canonical-source, Section 7, third-party, and trademark notices ship in source and binary distributions; and About/Legal content agrees with the release channel and signing identity.
- Native-message fuzzing and malformed IPC tests verify bounded behavior.
- Logs, crash output, diagnostics, backups, exports, and update requests are inspected for forbidden content and secrets.
- Threat modeling covers web capture, import parsers, provider calls, local storage, IPC, updates, release CI, and signing.
- A security review is required before representing a build as stable for broad public use.

## AI evaluation

Use synthetic or explicitly authorized representative resumes and job descriptions to evaluate each supported provider/model configuration for:

- Structured-output reliability
- Unsupported factual additions
- Important accidental omissions
- Prompt-injection resistance
- Change-summary accuracy
- Required-versus-preferred qualification classification; supported resume-category mapping; confirmed-mismatch and not-found accuracy; resolvable job/resume evidence; duplicate suppression; and exclusion of ambiguous, personal, protected, and legal-attestation requirements
- Prohibited-answer refusal
- Length/page-target adherence
- Latency, cancellation, and actionable failure behavior
- Input/output size and user-visible estimated cost
- Usage normalization and cost calculations across input, output, cached, reasoning, missing, and provider-specific billing categories
- Separation of logical operations from provider-call attempts and accurate retry aggregation
- Per-model and per-provider totals with partial-data and cross-currency cases
- Direct-spend reservation/settlement accuracy at every supported period boundary and under failures
- Codex requested/effective model behavior, no-tool containment, token telemetry provenance, quota-window display, and threshold enforcement

Provider presets are versioned. A provider/model change requires re-evaluation and release notes.

## Release gates

A stable release requires:

- All critical journeys pass on the active macOS-arm64 matrix. Each later platform must pass the full applicable matrix before it is described as supported.
- Local migrations and backup restoration pass from every supported prior version.
- Export clipping, link, selectable-text, and font tests pass on representative documents.
- Store/direct installers register, repair, update, and remove native messaging as documented.
- Update signature/provenance checks and rollback/recovery exercises pass.
- No known critical security vulnerability or secret exposure remains.
- License and distributed-asset review is complete.
- Privacy, provider-transmission, backup, unsigned-build, and local-only limitations are truthful in the application and its distributed documentation.
- The internal ledger records every ORT provider attempt without forbidden content and survives interruption; aggregate AI Monitoring graphs/totals export and delete correctly and never represent an estimate as an invoice or ORT-only Codex account statement.
- Required Qualification Alerts are generated within the existing tailoring call, contain only validated explicit requirement/evidence references, use no fit score or eligibility claim, remain non-blocking and dismissible, and are removed with the workspace rather than retained in the tracker.
- Direct spending caps cannot be bypassed by retry, concurrency, crash, activity deletion, clock change, or missing data; Codex caps block future operations based on freshly reported stable quota buckets and clearly disclose best-effort limits.
- Hostile PDF/DOCX inputs cannot escape the disposable parser worker, reach secrets/user files/network, spawn surviving children, or mutate canonical data; No-AI import preserves every extracted block for review.
- Vault tests demonstrate macOS desktop/native-host access controls, cross-account denial, and identity continuity without plaintext fallback or cross-secret access. The documented Windows same-user boundary is a later Windows qualification gate.
- Known limitations and deferred features are documented.

## Evidence

Future implementation plans assign stable requirement identifiers, test ownership, objective pass criteria, and evidence locations. A feature is not complete solely because its happy-path interface exists.

A release cannot satisfy a gate with an untested critical requirement, unexplained failure, evidence drawn from unauthorized personal content, or a manual assertion where an objective automated or repeatable check is feasible.
