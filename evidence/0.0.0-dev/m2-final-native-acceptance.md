# M2 final native acceptance (Step 6)

> **Closed:** M2 is accepted for macOS Apple Silicon development. This file is
> historical evidence/procedure. The [closure record](m2-acceptance-closure.md)
> supersedes pending-status and further-test instructions below.


The implementation candidate is complete; these are final human/native acceptance
checks, not missing feature implementation. M0/M1 remain successful. Use only the
`orttest` login and supplied synthetic resumes. Do not use real employment data.
The installed application has not been replaced.

## Final acceptance scope — user decision, 2026-09-09

The user waived the remaining manual matrix after reviewing round three and
requested only a fix for the app-menu Quit failure plus a quick real-account
verification. The [final quick handoff](m2-quit-final-handoff.md) supersedes older
instructions requiring another full matrix or 31-minute wait. Round three passed
expired-review cancellation, Command-Q with an unapplied review, reopening saved
state, rollback and UI deletion checks. Archive selection differed from the
recorded backup identity, so the replacement observation remains inconclusive.
Skipped checks remain unrun. M2 scoped acceptance is pending only the final focused
quit/persistence result; this is not a claim that unrun tests passed or new CI ran.

## Latest status — 2026-09-09

Rounds one and two are recorded in the supplied native reports; the earlier first
observations below are historical. Round two passed all-style multipage PDF/DOCX,
date persistence and sampled accessibility checks, but found stale-review
cancellation/quit failure and an unresolved overlay-exit observation.
See [round-three preparation](m2-round3-preparation.md) for the repairs, new signed
candidate, exact helper qualification, native recovery/APFS evidence and limits.
Use only the new transfer's [round-three plan](m2-round3-orttest-handoff.md) for the
next session. Neither this update nor previous CI marks M2 accepted.

## Acceptance observations (in progress)

The implementation is committed at `295e3b2`; the user reports all CI tests
passed for that commit. This supersedes the earlier uncommitted/no-hosted-result
handoff status. Hosted run details have not been independently retrieved in this
acceptance session. M0/M1 remain complete; M2 signoff remains pending.

Candidate: `/Users/Shared/ORT-M2-acceptance-jov1muwi`, launched through
`Open M2 Test.command` in the real `orttest` login. Developer-session hash checks
confirmed both original and Shared desktop/helper executables match the identities
in `target/m2-candidate/manifest.json`. No new signature verification is claimed.

First test group, user-observed:

- Startup passed: **Encrypted storage ready** appeared and an existing saved
  synthetic resume was present. The user recalled its title approximately as
  “M2 synthetic resume”; the exact title and numeric revision were not recorded.
- Native import picker cancellation passed for visible preservation: the prior
  saved resume and its title remained after canceling. Exact revision equality
  has not yet been established by recorded before/after numbers.
- One Keychain prompt appeared at initial startup. The user entered the password
  locally and selected **Allow**, explicitly not **Always Allow**. No further
  prompt was reported during this test group. Quit/reopen prompt behavior is
  still pending. Prompt text/item identity and its cause are not established;
  no password or certificate/private-key transfer was requested or recorded.

DOCX review cancellation/application, restart persistence, PDF, editor/output,
recovery/fault and accessibility checks below remain pending.

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
