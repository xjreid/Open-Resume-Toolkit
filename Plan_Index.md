# Open Resume Toolkit plan index

## Purpose

This is the navigation and authority map for the product plans. A person or AI agent should read this file before reviewing the product or creating technical implementation plans.

## Current planning status

The product direction and core workflows are defined. Exact frameworks, schemas, APIs, operating-system integrations, provider adapters, build systems, installers, and visual design are intentionally deferred to the reserved workspaces.

## Authority and precedence

When documents conflict, use this order:

1. The product plan identified below as authoritative for the exact subject.
2. The open-decisions register for an explicitly unresolved subject.
3. Supporting summaries and cross-cutting guidance.
4. Reserved implementation and aesthetic placeholders.

If two product plans genuinely conflict, update both deliberately. Precedence is not permission to leave contradictions.

## Current key decisions

- The public name is **Open Resume Toolkit**, abbreviated **ORT** where a short identifier is needed.
- ORT is free and open source. It is not described as a nonprofit.
- The repository license is `GPL-3.0-only`, supplemented for identified original project material by the reasonable author-attribution term in `ADDITIONAL_TERMS.md` as permitted by GPLv3 Section 7(b). Commercial use, sale, modification, and redistribution remain permitted. The exact dependency, contribution, and additional-term compatibility remains subject to legal and license review before the first stable release.
- `NOTICE` identifies the canonical source repository and copyright attribution. `TRADEMARKS.md` requires modified and third-party distributions to avoid implying that they are official releases; it does not restrict GPL rights in the code.
- The desktop application is the primary product. Chrome and Edge extensions are narrow capture companions, not replacements for the desktop experience.
- User content is local-first and remains on the user's computer. ORT operates no account service, subscription, product database, cloud document store, quota system, admin portal, or centrally funded AI key.
- The desktop app remains useful without AI for manual resume editing, local document rendering, application tracking, and exports.
- AI features use a key and provider deliberately configured by the user. Credentials are stored in the operating-system credential vault and are never exposed to the browser extension.
- AI requests require internet access unless a later local-model adapter is installed. Ordinary editing, tracking, rendering, and local search should work offline.
- There is exactly one autosaved master-resume draft and at most one deliberately published master-resume snapshot in a local profile.
- Tailoring uses the published master, never unpublished draft changes.
- One active application workspace holds one reviewed job description, temporary tailored versions, optional cover-letter drafts, and optional application-answer drafts.
- Finish Application optionally creates a local tracker entry with the selected final resume, cover letter, and approved answer set as structured snapshots, then deletes unselected temporary material and resets the workspace.
- Structured JSON is the canonical document format. PDF, DOCX, and plain text are locally rendered derived artifacts and are not retained by default.
- There are no plan-based application, import, AI, or storage quotas. Practical safety limits prevent malformed or excessively large inputs; the user's disk capacity and chosen AI provider govern usage.
- Chrome and Edge on Windows and macOS are the initial browser/platform targets. Firefox, Safari, Linux, mobile, OCR, local AI models, job-board notifications, and automatic form submission are not initial commitments.
- The initial Windows distribution target is a Microsoft Store MSIX, with GitHub releases and possible SignPath signing evaluated for direct downloads. The app performs or verifies per-user native-messaging registration without requiring manual registry editing in the normal flow.
- Store installations update through Microsoft Store APIs. Direct installations may check signed GitHub Releases. Update channels must never overwrite one another.
- A polished direct macOS release requires Apple Developer ID signing and notarization. The project may offer a clearly labeled unsigned preview before funding the Apple membership, but must not disguise Gatekeeper limitations.

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
- [Core workflows](<Product Plans/Core_Workflows.md>) — master resume, capture, tailoring, supporting generation, Finish Application, and tracker behavior.
- [Local data and document model](<Product Plans/Local_Data_and_Document_Model.md>) — local records, ownership, draft/publish rules, structured snapshots, backup, deletion, rendering, and migration.
- [Product states and operations](<Product Plans/Product_States_and_Operations.md>) — canonical resume, workspace, import, AI-operation, tracker-save, and update states.
- [Configuration limits and defaults](<Product Plans/Configuration_Limits_and_Defaults.md>) — centralized compatibility, payload, document, AI, timeout, retention, and performance defaults.
- [Local data retention and recovery](<Product Plans/Local_Data_Retention_and_Recovery.md>) — local retention, temporary cleanup, backups, exports, credentials, diagnostics, and deletion behavior.
- [AI and import](<Product Plans/AI_and_Import.md>) — user-selected AI providers, credentials, request safeguards, import, generation, factuality, and failure behavior.
- [Desktop-extension communication](<Product Plans/Desktop_Extension_Communication.md>) — Chrome/Edge capture, native messaging, local IPC, installation, permissions, and version compatibility.
- [Distribution and updates](<Product Plans/Distribution_and_Updates.md>) — Store/GitHub channels, signing, updates, release integrity, and extension stores.
- [Security, privacy, and open source](<Product Plans/Security_Privacy_and_Open_Source.md>) — threat boundaries, privacy promises, local security, open-source governance, licensing, and distributed documentation requirements.
- [Quality, accessibility, and verification](<Product Plans/Quality_Accessibility_and_Verification.md>) — quality requirements, accessibility, critical journeys, and release evidence.

### Reserved future workspaces

- [Implementation plans](<Implementation Plans/README.md>) — rules and template for future technical planning.
- Component folders under `Implementation Plans/` — placeholders for desktop, extensions, local data, AI/document processing, distribution/updates, and system documentation.
- [Aesthetic planning](Aesthetic/README.md) — placeholder for branding, themes, component visuals, and document-template design.

## Recommended reading routes

### Product review

1. Product scope and principles
2. Core workflows
3. Product states and operations
4. Local data and document model
5. Configuration limits and defaults
6. AI and import
7. Release scope and open decisions

### Implementation-planning handoff

1. Read every product plan.
2. Read the open-decisions register completely.
3. Use the applicable component placeholder and implementation-plan template.
4. Link each technical choice to the product requirements it satisfies.
5. Record irreversible technical choices later as architecture decision records.

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
