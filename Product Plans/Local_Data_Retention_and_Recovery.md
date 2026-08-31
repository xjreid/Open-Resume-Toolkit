# Local data retention, recovery, and deletion

## Purpose

Every local data category has a purpose, location, access rule, cleanup trigger, backup behavior, and export behavior. ORT has no cloud copy and cannot restore data that the user has not backed up.

## Local profile and settings

- Retained on the current device until the user deletes all local ORT data or removes it according to documented uninstall behavior.
- Portable settings may be included in full backup/export.
- Installation identifiers and update preferences are reset when all local data is deleted.
- Direct-provider keys and Codex sessions are references only; secret values remain in the OS credential vault and are excluded from ordinary backup/export.

## Master resume

- The one draft and zero-or-one published snapshot remain until changed or deleted by the user.
- Publishing replaces the published association while preserving the draft.
- Full backup/export contains both structured records and the render manifests needed for supported rendering.
- ORT does not maintain a hidden permanent user-visible version history. Transaction journals and migration safety copies are bounded recovery mechanisms and are cleaned after successful validation.

## Resume import

- The selected original and extracted working data exist only in ORT-controlled temporary processing locations.
- After proposal acceptance, rejection, cancellation, or unrecoverable failure, remove the original working copy and extraction data as soon as the review/recovery flow no longer needs them.
- Completed operation working files have a one-hour maximum cleanup window; abandoned pre-processing files have a 24-hour maximum.
- Accepted structured information becomes part of the master draft; the original document does not become a retained profile file.
- Provider-side copies and retention are governed by the chosen provider, not ORT. Documentation links to provider controls where practical.

## Current application workspace

- Reviewed job text, sanitized URL, temporary tailored versions, cover-letter drafts, question answers, and selections remain in the one local workspace until Finish Application, explicit finish without saving, discard, deliberate replacement, or all-data deletion.
- ORT does not automatically erase canonical workspace content after a fixed inactivity period. It may show the last-active time and offer cleanup, but deletion requires user intent.
- Successful Finish Application retains only the selected structured artifacts and confirmed tracker data; unselected workspace content is removed.
- A failed tracker save preserves the complete workspace.
- Temporary preview/render files are separate from the canonical workspace and follow the short cleanup defaults.

## Tracker and structured snapshots

- Tracker records and selected immutable resume, cover-letter, and question-answer snapshots remain until the user deletes or replaces them.
- Replacing an artifact creates a new validated snapshot and changes the association only after commit; superseded snapshots are deleted after the transaction's recovery window unless the implementation explicitly exposes revision history later.
- Deleting one artifact leaves the tracker entry and other artifacts intact.
- Deleting a tracker entry removes its associated snapshots after confirmation.
- CSV exports tracking fields and artifact-presence metadata. Full backup/export includes structured artifacts and render manifests, not pre-rendered binaries by default.

## Exports and external files

- PDF, DOCX, plain-text, CSV, backup, and full-data files saved outside ORT-controlled directories remain entirely under user control.
- Finish Application, tracker deletion, all-local-data deletion, and uninstall do not delete those external files.
- ORT should avoid persistent hidden export copies. When a temporary file is needed for preview, drag-and-drop, or handoff, its exact lifetime and cleanup status are visible or documented.

## Provider credentials and Codex session

- API keys remain in the current operating-system user's credential vault until replaced, disconnected, all ORT credentials are removed, or the OS removes them.
- A Codex managed ChatGPT session remains in an ORT-specific configuration/authentication root and OS credential namespace until the user signs out, removes all ORT credentials, or the OS removes it. ORT does not copy or mutate a general Codex client's auth/configuration files.
- Keys and Codex tokens are never included in backup, full-data export, diagnostics, extension storage, or ordinary logs.
- The application provides a credential removal action and verifies deletion where the platform allows.
- Uninstall behavior is tested and documented. If an installer cannot reliably remove a credential-vault entry, the user receives exact manual cleanup instructions.

## AI activity history

- AI activity records are durable, content-free user data and remain locally available until the user clears them, enables an age-based retention policy, or deletes all local ORT data.
- The default is to retain activity history without automatic age-based deletion so totals do not disappear unexpectedly. The interface offers 30-day, 90-day, one-year, and retain-until-cleared choices and states the active policy.
- A user can clear one record, a filtered selection, a date range, or all activity history without deleting generated content, tracker records, credentials, or provider-side records.
- Removing or replacing an API key does not remove historical activity records.
- Signing out of Codex does not remove historical ORT activity or last-known non-secret quota/token snapshots; the user can clear those through their own data controls.
- Encrypted backup and full structured export include AI activity records by default. Filtered CSV/JSON export contains only the visible non-content fields and never includes keys, prompts, responses, resume/job content, or full URLs.
- Ordinary diagnostic bundles exclude the full activity ledger by default. If the user deliberately includes relevant activity metadata, the preview names the included fields and applies the same content/secret exclusions.

## AI guardrail state

- Direct spend counters, active reservations, unresolved outcomes, period boundaries, and Codex quota-threshold mappings remain until their period resets, the user changes/resets the policy explicitly, the credential/session identity is removed, or all local data is deleted.
- Activity-history clearing and age-based retention never reset or reduce guardrail state. The UI states this before clearing history.
- Calendar period detail may be compacted after reset, but enough non-content state remains to prevent duplicate resets and explain the current counter. All-time state remains until its separately confirmed baseline reset.
- Backups may include policy configuration and historical counters, but restored policies remain inactive until the user binds them to a newly authenticated credential/session. Secrets are never included and counters never silently transfer to another identity.
- Full-data export identifies guardrail records separately from AI Activity. Ordinary activity CSV/JSON does not include security-sensitive internal reservation state.

## Browser-extension content

- Captured text and URL remain only long enough to review, transmit, retry safely, or allow the user to copy after a connection error.
- Clear extension capture data after desktop acknowledgement or cancellation and during extension uninstall/reset.
- Extension preferences contain no resume, tracker, provider-key, or generated content.

## Diagnostics

- Ordinary centralized telemetry is not part of the initial product.
- Content-free local logs, when needed, rotate by time and size and are retained no longer than the configuration default.
- A diagnostic bundle is generated only on user request, shows included categories before export, redacts paths/usernames where feasible, and excludes document content, full URLs, credentials, and provider request bodies by default.
- The user controls any diagnostic file after saving or sharing it.

## Backups

- ORT backups are user-created or written on an explicitly configured schedule to a user-selected directory.
- Backups are encrypted and integrity-protected. The user-supplied passphrase is not uploaded or recoverable by maintainers.
- API keys and Codex sessions are excluded.
- Automatic rotation policy is selected and disclosed during implementation; ORT never deletes backups outside its configured backup target.
- A destination synchronized by iCloud, OneDrive, Dropbox, or another service causes that provider to receive the encrypted archive. This is user-controlled external synchronization, not ORT cloud storage.
- Restore validates the archive before mutation and creates or offers a safety copy of current data.

## Local deletion and uninstall

The settings area supports:

- Discard current application workspace
- Delete individual tracker artifacts or entries
- Remove provider credentials
- Clear selected or all AI activity history
- Disable or reset AI guardrail policies/counters through a separate confirmation
- Delete master-resume content
- Delete all local ORT data

Destructive actions identify what will and will not be deleted, mention external exports/backups, and require confirmation. There is no remote recovery window; recovery is possible only from an existing user-controlled backup or a still-valid local safety copy disclosed by the interface.

Uninstall documentation must state separately whether it removes application binaries, native-host registration, settings/database, credentials, and user-selected exports/backups. The safest mainstream default is to avoid silently destroying the only local profile merely because the application binary is uninstalled, while still providing a complete removal path.
