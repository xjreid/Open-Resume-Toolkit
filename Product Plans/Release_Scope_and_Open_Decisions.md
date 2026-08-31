# Initial release scope and open decisions

## Status key

- **[IMPLEMENTATION]** — select or specify while writing technical implementation plans.
- **[VALIDATION]** — prove with implemented builds and representative tests before stable release.
- **[RELEASE]** — operational, distribution, policy, or external requirement before broad public release.
- **[LATER]** — intentionally outside the initial release; not a current commitment.

This is the only central register for unresolved and deferred work in the product plans.

## Initial release capabilities

1. Free open-source Windows and macOS desktop application with one local profile per operating-system user.
2. One autosaved master-resume draft and one deliberately published master resume.
3. Manual structured resume creation plus reviewed AI-assisted import of text-bearing PDF/DOCX when a user provider is configured.
4. Three initial ATS-conscious resume style directions with local preview and rendering.
5. Chrome and Edge deliberate capture through an installed native-messaging host.
6. One locally recoverable application workspace with resume tailoring, approximately three verified change-summary bullets, cover-letter drafting, and permitted application-answer drafting.
7. Exactly one active AI mode: a user-supplied OpenAI, Anthropic, or Gemini API key; a user-authorized ChatGPT/Codex subscription connection; or No AI. Secrets use the OS credential vault and ORT funds no inference.
8. Curated, adaptable Economy/Balanced/Quality model choices; local AI Activity with per-model and per-provider token/cost totals; optional direct-API weekly, monthly, yearly, and all-time estimated-spend caps; and optional Codex provider-quota thresholds.
9. Direct structured editing and local PDF, DOCX, and text export.
10. Finish Application and a local tracker retaining selected resume, cover-letter, and answer JSON snapshots.
11. Encrypted portable backup/restore and full structured data export.
12. Check-for-updates behavior appropriate to signed direct GitHub releases, the fallback Microsoft Store channel, and unsigned macOS preview limitations.

## Initial non-goals

- ORT-owned accounts, authentication, subscriptions, payment, cloud storage, synchronization, hosted AI, admin tools, or behavioral analytics. User-authorized provider authentication remains optional and provider-controlled.
- Automatic application submission or form completion.
- Job-board notifications, reminders, calendars, employer tooling, or job recommendations.
- OCR, scanned resume support, or bundled local-model inference.
- Multiple master resumes or permanent intermediate-draft libraries.
- Firefox, Safari, Linux, or mobile clients.
- Exact submitted PDF/DOCX archival.

## Implementation decisions

1. **[IMPLEMENTATION]** Select the desktop framework and supported Windows/macOS architectures while preserving overlay, renderer, secure storage, native messaging, accessibility, signing, and updater requirements.
2. **[IMPLEMENTATION]** Define the versioned local schema, transaction model, migrations, crash recovery, and exact content-at-rest encryption/key-vault design.
3. **[IMPLEMENTATION]** Define encrypted `.ort-backup` format, cryptography, integrity, restore/merge behavior, and forgotten-passphrase limitations.
4. **[IMPLEMENTATION]** Select local PDF/DOCX extraction and rendering libraries and confirm licenses, font embedding, selectable text, links, tagging, and deterministic layout behavior.
5. **[IMPLEMENTATION]** Implement direct adapters for OpenAI, Anthropic, and Gemini and freeze the tested Economy/Balanced/Quality presets from the dated baseline in `AI_and_Import.md`. Verify model-list access, exact identifiers, lifecycle status, structured output, token categories, official prices, and account availability immediately before each release.
6. **[IMPLEMENTATION]** Define prompt/schema versioning, provider cancellation, streaming, timeouts, retries, local request recovery, error taxonomy, requested-versus-effective model handling, and explicit no-fallback behavior.
7. **[IMPLEMENTATION]** Define the standardized provider-call accounting boundary, versioned AI activity schema, provider/model aggregation, raw-usage normalization, signed pricing-catalog format/channel/signing/rollback process, official-source verification and expiry rules, cost-estimate rules, export, retention, deletion, and interrupted-attempt recovery.
8. **[IMPLEMENTATION]** Specify direct-API guardrail transactions: credential identities, period/time-zone semantics, preflight token counting and maximum reservations, price/currency handling, settlement, unknown outcomes, notifications, activity-deletion separation, cap changes, and all-time baseline resets.
9. **[IMPLEMENTATION]** Specify the external Codex app-server integration: executable discovery and verification, supported minimum/maximum runtime and protocol versions, generated schemas and capability negotiation, app-managed `stdio` child-process launch/supervision/termination, ORT-specific config/auth root and keyring namespace, managed ChatGPT and device-code login, required client/service identification or registration, externally contained no-tool execution profile, dynamic model intersection, thread-token/account-usage/quota normalization, bucket-threshold caps, orphan recovery, cleanup, safe disablement, and sign-out. The initial ORT package does not bundle or silently install/update Codex.
10. **[IMPLEMENTATION]** Specify the native-messaging protocol, IPC authentication, per-user installation, desktop launch, version window, repair, and uninstall behavior.
11. **[IMPLEMENTATION]** Prove the preferred direct Windows installer, per-user native-host registration, repair, signed update handoff, and uninstall behavior; separately prove how fallback Microsoft Store MSIX first-run setup writes browser-visible registrations and keeps paths valid after updates.
12. **[IMPLEMENTATION]** Define the Windows GitHub packaging/signing path, reproducible source-to-binary CI, checksums, SBOM/provenance, public code-signing policy, release roles, and SignPath application readiness. Define the Microsoft Store MSIX contingency if SignPath is declined or unavailable.
13. **[IMPLEMENTATION]** Select the unsigned macOS preview package format, checksum/provenance and authenticated update-notification design, Gatekeeper guidance, and native-messaging installation behavior. Define measurable adoption, support-burden, and funding triggers for later Developer ID signing/notarization and stable status.
14. **[IMPLEMENTATION]** Define channel-specific update metadata, signature verification, release provenance, schema-safe update order, and recovery.
15. **[IMPLEMENTATION]** Define safe local diagnostic bundles without centralized telemetry.
16. **[IMPLEMENTATION]** Freeze protective limits for import size/pages, extracted text, native messages, document pages, collections, backups, provider requests, and exports.
17. **[IMPLEMENTATION]** Implement a user-visible About/Legal view containing GPL, copyright, canonical-source attribution, third-party notices, trademark-policy links, build provenance, and official/preview status without exposing secrets or local identifiers.

## Validation and release work

1. **[VALIDATION]** Ensure resume creation remains approachable despite flexible structured sections and entries.
2. **[VALIDATION]** Prevent layout overflow, clipping, broken links, unreadable fonts, and poor DOCX behavior across templates and operating systems.
3. **[VALIDATION]** Demonstrate that draft/publish state is understandable and unpublished changes never enter tailoring silently.
4. **[VALIDATION]** Confirm Finish Application prevents accidental loss and survives failed tracker writes.
5. **[VALIDATION]** Test long-lived local tracker performance and historical structured rendering with large representative profiles.
6. **[VALIDATION]** Test backup creation, corruption detection, passphrase handling, full restore, version migration, and low-disk-space recovery.
7. **[VALIDATION]** Evaluate every supported direct and Codex model preset for factuality, prompt injection, prohibited answers, structural validity, requested-versus-effective model behavior, provider cost/quota visibility, and cancellation behavior.
8. **[VALIDATION]** Reconcile representative successful, failed, retried, cancelled, timed-out, streamed, cached/reasoning-token, missing-usage, rerouted, and ambiguous responses against model/provider totals and verified pricing fixtures; prove that unavailable cost is never displayed as zero or as an invoice.
9. **[VALIDATION]** Prove direct-API caps under boundaries, concurrent dispatch, retries, crashes, time-zone/clock changes, price changes, missing usage, activity clearing, credential replacement, and all-time reset. No request may dispatch without a committed reservation when a cap is active.
10. **[VALIDATION]** Prove external Codex-runtime absence, discovery, compatible/incompatible version handling, app-managed `stdio` launch and termination, browser/device login, isolated configuration/authentication, model discovery, no-tool containment, token/quota provenance, delayed/rounded quota updates, provider bucket changes, cap blocking, sign-out, protocol mismatch, orphan recovery, and cleanup on every supported OS/package channel.
11. **[VALIDATION]** Test Chrome/Edge permissions, profiles, native messaging, IPC impersonation resistance, app launch, update skew, repair, and uninstall across the compatibility matrix.
12. **[VALIDATION]** Verify that provider calls, AI Activity, guardrail state, Codex account snapshots, update checks, diagnostics, and logs match the application's privacy explanations.
13. **[RELEASE]** Obtain qualified review and finalize the exact `GPL-3.0-only` dependency/asset compatibility, Section 7 attribution term, contributor inbound-license treatment, SPDX approach, Store/signing-program eligibility, and trademark policy; add the remaining repository governance files.
14. **[RELEASE]** Complete security review of imports, local storage, direct and Codex credential handling, app-server containment, guardrail transactions, extensions, native IPC, updater, release CI, and diagnostic export.
15. **[RELEASE]** Complete accessibility testing and document-export readability checks.
16. **[RELEASE]** Confirm current Store policies, signing requirements, API/Codex integration terms, provider model availability and pricing, and platform rules immediately before distribution.
17. **[RELEASE]** Before accepting nontrivial external code contributions, decide whether hosted-service risk justifies changing future releases from GPLv3 to AGPLv3. The default remains `GPL-3.0-only`; any change requires copyright authority, compatibility review, a documented transition, and updates to every license and public claim.

## Later possibilities

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
