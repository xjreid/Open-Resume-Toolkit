# Initial release scope and open decisions

## Status key

- **[IMPLEMENTATION]** — specified in the current technical plans; implementation and evidence are still required.
- **[VALIDATION]** — prove with implemented builds and representative tests before stable release.
- **[RELEASE]** — operational, distribution, policy, or external requirement before broad public release.
- **[LATER]** — intentionally outside the initial release; not a current commitment.

This is the central register for implementation gates, unresolved validation/release work, and deferred work in the product plans. The detailed technical selections are authoritative only for implementation mechanics and are indexed in `../Implementation Plans/README.md`.

## Current platform qualification scope (2026-09-04)

- The only active development, native-testing, and preview-qualification target for M0, M1, and M2 is **macOS on Apple Silicon (`arm64`)**. The exact minimum supported macOS release remains a later distribution decision.
- A milestone described as complete during this phase means **complete for the macOS-arm64 development/preview scope**. It does not claim Windows, Intel Mac, universal-binary, or broad public-release qualification.
- Windows and Intel Mac remain intended platform-expansion goals, but their native adapters, installers, signing, manual matrices, and release evidence are **[LATER]** work and do not block M0-M2 macOS-arm64 completion.
- Shared contracts, data formats, domain behavior, and platform abstractions remain portable. Existing Windows and Intel-Mac CI jobs stay enabled as regression signals where practical; a green build is useful portability evidence, not native qualification or a support claim.
- A feature remains disabled on every platform whose security gate has not passed. In particular, document import cannot be enabled merely because shared or CI tests pass.
- During local development, a locally created self-signed Code Signing identity may be used to test stable code requirements and macOS Keychain access across rebuilds. It is not a Developer ID, is not suitable for distribution, and does not satisfy notarization or stable-release gates. Ad-hoc signing remains acceptable only for tests that do not require identity continuity.

## Initial release capabilities

1. Free open-source macOS Apple Silicon desktop application with one local profile per operating-system user; Windows and Intel Mac are deferred platform expansions.
2. One autosaved master-resume draft and one deliberately published master resume.
3. Manual structured resume creation plus deterministic reviewed local import of text-bearing PDF/DOCX in No AI mode; a configured provider may optionally propose improved structure only after separate transmission confirmation.
4. Three initial ATS-conscious resume style directions with local preview and rendering. The default Technical/Engineering style follows the supplied Jake's Resume reference as closely as compatible licensing permits and remains visually separate from ORT branding.
5. Chrome and Edge deliberate capture through an installed native-messaging host.
6. One locally recoverable overlay-owned application workspace with Stage 1 capture/review and Stage 2 Resume/Cover letter/Answers tabs; no more than three verified change-summary points; non-blocking Required Qualification Alerts; expanded structured editing with PDF preview; resume/cover-letter PDF drag and Download controls; prompted resume regeneration; and resettable question capture/answer drafting.
7. Exactly one active AI mode: a user-supplied OpenAI, Anthropic, or Gemini API key; a user-authorized ChatGPT/Codex subscription connection; or No AI. Secrets use the OS credential vault and ORT funds no inference.
8. Curated, adaptable Economy/Balanced/Quality model choices; aggregate local AI Monitoring with Week/Month/Year/All time token and direct-cost graphs/totals plus secondary breakdowns; optional direct-API weekly, monthly, yearly, and all-time estimated-spend caps; and optional Codex provider-quota thresholds.
9. Direct structured editing and local PDF, DOCX, and text export.
10. Finish Application and a local tracker retaining selected resume, cover-letter, and answer JSON snapshots.
11. Encrypted portable backup/restore and full structured data export.
12. Check-for-updates behavior appropriate to unsigned macOS preview limitations and a later Developer ID-signed channel. Windows GitHub/Store update behavior is deferred with Windows qualification.

## Initial non-goals

- ORT-owned accounts, authentication, subscriptions, payment, cloud storage, synchronization, hosted AI, admin tools, or behavioral analytics. User-authorized provider authentication remains optional and provider-controlled.
- Automatic application submission or form completion.
- Job-board notifications, reminders, calendars, employer tooling, or job recommendations.
- OCR, scanned resume support, or bundled local-model inference.
- Multiple master resumes or permanent intermediate-draft libraries.
- Firefox, Safari, Linux, or mobile clients.
- Exact submitted PDF/DOCX archival.

## Implementation specifications and gates

The current selections and module-level acceptance criteria are documented under `../Implementation Plans/`. Items remain marked **[IMPLEMENTATION]** until working code and the specified evidence exist; they are no longer unanswered architecture placeholders.

1. **[IMPLEMENTATION]** Implement the selected desktop framework on macOS arm64 while preserving explicit adapters for later Windows and Intel-Mac expansion.
2. **[IMPLEMENTATION]** Define the versioned local schema, transaction model, migrations, crash recovery, exact content-at-rest encryption/key-vault design, macOS trusted-application/access-control behavior, and desktop/native-host secret-sharing proof. The specified Windows same-user boundary remains a later platform gate.
3. **[IMPLEMENTATION]** Define encrypted `.ort-backup` format, RFC 9106 Argon2id writer profile and bounded reader policy, authenticated encryption, canonical header parsing, integrity, restore/merge behavior, and forgotten-passphrase limitations.
4. **[IMPLEMENTATION]** Select local PDF/DOCX extraction and rendering libraries; implement mandatory per-import process isolation; and confirm licenses, font embedding, selectable text, links, tagging, deterministic layout, worker resource ceilings, crash behavior, and sandbox evidence.
5. **[IMPLEMENTATION]** Implement direct adapters for OpenAI, Anthropic, and Gemini and freeze the tested Economy/Balanced/Quality presets from the dated baseline in `AI_and_Import.md`. Verify model-list access, exact identifiers, lifecycle status, structured output, token categories, official prices, and account availability immediately before each release.
6. **[IMPLEMENTATION]** Define prompt/schema versioning, provider cancellation, streaming, timeouts, retries, local request recovery, error taxonomy, requested-versus-effective model handling, and explicit no-fallback behavior.
   - Include a bounded Required Qualification Alert response schema in the existing tailoring operation; define mandatory-versus-preferred classification, supported resume mappings, resolvable evidence references, deterministic/local validation, dismissal state, and false-positive handling without a second provider call.
7. **[IMPLEMENTATION]** Define the standardized provider-call accounting boundary, versioned AI activity schema, provider/model aggregation, raw-usage normalization, signed pricing-catalog format/channel/signing/rollback process, official-source verification and expiry rules, cost-estimate rules, export, retention, deletion, and interrupted-attempt recovery.
8. **[IMPLEMENTATION]** Specify direct-API guardrail transactions: credential identities, period/time-zone semantics, preflight token counting and maximum reservations, price/currency handling, settlement, unknown outcomes, notifications, activity-deletion separation, cap changes, and all-time baseline resets.
9. **[IMPLEMENTATION]** Specify the external Codex app-server integration: strict official-runtime identity/provenance verification with no arbitrary-executable override, supported minimum/maximum runtime and protocol versions, generated schemas and capability negotiation, app-managed `stdio` child-process launch/supervision/termination, ORT-specific config/auth root and keyring namespace, managed ChatGPT and device-code login, required client/service identification or registration, externally contained no-tool execution profile, experimental capabilities disabled, request/event allowlists, dynamic model intersection, thread-token/account-usage/quota normalization, bucket-threshold caps, orphan recovery, cleanup, safe disablement, and sign-out. The initial ORT package does not bundle or silently install/update Codex.
10. **[IMPLEMENTATION]** Specify the native-messaging protocol, IPC authentication, per-user installation, desktop launch, version window, repair, and uninstall behavior.
11. **[LATER]** Prove the preferred direct Windows installer, per-user native-host registration, repair, signed update handoff, and uninstall behavior; separately prove how fallback Microsoft Store MSIX first-run setup writes browser-visible registrations and keeps paths valid after updates.
12. **[LATER]** Define the Windows GitHub packaging/signing path, reproducible source-to-binary CI, checksums, SBOM/provenance, public code-signing policy, release roles, and SignPath application readiness. Define the Microsoft Store MSIX contingency if SignPath is declined or unavailable.
13. **[IMPLEMENTATION]** Select the unsigned macOS preview package format, checksum/provenance and authenticated update-notification design, Gatekeeper guidance, and native-messaging installation behavior. Define measurable adoption, support-burden, and funding triggers for later Developer ID signing/notarization and stable status.
14. **[IMPLEMENTATION]** Define channel-specific update metadata, signature verification, release provenance, schema-safe update order, and recovery.
15. **[IMPLEMENTATION]** Define safe local diagnostic bundles without centralized telemetry.
16. **[IMPLEMENTATION]** Freeze protective limits for import size/pages, extracted text, native messages, document pages, collections, backups, provider requests, and exports.
17. **[IMPLEMENTATION]** Implement a user-visible About/Legal view containing GPL, copyright, canonical-source attribution, third-party notices, trademark-policy links, build provenance, and official/preview status without exposing secrets or local identifiers.
18. **[IMPLEMENTATION]** Implement the main-window/overlay capability split, Stage 1/Stage 2 state machine and tabs, expanded structured editors with PDF preview, safe Download and OS drag materialization/cleanup, required regeneration instruction, and aggregate Week/Month/Year/All time AI Monitoring queries.
19. **[IMPLEMENTATION]** Record the Quiet Navy/Open Frame light-only tokens/assets and verify the Jake's Resume source/license or independent template implementation before shipping any related asset or template code.
20. **[IMPLEMENTATION]** Implement deterministic No-AI import mapping into existing fields and versioned custom/simple sections, preserving all unrecognized text for explicit review without a network request.

## Validation and release work

1. **[VALIDATION]** Ensure resume creation remains approachable despite flexible structured sections and entries.
2. **[VALIDATION]** Prevent layout overflow, clipping, broken links, unreadable fonts, and poor DOCX behavior across templates on the active macOS-arm64 matrix. Later platforms repeat this gate before qualification.
3. **[VALIDATION]** Demonstrate that draft/publish state is understandable and unpublished changes never enter tailoring silently.
4. **[VALIDATION]** Confirm Finish Application prevents accidental loss and survives failed tracker writes.
5. **[VALIDATION]** Test long-lived local tracker performance and historical structured rendering with large representative profiles.
6. **[VALIDATION]** Test backup creation, corruption detection, passphrase handling, full restore, version migration, and low-disk-space recovery.
7. **[VALIDATION]** Evaluate every supported direct and Codex model preset for factuality, prompt injection, prohibited answers, structural validity, required-qualification alert precision/recall and evidence validity, requested-versus-effective model behavior, provider cost/quota visibility, and cancellation behavior.
8. **[VALIDATION]** Reconcile representative successful, failed, retried, cancelled, timed-out, streamed, cached/reasoning-token, missing-usage, rerouted, and ambiguous responses against model/provider totals and verified pricing fixtures; prove that unavailable cost is never displayed as zero or as an invoice.
9. **[VALIDATION]** Prove direct-API caps under boundaries, concurrent dispatch, retries, crashes, time-zone/clock changes, price changes, missing usage, activity clearing, credential replacement, and all-time reset. No request may dispatch without a committed reservation when a cap is active.
10. **[VALIDATION]** Prove external Codex-runtime absence, official identity/provenance and counterfeit-runtime rejection, compatible/incompatible version handling, app-managed `stdio` launch and termination, browser/device login, isolated configuration/authentication, model discovery, exact request/event allowlists, experimental-capability disablement, fail-closed command/process/filesystem/tool/approval/permission/elicitation events, token/quota provenance, delayed/rounded quota updates, provider bucket changes, cap blocking, sign-out, protocol mismatch, orphan recovery, and cleanup on every supported OS/package channel.
11. **[VALIDATION]** Test Chrome/Edge permissions, profiles, native messaging, IPC impersonation resistance, app launch, update skew, repair, and uninstall across the active macOS-arm64 compatibility matrix; repeat for each later platform before support is claimed.
12. **[VALIDATION]** Verify that provider calls, internal accounting, aggregate AI Monitoring, guardrail state, Codex account snapshots, update checks, diagnostics, and logs match the application's privacy explanations.
13. **[RELEASE]** Obtain qualified review and finalize the exact `GPL-3.0-only` dependency/asset compatibility, Section 7 attribution term, contributor inbound-license treatment, SPDX approach, Store/signing-program eligibility, and trademark policy; add the remaining repository governance files.
14. **[RELEASE]** Complete security review of imports, local storage, direct and Codex credential handling, app-server containment, guardrail transactions, extensions, native IPC, updater, release CI, and diagnostic export.
15. **[RELEASE]** Complete accessibility testing and document-export readability checks.
16. **[RELEASE]** Confirm current Store policies, signing requirements, API/Codex integration terms, provider model availability and pricing, and platform rules immediately before distribution.
17. **[RELEASE]** Before accepting nontrivial external code contributions, decide whether hosted-service risk justifies changing future releases from GPLv3 to AGPLv3. The default remains `GPL-3.0-only`; any change requires copyright authority, compatibility review, a documented transition, and updates to every license and public claim.
18. **[VALIDATION]** Prove the complete overlay journey, resume/cover-letter PDF drag and Download behavior/fallback/cleanup, main-window route exclusion, single light visual scheme, document-brand separation, and aggregate monitoring accessibility on macOS arm64 during the current phase.
19. **[VALIDATION]** Prove the hostile-document worker denies user/application files, vaults, database, native IPC, network, and child processes; enforces resource ceilings; kills its full process tree; and cannot mutate canonical data after malformed, crashing, timed-out, or adversarial PDF/DOCX input.
20. **[VALIDATION]** Prove database/provider/IPC vault namespace separation, macOS development-identity and preview desktop/native-host behavior, moved/update/repair flows, cross-account denial, and absence of any plaintext fallback. Windows boundary proof is deferred with Windows qualification.

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
