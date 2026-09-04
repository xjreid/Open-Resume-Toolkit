# Open Resume Toolkit plan index

## Purpose

This is the navigation and authority map for the product and technical implementation plans. A person or AI agent should read this file before reviewing or implementing the product.

## Current planning status

The product direction and core workflows are defined. Implementation is underway
at M2 on the selected Tauri 2, React/TypeScript, Rust, SQLCipher and Typst
baseline. The development build supports encrypted structured editing,
publication, local exports/rendering, portable backup/recovery, storage reporting
and exact crash-resumable local-profile deletion. A common fail-closed
parser-supervision coordinator is implemented, but hostile document import remains
disabled pending native macOS/Windows adapters and containment proof; AI, browser messaging,
updates and release hardening remain later milestones. Quiet Navy/Open Frame is
the approved application direction. M2 completion includes the complete manual
master-editor behavior in the authoritative resume-editor plan and all three
qualified PDF/DOCX style categories; the current plain fixtures do not satisfy
that gate. Historical replay retains immutable structured sources and uses a
truthfully identified current renderer when the original tuple is unavailable,
without retaining superseded drafts or old executable renderer bundles solely
for replay.

## Authority and precedence

When documents conflict, use this order:

1. The product plan identified below as authoritative for the exact subject.
2. The open-decisions register for an explicitly unresolved subject.
3. Supporting summaries and cross-cutting guidance.
4. Technical implementation plans, for implementation details that do not redefine product behavior.
5. Reserved aesthetic planning.

If two product plans genuinely conflict, update both deliberately. Precedence is not permission to leave contradictions.

## Current key decisions

- The public name is **Open Resume Toolkit**, abbreviated **ORT** where a short identifier is needed.
- ORT is free and open source. It is not described as a nonprofit.
- The repository license is `GPL-3.0-only`, supplemented for identified original project material by the reasonable author-attribution term in `ADDITIONAL_TERMS.md` as permitted by GPLv3 Section 7(b). Commercial use, sale, modification, and redistribution remain permitted. The exact dependency, contribution, and additional-term compatibility remains subject to legal and license review before the first stable release.
- `NOTICE` identifies the canonical source repository and copyright attribution. `TRADEMARKS.md` requires modified and third-party distributions to avoid implying that they are official releases; it does not restrict GPL rights in the code.
- The desktop application is the primary product. Chrome and Edge extensions are narrow capture companions, not replacements for the desktop experience.
- User content is local-first and remains on the user's computer. ORT operates no account service, ORT subscription, product database, cloud document store, provider quota, admin portal, or centrally funded AI key. Optional local guardrails govern only future ORT calls.
- The desktop app remains useful without AI for manual resume editing, deterministic reviewed PDF/DOCX import into existing/custom sections, local document rendering, application tracking, and exports.
- AI features use exactly one active mode: a deliberately configured OpenAI, Anthropic, or Gemini API key; a managed ChatGPT/Codex subscription connection; or No AI. API keys and Codex tokens use the operating-system credential vault and are never exposed to the browser extension. Vault storage protects the documented OS-user boundary and is not claimed to defeat malware already executing as the same unlocked user.
- Direct API models use curated Economy/Balanced/Quality presets. Codex models are the tested intersection of the account's current app-server model list and ORT compatibility catalog; no adapter silently falls back or enables an untested model.
- A private local ledger records every provider call initiated by ORT. The user-facing AI Monitoring view shows aggregate token/direct-cost graphs and totals for Week/Month/Year/All time plus secondary breakdowns; it is not an individual-call feed. Direct totals cannot claim to show calls made by other applications using the key; Codex account summaries are visibly account-wide and have no invented dollar cost.
- Users may configure optional direct estimated-spend caps for calendar week/month/year/all-time periods or Codex provider-quota percentage thresholds. Guardrail state is durable and separate from activity-history deletion, while provider billing and quota controls remain authoritative.
- AI requests require internet access unless a later local-model adapter is installed. Ordinary editing, tracking, rendering, and local search should work offline.
- There is exactly one autosaved master-resume draft and at most one deliberately published master-resume snapshot in a local profile.
- Tailoring uses the published master, never unpublished draft changes.
- The tailoring operation also produces optional Required Qualification Alerts for explicit mandatory job requirements that map to resume content. Alerts distinguish a confirmed conflict from a qualification not found in the published master, ignore preferred/ambiguous/personal-or-legal requirements, never become a fit score, and never block the user from continuing.
- One active overlay-owned application workspace has Stage 1 capture/review and Stage 2 Resume/Cover letter/Answers tabs. The main window has no job-specific route.
- Resume and cover-letter PDF cards provide both Download and temporary operating-system drag-out; expanded preview edits structured source, not PDF bytes.
- Quiet Navy/Open Frame is the approved universal light-only application/website direction. Resume and cover-letter documents remain visually independent; the default Technical template follows the supplied Jake's Resume reference as licensing permits.
- Finish Application optionally creates a local tracker entry with the selected final resume, cover letter, and approved answer set as structured snapshots, then deletes unselected temporary material and resets the workspace.
- Structured JSON is the canonical document format. PDF, DOCX, and plain text are locally rendered derived artifacts and are not retained by default.
- There are no ORT plan-based application, import, AI, or storage quotas. Practical safety limits prevent malformed or excessively large inputs, and optional user-defined AI guardrails protect the user's provider budget or subscription quota.
- Chrome and Edge on Windows and macOS are the initial browser/platform targets. Firefox, Safari, Linux, mobile, OCR, local AI models, job-board notifications, and automatic form submission are not initial commitments.
- Codex subscription mode requires a separately installed compatible Codex runtime; ORT does not bundle Codex in the initial release. ORT detects and validates the runtime, launches and stops `codex app-server` automatically over local `stdio`, and keeps its configuration, authentication, keyring, and temporary-data boundary separate from the user's general Codex environment.
- The preferred stable Windows channel is a SignPath Foundation-signed direct package published through canonical GitHub Releases after the project meets SignPath eligibility and is accepted. Before trusted signing is available, direct Windows artifacts are clearly labeled previews. Microsoft Store MSIX is the fallback stable channel if SignPath is unavailable or declined. The app performs or verifies per-user native-messaging registration without requiring manual registry editing in the normal flow.
- Signed direct installations may check canonical GitHub Releases; a fallback Store edition uses Microsoft Store update APIs. Update channels must never overwrite one another.
- Initial macOS GitHub artifacts are explicitly unsigned previews with checksums, provenance, Gatekeeper instructions, and authenticated update metadata. A stable macOS release requires Apple Developer ID signing and notarization once usage, support burden, organizational adoption, or funding justifies the recurring membership cost.

## Authoritative document catalog

### Entry and coordination

- [README](README.md) — workspace entry point and boundaries.
- [Plan index](Plan_Index.md) — navigation, authority, current decisions, and document roles.
- [Release scope and open decisions](<Product Plans/Release_Scope_and_Open_Decisions.md>) — the only unresolved/deferred-work register.
- [GNU GPLv3 license](LICENSE) — unmodified `GPL-3.0-only` license text.
- [Copyright and canonical-source notice](NOTICE) — project attribution and canonical repository.
- [Additional attribution term](ADDITIONAL_TERMS.md) — GPLv3 Section 7(b) attribution requirement for identified original material.
- [Trademark policy](TRADEMARKS.md) — permitted name use and official-versus-third-party distribution boundaries.
- [Contribution guidelines](.github/CONTRIBUTING.md) — current contribution scope, pull-request expectations, inbound license, and DCO sign-off.
- [Code of Conduct](.github/CODE_OF_CONDUCT.md) — community behavior, reporting, moderation, and enforcement expectations.
- [Security policy](.github/SECURITY.md) — private vulnerability-reporting process and supported-release status.

### Product definition

- [Product scope and principles](<Product Plans/Product_Scope_and_Principles.md>) — mission, audience, surfaces, terminology, goals, and non-goals.
- [Resume editor and schema](<Product Plans/Resume_Editor_and_Schema.md>) — detailed editor layout, section and entry types, fields, ordering, optional-value rendering, and draft/publish experience.
- [Core workflows](<Product Plans/Core_Workflows.md>) — master resume, overlay capture/tailoring/material generation, AI Monitoring, Finish Application, and tracker behavior.
- [Local data and document model](<Product Plans/Local_Data_and_Document_Model.md>) — local records, ownership, draft/publish rules, structured snapshots, AI activity/guardrails, Codex usage cache, backup, deletion, rendering, and migration.
- [Product states and operations](<Product Plans/Product_States_and_Operations.md>) — canonical resume, workspace, import, AI connection/operation/activity/guardrail, tracker-save, and update states.
- [Configuration limits and defaults](<Product Plans/Configuration_Limits_and_Defaults.md>) — centralized compatibility, payload, document, AI model, spend/quota guardrail, timeout, retention, and performance defaults.
- [Local data retention and recovery](<Product Plans/Local_Data_Retention_and_Recovery.md>) — local retention, temporary cleanup, AI activity history, backups, exports, credentials, diagnostics, and deletion behavior.
- [AI and import](<Product Plans/AI_and_Import.md>) — direct providers/models, Codex subscription integration, credentials, spend/quota guardrails, request safeguards, usage/cost accounting, import, generation, factuality, and failure behavior.
- [Desktop-extension communication](<Product Plans/Desktop_Extension_Communication.md>) — Chrome/Edge capture, native messaging, local IPC, installation, permissions, and version compatibility.
- [Distribution and updates](<Product Plans/Distribution_and_Updates.md>) — GitHub/Store channels, signing, application/catalog updates, external Codex-runtime requirements, release integrity, and extension stores.
- [Security, privacy, and open source](<Product Plans/Security_Privacy_and_Open_Source.md>) — threat boundaries, privacy promises, local security, open-source governance, licensing, and distributed documentation requirements.
- [Quality, accessibility, and verification](<Product Plans/Quality_Accessibility_and_Verification.md>) — quality requirements, accessibility, critical journeys, and release evidence.

### Technical implementation and future aesthetic work

- [Implementation plans](<Implementation Plans/README.md>) — selected technical baseline, component plans, delivery order, and implementation gates.
- [Development and deployment outline](<Implementation Plans/System Documentation/Development_and_Deployment_Outline.md>) — shared desktop/extension source ownership, platform and Store artifacts, environments, CI/release sequence, mandatory gates, rollback, and M0 readiness.
- Component folders under `Implementation Plans/` — implementation-ready plans for architecture/security/build, desktop, extensions/IPC, local data/migration, AI/document processing, and distribution/updates.
- [Aesthetic planning](Aesthetic/README.md) — approved Quiet Navy/Open Frame direction, surface responsibilities, document independence, and remaining visual deliverables.

## Recommended reading routes

### Product review

1. Product scope and principles
2. Core workflows
3. Product states and operations
4. Local data and document model
5. Configuration limits and defaults
6. AI and import
7. Release scope and open decisions

### Implementation handoff

1. Read every product plan.
2. Read the open-decisions register completely.
3. Read the architecture, security, repository/build, and delivery-roadmap plans.
4. Read the development and deployment outline.
5. Read the applicable component implementation plan.
6. Link code and tests to the product requirement and technical completion criteria.
7. Record major irreversible refinements as architecture decision records.

### Security and privacy review

1. Security, privacy, and open source
2. Local data and document model
3. Local data retention and recovery
4. AI and import
5. Desktop-extension communication
6. Distribution and updates

## Change control

- A product-rule change must first update its authoritative product plan, this index when it affects a key decision, and every impacted workflow summary.
- Do not copy numeric values between documents when one authoritative location can be referenced.
- Do not present deferred work as an initial-release promise.
- Preserve schema migration, export, and user-data recovery whenever an implementation choice changes.
