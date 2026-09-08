# M2 final native acceptance (Step 6)

The implementation candidate is complete; these are final human/native acceptance
checks, not missing feature implementation. M0/M1 remain successful. Use only the
`orttest` login and supplied synthetic resumes. Do not use real employment data.
The installed application has not been replaced.

## Open the exact candidate

Quit ORT in both accounts. Use Fast User Switching to log into `orttest`; Codex
does not need to be installed there. Open the newly supplied Shared transfer
folder in Finder and double-click `Open M2 Test.command`. The launcher refuses
other accounts. If Keychain requests authorization, report whether you selected
Allow/Always Allow and whether it repeats after quit/reopen. Never share passwords.

## Import and review

1. Confirm encrypted storage is ready. If an earlier synthetic draft exists,
   keep it; import also supports an existing saved draft.
2. Select Import an existing resume. Cancel the picker. Confirm no draft revision
   or resume content changes.
3. Import `Synthetic resume.docx` from the transfer folder. Confirm original text
   and pending decisions appear; nothing has been saved or accepted automatically.
   Cancel review. Confirm the prior draft/empty profile is unchanged.
4. Import the DOCX again. Accept the intended contact and section/text proposals,
   choose explicit destinations for text, and reject any unwanted blocks. Edit
   one proposed value so it is visibly distinguishable. Apply after every block
   has a decision. Confirm exactly one draft revision is added.
5. Quit/reopen. Confirm the reviewed result persists. Import the supplied PDF;
   compare its original text and reject/cancel without changing the saved draft.
6. During preparation, exercise Cancel import. If the small fixture completes
   first, cancel its resulting review. Confirm the editor becomes usable again.

## Offline editor and output

- Edit/reorder/duplicate/remove an entry, undo, add an identified date/link and
  resolve a validation error using its navigation link. Confirm keyboard focus.
- Select all three document styles and inspect exact PDF preview. Export PDF,
  DOCX and text to new filenames. Open them in the native readers available on
  the test account; inspect text order, dates, links, clipping and pagination.
- Publish, change the draft, and reopen the retained published source. Confirm
  it remains immutable and current-renderer regeneration is labeled truthfully.
- Create/check an encrypted portable backup. Follow the existing recovery matrix
  for restore, interrupted save, locked Keychain, deletion and low-disk cases;
  automated unit checks do not substitute for those native fault observations.
- Exercise VoiceOver and keyboard navigation through import, review, validation,
  preview, native dialogs and quit. Confirm labels, focus and cancellation.

Record pass/fail for each group, exact failure text without private data, and
whether the Keychain prompt repeats. Step 6 and the full M2 milestone remain
unaccepted until this matrix is completed; no automated result claims these
human observations.
