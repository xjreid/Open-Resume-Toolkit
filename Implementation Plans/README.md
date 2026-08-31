# Technical implementation planning — reserved

## Status

This workspace is intentionally reserved for the later technical implementation-planning phase. Current files define organization and required content only. They do not yet select frameworks, libraries, schemas, SDKs/endpoint versions, packaging tools, or code organization beyond the provider and Codex product boundaries approved in the authoritative plans.

## Authority boundary

Implementation plans explain how approved product requirements will be built. They may not independently introduce accounts, cloud content storage, subscriptions, hosted AI keys, telemetry, new data collection, feature gates, or a different lifecycle. If implementation work requires a product-rule change, update the authoritative product plans first.

## Required inputs

Before writing a component plan, read:

1. `../Plan_Index.md`
2. Every file in `../Product Plans/`
3. The affected component placeholder in this folder
4. The open-decisions register completely

## Planned locations

- `Desktop Application/` — shell, windows, overlay, editor, tracker, AI Activity, direct/Codex connection and guardrail settings, accessibility, credential vault, and local orchestration.
- `Browser Extensions/` — Chrome/Edge extensions, native host, IPC, installation, repair, permissions, and compatibility.
- `Local Data and Migration/` — schema, transactions, encryption, AI activity/guardrail state, Codex usage cache, migrations, backup, restore, deletion, and portability.
- `AI and Document Processing/` — direct provider adapters, Codex app-server, prompts, model catalogs, usage/cost/quota accounting, guardrail enforcement, validation, import extraction, rendering, exports, and evaluation.
- `Distribution and Updates/` — packaging, Stores, signing, notarization, GitHub releases, provenance, updates, and rollback.
- `System Documentation/` — architecture decisions, interfaces, build/development setup, threat model, release runbooks, and troubleshooting.

## Required implementation-plan template

Every technical plan should include:

1. Status, owner, and milestone.
2. Purpose and explicit non-goals.
3. Authoritative product requirements and stable requirement identifiers.
4. Dependencies and affected components.
5. Concrete modules, records, interfaces, states, permissions, and error contracts.
6. Security, privacy, accessibility, licensing, and resource controls.
7. Compatibility, migration, rollout, rollback, repair, and recovery behavior.
8. Unit, contract, integration, end-to-end, failure, migration, and security tests.
9. Objective completion criteria and evidence locations.
10. Remaining technical questions that do not silently redefine product behavior.

## Planning rules

- Prefer one canonical contract and cross-reference it rather than copying it.
- Version every durable schema and cross-component protocol.
- Keep content data, AI activity, guardrail counters, provider account snapshots, secrets, diagnostics, and derived exports in their approved boundaries.
- Treat Store, direct-download, development, and preview builds as explicit channels.
- Record major irreversible choices as architecture decision records under `System Documentation/`.
- A provider or library name in a technical plan is not permanent product policy unless the authoritative product plan makes it one.
