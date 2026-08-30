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
7. User-selected remote AI provider credentials stored in the OS credential vault; no ORT-funded key.
8. Direct structured editing and local PDF, DOCX, and text export.
9. Finish Application and a local tracker retaining selected resume, cover-letter, and answer JSON snapshots.
10. Encrypted portable backup/restore and full structured data export.
11. Check-for-updates behavior appropriate to Microsoft Store or signed direct-release channels.

## Initial non-goals

- ORT accounts, authentication, subscriptions, payment, cloud storage, synchronization, hosted AI, admin tools, or behavioral analytics.
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
5. **[IMPLEMENTATION]** Select initial provider adapters and tested cost-conscious and high-quality model presets. Verify current public API availability before documenting a preset.
6. **[IMPLEMENTATION]** Define prompt/schema versioning, provider cancellation, streaming, timeouts, retries, local request recovery, cost estimates, and error taxonomy.
7. **[IMPLEMENTATION]** Specify the native-messaging protocol, IPC authentication, per-user installation, desktop launch, version window, repair, and uninstall behavior.
8. **[IMPLEMENTATION]** Prove how Microsoft Store MSIX first-run setup writes browser-visible per-user native-host registrations and keeps their paths valid after updates.
9. **[IMPLEMENTATION]** Select the Windows GitHub packaging/signing path and prepare a SignPath application after the project meets eligibility requirements.
10. **[IMPLEMENTATION]** Select macOS package format and direct updater. Decide when to begin paid Developer ID signing/notarization versus publishing only an unsigned preview.
11. **[IMPLEMENTATION]** Define channel-specific update metadata, signature verification, release provenance, schema-safe update order, and recovery.
12. **[IMPLEMENTATION]** Define safe local diagnostic bundles without centralized telemetry.
13. **[IMPLEMENTATION]** Freeze protective limits for import size/pages, extracted text, native messages, document pages, collections, backups, provider requests, and exports.

## Validation and release work

1. **[VALIDATION]** Ensure resume creation remains approachable despite flexible structured sections and entries.
2. **[VALIDATION]** Prevent layout overflow, clipping, broken links, unreadable fonts, and poor DOCX behavior across templates and operating systems.
3. **[VALIDATION]** Demonstrate that draft/publish state is understandable and unpublished changes never enter tailoring silently.
4. **[VALIDATION]** Confirm Finish Application prevents accidental loss and survives failed tracker writes.
5. **[VALIDATION]** Test long-lived local tracker performance and historical structured rendering with large representative profiles.
6. **[VALIDATION]** Test backup creation, corruption detection, passphrase handling, full restore, version migration, and low-disk-space recovery.
7. **[VALIDATION]** Evaluate each supported AI preset for factuality, prompt injection, prohibited answers, structural validity, provider cost visibility, and cancellation behavior.
8. **[VALIDATION]** Test Chrome/Edge permissions, profiles, native messaging, IPC impersonation resistance, app launch, update skew, repair, and uninstall across the compatibility matrix.
9. **[VALIDATION]** Verify that provider calls, update checks, diagnostics, and logs match the application's privacy explanations.
10. **[RELEASE]** Finalize the exact `GPL-3.0-only` dependency/asset compatibility review and add repository governance files.
11. **[RELEASE]** Complete security review of imports, local storage, credential vault, extensions, native IPC, updater, release CI, and diagnostic export.
12. **[RELEASE]** Complete accessibility testing and document-export readability checks.
13. **[RELEASE]** Confirm current Store policies, signing requirements, API terms, and platform rules immediately before distribution.

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
- Official releases are traceable to public source, appropriately signed or unmistakably labeled as previews, and distributed through documented channels.
