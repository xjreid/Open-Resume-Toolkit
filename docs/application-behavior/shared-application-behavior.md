# Shared application behavior

## Main workspace navigation

The desktop navigation contains Master resume, AI & monitoring, Application
tracker, and Settings. Master resume has Edit, View, and Import subsections.
The right-side Overlay icon shows or hides the separate application window.
Switching destinations does not discard the owning workspace's
in-memory state. Navigation is temporarily disabled while a blocking native
operation or confirmation is active.
The top-left brand always reads **Open Resume Toolkit**, with Open on the first
line and Resume Toolkit on the second; it does not change with the destination.

The storage-health badge reflects typed native health state. When storage is
unavailable, the badge retries opening it, including the operating-system
keychain prompt. Features that require encrypted storage remain disabled or
fail closed until it is ready.

## Local storage and external files

The active profile is stored in an encrypted SQLCipher database. Its database
key and provider API secrets live in the operating-system credential vault.
The application is local-first: resume content, settings, AI accounting,
guardrails, diagnostics, and render receipts remain local unless the user
explicitly exports or invokes an external AI provider.

External PDF, DOCX, JSON, text, and portable-backup files are not part of the
active encrypted profile. Ordinary exports are unencrypted. Portable backups
are encrypted with the user-provided passphrase.

## Resume import

Import accepts supported text-based PDF or DOCX files through a native Open
dialog. Scanned documents require OCR and are not supported.

Extracted content is staged into a review session. The user must review and
accept or reject every proposed change and choose its destination before the
saved draft changes. Cancelling review applies nothing. A concurrent revision
change prevents stale review content from silently overwriting the draft.

## Backup and recovery

- Backup writes a passphrase-protected portable archive through a native Save
  dialog. ORT cannot recover a forgotten passphrase.
- Validation authenticates and inventories a selected backup before restore.
- Restore stages a fresh encrypted replacement profile and activates it safely
  on restart rather than merging it into the current profile.
- The former profile can be retained as an encrypted local safety copy.
  Rollback and permanent safety-copy deletion are explicit actions.
- External exports and unrelated backup files are not changed by profile
  restore or local profile deletion.

## Storage inventory and delete-all behavior

Settings displays a content-free inventory of known encrypted profile records
and files. Counts exclude external exports/backups, operating-system vault
items, and in-memory preview bytes.

**Delete all local ORT data** requires the exact confirmation phrase. It
deletes the active encrypted profile, draft, publications, settings, render
history, diagnostics, local recovery data, pending restore state, provider
credentials, and profile keys. It also discards unsaved editor state. It does
not uninstall the application or delete external exports and backups.

If committed cleanup is incomplete, the application reports that state and
requires restart/recovery rather than pretending deletion fully finished.

## Common mutation and failure rules

- Destructive operations require confirmation proportional to their scope.
- A successful response updates the visible local state; a failed or uncertain
  response must not be displayed as success.
- Operations that may overlap storage or AI accounting use busy gates. The UI
  asks the user to finish or cancel the active operation rather than allowing a
  partial competing mutation.
- Provider secrets, backup passphrases, and resume content must not appear in
  diagnostic errors or debug output.
- Native file pickers own path selection. The UI does not accept arbitrary
  output paths from web content.
- Failure states remain actionable: reload when storage state is unknown,
  retry explicit cleanup when cleanup failed, or preserve edits when a save was
  not confirmed.
