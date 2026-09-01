# Requirement traceability

## Use

These IDs provide stable anchors for code issues, tests, and release evidence. They summarize—without replacing—the named authoritative product sections. A behavior change still requires the product plan to change first.

| ID | Requirement | Product authority | Implementation owner | Minimum evidence |
|---|---|---|---|---|
| ORT-SYS-001 | Core editing, tracking, rendering, and export work without network or an ORT account | Product scope / Local ownership | Architecture, desktop | intercepted-network offline journey |
| ORT-SYS-002 | Exactly one active AI mode: No AI, Direct, or Codex | AI and import / AI operating model | Desktop, AI | connection transition tests |
| ORT-SYS-003 | Remote telemetry and hosted user-content services are absent | Security / Telemetry and diagnostics | Architecture, website | binary/site network audit |
| ORT-DATA-001 | Canonical local records use versioned structured schemas | Local data / Canonical records | Local data | schema drift and round-trip tests |
| ORT-DATA-002 | At most one master draft, published master, and current workspace exist per profile | Configuration / Fixed product values | Local data | database constraint tests |
| ORT-DATA-003 | Sensitive local records are encrypted at rest and keys remain in the OS vault | Security / Local data exposure | Local data, vault | database/WAL plaintext and vault tests |
| ORT-DATA-004 | Writes, Finish Application, and guardrail reservations are atomic and crash recoverable | Product states / operation lifecycle | Local data | fault-injection transaction tests |
| ORT-DATA-005 | Portable backup is encrypted, authenticated, versioned, cross-device, and excludes secrets | Retention / Backups | Local data | cross-platform restore and hostile archive suite |
| ORT-DATA-006 | Activity deletion, cap reset, workspace deletion, and full profile deletion remain distinct | Retention / deletion sections | Local data, desktop | record-scope deletion tests |
| ORT-RES-001 | Draft edits are autosaved but tailoring reads only the published snapshot | Core workflows / Draft and publish | Desktop, local data | critical draft/publish journey |
| ORT-RES-002 | Import performs local extraction and requires confirmation before AI mapping/transmission | AI and import / Import flow | Documents, AI, desktop | import privacy and review tests |
| ORT-RES-003 | Preview and PDF export use the same renderer tuple and structured source | Core workflows / Rendering and export | Documents | golden render/receipt comparison |
| ORT-RES-004 | PDF, DOCX, and text are derived exports; structured JSON remains canonical | Local data / Structured documents | Documents, local data | round-trip/authority tests |
| ORT-RES-005 | Document templates are visually independent of ORT branding; Technical defaults to the licensed/independently implemented Jake's Resume structure | Resume editor / Styles | Documents, aesthetic | license receipt and golden render comparison |
| ORT-RES-006 | Overlay resume and cover-letter PDF cards provide both Download and temporary OS file drag-out, with safe cleanup and a rejected-drop fallback | Core workflows / Rendering | Desktop, documents, local data | cross-platform drag/download/cleanup matrix |
| ORT-AI-001 | Direct OpenAI, Anthropic, and Gemini calls use user-owned vault credentials | AI and import / Direct credentials | AI, vault | adapter/vault contract tests |
| ORT-AI-002 | Models/presets are tested, cataloged, and never silently rerouted | AI and import / models | AI, distribution | model resolution/failure tests |
| ORT-AI-003 | Every provider call is recorded before dispatch with operation/attempt separation | Product states / AI operation | AI, local data | crash/retry ledger tests |
| ORT-AI-004 | Direct token/cost accounting exposes completeness/provenance and never shows unknown cost as zero | AI and import / accounting | AI, desktop | pricing/usage fixture reconciliation |
| ORT-AI-007 | AI Monitoring presents Week/Month/Year/All time token/direct-cost graphs and totals, while attempt rows remain internal | Core workflows / AI Monitoring | Desktop, AI, local data | aggregate-query, accessibility, and no-call-list UI tests |
| ORT-AI-005 | Direct spending caps reserve before dispatch and fail closed when unevaluable | AI and import / spending guardrails | AI, local data | concurrency/boundary/crash tests |
| ORT-AI-006 | AI output cannot introduce unsupported resume facts | AI and import / factual boundaries | AI, documents | adversarial factuality corpus |
| ORT-ALT-001 | Tailoring returns change summary and alerts in the same logical/provider call | Core workflows / tailoring | AI, local data | operation/attempt count and schema tests |
| ORT-ALT-002 | Alerts only show Confirmed mismatch or Not found for explicit mandatory resume-comparable requirements | Core workflows / alerts | AI | classification/evidence corpus |
| ORT-ALT-003 | Alerts are dismissible/ignorable/reopenable and never block or auto-edit | Core workflows / alerts | Desktop, local data | interaction/accessibility tests |
| ORT-CODEX-001 | Codex is separately installed, verified, and app-managed over stdio | AI and import / Codex | AI, distribution | absence/version/lifecycle matrix |
| ORT-CODEX-002 | Codex uses isolated config/auth/keyring and does not inherit normal user Codex state | Configuration / Codex defaults | AI, platform | seeded-config isolation tests |
| ORT-CODEX-003 | Codex cannot use tools/files/commands and has provider-only egress | Security / Codex containment | AI, security | per-platform containment report |
| ORT-CODEX-004 | Codex quota data retains provider/account-wide provenance; no API-equivalent cost is shown | AI and import / Codex usage | AI, desktop | account snapshot/provenance tests |
| ORT-IPC-001 | Extension captures selected text only after an explicit user action; the overlay owns review and Continue/Generate decisions | Desktop-extension / Capture | Extension, desktop | passive-browsing and overlay-review tests |
| ORT-IPC-002 | Native messaging and desktop IPC are versioned, bounded, origin-checked, authenticated, and replay resistant | Desktop-extension / IPC | Extension, IPC | hostile protocol suite |
| ORT-IPC-003 | Install, launch, repair, update, and uninstall preserve valid native-host registration | Desktop-extension / Installation | IPC, distribution | clean-machine channel matrix |
| ORT-APP-001 | The main window owns master/admin surfaces and has no job-specific route; the overlay owns both application stages and all tailoring/material interactions | Core workflows / Overlay application workflow | Desktop | route/capability and full overlay journey tests |
| ORT-APP-002 | Stage 2 has Resume/Cover letter/Answers tabs, no more than three change points, prompted regeneration, resettable question capture, and persistent Finish Application | Core workflows / Stage 2 | Desktop, AI | state-machine and accessibility tests |
| ORT-TRK-001 | Finish Application atomically saves selected structured snapshots and clears temporary workspace data | Core workflows / Finish Application | Desktop, local data | failure-injection journey |
| ORT-DIST-001 | Preferred Windows stable is SignPath-signed GitHub direct; Store is fallback; unsigned Windows is preview | Distribution / Windows | Distribution, website | signature/channel tests |
| ORT-DIST-002 | Initial unsigned macOS GitHub downloads are previews until Developer ID/notarization | Distribution / macOS | Distribution, website | artifact/claim verification |
| ORT-DIST-003 | Direct/Store/preview update channels cannot overwrite one another | Distribution / Update behavior | Distribution | crossover tests |
| ORT-DIST-004 | Releases include checksums, SBOM, provenance, compatibility, and truthful signing status | Distribution / Release docs | Distribution | release evidence bundle |
| ORT-ACC-001 | Critical journeys support keyboard, screen reader, scaling, high contrast, and reduced motion | Quality / Accessibility | Desktop, website | automated and manual platform matrix |
| ORT-WEB-001 | Website is static public information only and never accepts resume/application content | Private website boundary | Website | route/network/form audit |
| ORT-WEB-002 | Download pages derive from verified canonical release metadata and enforce channel warnings | Website content / Download | Website, distribution | production manifest/link smoke test |

## Maintenance rule

Every implementation pull request lists the IDs it satisfies or affects. Release evidence indexes these IDs. When a product requirement is split or materially changed, add a new ID and deprecate the old one with a pointer; do not silently reuse an ID for different behavior.
