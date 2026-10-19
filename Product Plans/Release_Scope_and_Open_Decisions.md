# Initial release scope and open decisions

## Status key

- **[IMPLEMENTED]** — present in development code; qualification/acceptance limits are stated separately.
- **[PARTIAL]** — working development behavior with specific remaining work.
- **[IMPLEMENTATION]** — specified work still to implement.
- **[VALIDATION]** — prove with implemented builds and representative tests before stable release.
- **[RELEASE]** — operational, distribution, policy, or external requirement before broad public release.
- **[LATER]** — intentionally outside the initial release; not a current commitment.

This is the central register for implementation gates, unresolved validation/release work, and deferred work in the product plans. The detailed technical selections are authoritative only for implementation mechanics and are indexed in `../Implementation Plans/README.md`.

## Current implementation status — October 15, 2026

The [implementation map](../docs/development/current-state.md) distinguishes
working features from planned distribution. M0–M4 have recorded development
acceptance. M5 remains open for Edge/browser acceptance and the October 19, 2026
approved, unimplemented [application-answer feature](Application_Answer_Autofill.md);
optional M6 Codex is
implemented in development without a full milestone sign-off. M7 production
signing, notarization, final host/Store identities and release/update qualification
remain planned. This documentation refresh does not grant milestone acceptance.

## Current platform qualification scope

- The only active development, native-testing, and preview-qualification target is **macOS on Apple Silicon (`arm64`)**. The exact minimum supported macOS release remains a later distribution decision.
- A milestone described as complete during this phase means **complete for the macOS-arm64 development/preview scope**. It does not claim Windows, Intel Mac, universal-binary, or broad public-release qualification.
- Windows and Intel Mac remain intended platform-expansion goals, but their native adapters, installers, signing, manual matrices, and release evidence are **[LATER]** work and do not add native support claims to macOS-arm64 development acceptance.
- Shared contracts, data formats, domain behavior, and platform abstractions remain portable. Existing Windows and Intel-Mac CI jobs stay enabled as regression signals where practical; a green build is useful portability evidence, not native qualification or a support claim.
- A feature remains disabled on every platform whose security gate has not passed. In particular, document import cannot be enabled merely because shared or CI tests pass.
- During local development, a locally created self-signed Code Signing identity may be used to test stable code requirements and macOS Keychain access across rebuilds. It is not a Developer ID, is not suitable for distribution, and does not satisfy notarization or stable-release gates. Ad-hoc signing remains acceptable only for tests that do not require identity continuity.

## Milestone signing boundary — accepted 2026-10-05, reordered 2026-10-07

M0–M6 develop and qualify functionality on the active macOS Apple Silicon
configuration without requiring paid Apple signing. M5 includes Chrome/Edge
native messaging using a clearly identified current-user development bridge,
exact extension origins, authenticated bounded messages and explicit capture
sessions. It does not claim signed process isolation. Store-ready extension code
and packages can be tested before M7; public Store publication is not an M5 gate.
M7 is the production milestone and requires Developer ID signing of ORT and its
native helpers, notarization/stapling, production IPC identity/Keychain controls,
and final Store identities/publication and installed integration qualification.
Local ad-hoc/self-signed integrity mechanisms and all existing import, provider,
external-runtime, data and authentication protections remain required where
applicable. This boundary does not authorize removing runtime checks or making
unsigned test transport a silent production fallback.

## Initial release capabilities

1. Free open-source macOS Apple Silicon desktop application with one local profile per operating-system user; Windows and Intel Mac are deferred platform expansions.
2. One autosaved master draft and one current published master; earlier immutable publication revisions retained for replay/backup.
3. Manual structured resume creation plus deterministic reviewed local import of text-bearing PDF/DOCX in No AI mode; a configured provider may optionally propose improved structure only after separate transmission confirmation.
4. Three initial ATS-conscious resume style directions with local preview and rendering. The default Technical/Engineering style follows the supplied Jake's Resume reference as closely as compatible licensing permits and remains visually separate from ORT branding.
5. Chrome and Edge deliberate capture through an installed native-messaging host.
6. One locally recoverable overlay-owned application workspace with Stage 1 capture/review and Stage 2 Resume/Cover letter/Answers tabs; no more than three verified change-summary points; non-blocking Required Qualification Alerts; expanded structured editing with PDF preview; resume/cover-letter PDF drag and Download controls; prompted resume regeneration; and resettable question capture/answer drafting.
7. Exactly one active AI mode: a user-supplied OpenAI, Anthropic, or Gemini API key; a user-authorized ChatGPT/Codex subscription connection; or No AI. Direct keys use the OS vault; Codex authorization is memory-only. ORT funds no inference.
8. Explicit tested catalog model IDs (legacy tier names are migration metadata); aggregate local AI Monitoring with Week/Month/Year/All time token and direct-cost graphs/totals plus secondary breakdowns; optional direct-API weekly, monthly, yearly, and all-time estimated-spend caps; and optional Codex provider-quota thresholds.
9. Direct structured editing and local PDF, DOCX, and text export.
10. Finish Application and a local tracker retaining selected resume, cover-letter, and answer JSON snapshots.
11. Encrypted portable backup/restore and full structured data export.
12. Check-for-updates behavior appropriate to unsigned macOS preview limitations and a later Developer ID-signed channel. Windows GitHub/Store update behavior is deferred with Windows qualification.
13. **[IMPLEMENTATION — M5]** Application answers: encrypted reusable question/answer library, Off/Capture only/Autofill + capture modes, foreground native application-field monitoring, reviewed aliases, approval/Undo/stop popup, and portable saved data under the [feature authority](Application_Answer_Autofill.md). No AI matching, implicit resume/tracker import, automatic submission, or durable extension answer store.

## Initial non-goals

- ORT-owned accounts, authentication, subscriptions, payment, cloud storage, synchronization, hosted AI, admin tools, or behavioral analytics. User-authorized provider authentication remains optional and provider-controlled.
- Automatic application submission and form completion outside the narrowly opted-in [M5 answer capability](Application_Answer_Autofill.md). Unrelated/background collection, custom widgets/embedded forms, and broader controls remain outside its v1 scope.
- Job-board notifications, reminders, calendars, employer tooling, or job recommendations.
- OCR, scanned resume support, or bundled local-model inference.
- Multiple master resumes or permanent intermediate-draft libraries.
- Firefox, Safari, Linux, or mobile clients.
- Exact submitted PDF/DOCX archival.

## Implementation specifications and gates

The current selections and module-level acceptance criteria are documented under `../Implementation Plans/`. The statuses below describe development implementation. Release-time review, native qualification, and milestone acceptance remain separate.

1. **[IMPLEMENTED]** Tauri/React/Rust desktop, generated contracts, and isolated macOS-arm64 development identity. Windows/Intel native qualification is **[LATER]**.
2. **[IMPLEMENTED]** SQLCipher schema 9, OS-vault keys, forward migrations, process profile ownership, recovery, and retryable retirement of active/retained credentials. Production identity/vault qualification remains M7.
3. **[IMPLEMENTED]** Authenticated `.ort-backup` 1.9, bounded collection/serialization, legacy readers/vectors, fresh-identity restart restore, safety rollback and deletion. Merge restore/automatic backup are not implemented.
4. **[IMPLEMENTED]** Local PDF/DOCX/text output and reviewed macOS metered-Wasm import with packaged helper identity pins. OCR, other-platform importer qualification, and production packaging remain outside this development claim.
5. **[IMPLEMENTED]** OpenAI/Anthropic/Gemini adapters and explicit model-ID selection from a signed catalog. **[RELEASE]** Reverify official model availability, features and prices before shipping; old research tables are not current guarantees.
6. **[IMPLEMENTED]** Versioned structured generation/refinement, cancellation, bounded retry/recovery, factual validation, Required Qualification Alerts, and safe error categories. Live account-specific quality/availability remains separate.
7. **[IMPLEMENTED]** Operation/attempt accounting, durable lifetime/guardrail records, Data graphs/export/clear, provenance and interrupted-attempt recovery. History clearing does not reset spending exposure.
8. **[IMPLEMENTED]** Per-key/general spend-cap reservation/settlement and explicit restart/disable behavior. Current key controls use all-time caps; historical calendar policies and imported guardrails retain typed storage support.
9. **[IMPLEMENTED]** Optional macOS-arm64 verified Codex runtime/installer, contained stdio, Start/Stop server, memory-only managed login, model/reasoning selection, usage/reserve checks and process cleanup. **[VALIDATION]** Remaining M6 live acceptance is not recorded; there is no credential-file/keyring fallback.
10. **[PARTIAL]** Chrome 0.3.0 capture, exact origins, bounded authenticated development transport and setup/repair helpers are implemented. Edge remains capture-disabled. Remaining browser walkthrough and production identity/Store qualification stay open.
11. **[LATER]** Windows installers, native-host registration, SignPath signing, and Microsoft Store fallback qualification.
12. **[PARTIAL]** Local development app/helper and preview integrity packaging exist. **[RELEASE]** macOS Developer ID signing/notarization, release provenance/SBOM, final host identities, and final Store publication remain M7.
13. **[IMPLEMENTATION]** Authenticated updater metadata, release/channel automation, installed update/repair/uninstall qualification, and safe diagnostic bundles. A checked-in CI workflow is not a production release pipeline.
14. **[PARTIAL]** Protective parser/message/document/request/backup/export limits and focused regressions are implemented. Recheck limits with each changed adapter and production package.
15. **[IMPLEMENTATION]** Complete production About/Legal/provenance and redistribution-notice verification for the actual shipped artifact.
16. **[IMPLEMENTED]** Overlay Stage 1/Stage 2, structured editing/PDF preview, macOS temporary drag cleanup, atomic Finish, tracker snapshots and background paginated search. Cross-platform drag qualification remains deferred.
17. **[IMPLEMENTED]** Open Folio light-only app direction and independent Technical/Professional/Modern document styles. Prior Quiet Navy/Precision Workbench plans are historical. **[RELEASE]** Final shipped asset/license review remains required.
18. **[IMPLEMENTED]** Deterministic reviewed No-AI PDF/DOCX import without provider transmission. Optional AI mapping requires its separate explicit action and valid provider.
19. **[IMPLEMENTATION — M5]** Reusable answer domain/storage/contracts → desktop library → v2 authorization/protocol with v1 capture compatibility → capture-only monitor → autofill/popup → focused Chrome/Edge qualification. Schema 10 and backup 1.10 are planned transitions from the current schema 9/backup 1.9, not implemented formats. M5 stays open; production identities, disclosures, and Store checks remain M7.

## Validation and release work

The [revised quality policy](Quality_Accessibility_and_Verification.md) replaces
the earlier exhaustive validation list for M3 onward. Use the roadmap's short
milestone checks and record known limitations. M0–M2 remain closed.

1. Check the new user journey and its main failure/cancel path with synthetic data.
2. For AI, check representative factual/structured output, alert behavior, provider
   errors, redaction, cost arithmetic and cap rejection with small fixtures.
3. For workspace/browser changes, check capture/review/save/reopen, authentication
   rejection and that capture does not automatically transmit content to AI.
4. For optional Codex, inspect isolation enforcement and check one supported
   runtime plus wrong-identity/version/forbidden-event rejection. Defer it if unsafe.
5. For changed data handling, check normal save/restore and a representative rejected
   write/input using mocks rather than a crash or low-disk campaign.
6. Spot-check keyboard/accessibility and output readability on changed interfaces.
7. For distribution, check the actual channel's install/lifecycle, artifact identity,
   notices, current Store/provider terms and truthful limitations. No broad platform
   matrix or specialist testing campaign is required.
8. Before accepting nontrivial external code contributions, decide whether hosted-
   service risk justifies changing future releases from GPLv3 to AGPLv3. The default
   remains `GPL-3.0-only`; any change requires copyright authority, compatibility
   review, a documented transition and updates to every license/public claim.

The new M5 answer feature additionally needs its own focused matching/approval,
user-edit versus programmatic-event, empty-field/Undo, navigation/revocation,
migration/backup/deletion, and redaction checks, plus one Chrome learning-to-Undo
journey, an Edge smoke check, and accessible desktop/popup controls. Existing
capture acceptance cannot stand in for these [future checks](../Implementation%20Plans/Browser%20Extensions/Application_Answer_Autofill_Plan.md).

## Later possibilities

- **[LATER]** Windows development and qualification, including x64/ARM64 decisions, Credential Manager/AppContainer behavior, installers, signing, browsers, accessibility, and clean-machine evidence.
- **[LATER]** Intel Mac (`x86_64`) and universal-binary development and qualification, including native dependencies, packaging, performance, accessibility, and clean-machine evidence.
- **[LATER]** Firefox, Safari, Linux, and mobile support.
- **[LATER]** OCR and scanned resume import.
- **[LATER]** Downloadable local-model adapters with explicit hardware/license disclosures.
- **[LATER]** Optional encrypted synchronization through a storage provider selected and controlled by the user.
- **[LATER]** Job-board notifications, reminders, calendar integration, and richer application-stage workflows.
- **[LATER]** Additional document templates and exact submitted-file archives.
- **[LATER]** Import/export interoperability with other open resume schemas.
- **[LATER]** Privacy-preserving, explicitly opt-in crash reporting if community support cannot diagnose failures otherwise.

## Stable-release success criteria

- A new user can install the desktop app and extension, create or import a resume, publish it, capture a job, tailor and review it, export it, finish the application, and reopen retained materials without an ORT account.
- The same journey remains usable without AI except for explicitly AI-dependent transformations.
- The app can recover from restart, provider failure, failed tracker save, extension/desktop version skew, and safe update without losing the last valid local state.
- A user can understand where data is stored, what leaves the device, who charges for AI, how to back up, and how to delete everything.
- A user can inspect every provider call made by ORT, distinguish operations from retry attempts, compare token and estimated-cost totals by model/provider, understand ORT-only versus account-wide and reported versus estimated usage, change a tested model for future calls, enforce optional direct or Codex guardrails, export or clear local history without resetting limits, and reach the provider's authoritative usage/billing tools.
- Official releases are traceable to public source, appropriately signed or unmistakably labeled as previews, and distributed through documented channels.
- Modified and third-party distributions can preserve required source attribution without being mistaken for official releases.
