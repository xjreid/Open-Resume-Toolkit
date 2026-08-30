# Product states and local operation lifecycle

## Purpose and authority

This file defines canonical user-visible record states and recoverable local operations. Implementation details may add internal substates but may not bypass the transitions, confirmation, validation, or recovery rules below.

## Master-resume state

- Each local profile has exactly one mutable draft and zero or one published snapshot.
- Draft edits autosave locally and never change the published snapshot implicitly.
- Publishing validates the current draft, creates an immutable snapshot of its accepted revision, and replaces the previous published association atomically.
- A failed publish preserves both the prior published snapshot and current draft.
- Tailoring records the published revision it used.
- Deleting or editing draft content does not affect the published snapshot until a later successful publish.

## Application-workspace states

One local profile has zero or one current application workspace:

1. **Empty**
   - Ready for a deliberate job-description capture or manual workspace start.

2. **Capture under review**
   - Contains proposed job text and an optional sanitized URL.
   - No AI request begins until the user accepts the content.

3. **Active**
   - Contains one reviewed job context and may contain temporary tailored-resume versions, cover-letter drafts, application-question drafts, user edits, and exports-in-progress.
   - Another job cannot be merged or substituted without explicit finish, discard, or replace confirmation.

4. **Final resume selected**
   - One reviewed tailored resume is selected for export or optional tracker retention.
   - A reviewed cover letter and approved ordered answer set may also be selected.
   - The user can continue editing, change selections, or return to Active.

5. **Finishing**
   - The user has selected Finish Application and reviewed retained tracking fields, selected artifacts, and the deletion warning.
   - A tracker save is recoverable and must finish before temporary content is deleted.

6. **Resetting**
   - The selected tracker entry and artifacts committed successfully, or the user deliberately chose finish without saving.
   - Unselected temporary content and ORT-owned preview files are removed.

7. **Empty**
   - Reset completes and a new application can begin.

A failed tracker save returns to Final resume selected or Active with all workspace content intact. Explicit discard may transition from any non-finishing state to a confirmed destructive reset.

## Resume-import states

1. **File selected** — local file has not been parsed or transmitted.
2. **Locally validating** — type, signature, size, page, compression, and parser-safety checks run.
3. **Extracted for review** — usable text and structure hints are available locally.
4. **Awaiting provider confirmation** — the user reviews provider/model, extracted content scope, and expected provider charges.
5. **Provider operation active** — the selected provider is mapping extracted content.
6. **Proposal under review** — structured content, confidence indicators, uncertainties, and duplicates are presented beside the source.
7. **Accepted** — selected proposal data merges into the draft atomically; publishing is still separate.
8. **Rejected, cancelled, or failed** — the draft remains unchanged and temporary import content enters cleanup.

The original document is not retained as a permanent profile record.

## AI-operation lifecycle

An AI-assisted import, tailoring, refinement, cover letter, or application answer uses one logical local operation with one or more explicitly permitted attempts:

- **Prepared** — inputs have been assembled locally but not transmitted.
- **Awaiting confirmation** — provider/model, transmitted content, and possible charges are visible when confirmation is required.
- **Running** — the desktop owns the current attempt.
- **Waiting on provider** — a remote response or stream is outstanding.
- **Validating** — returned content is undergoing schema, size, factual, prohibited-answer, and renderer checks.
- **Succeeded** — a reviewable validated result is stored in the current workflow.
- **Failed retryable** — a transport, throttling, or transient provider error permits an explicit retry.
- **Failed final** — validation, authentication, policy, permanent-provider, or safety failure prevents automatic retry.
- **Cancellation requested** — ORT has stopped accepting new result data and requests provider cancellation where supported.
- **Cancelled** — no result is promoted; provider billing may still have occurred.
- **Expired** — the operation exceeded its total local lifetime and working content enters cleanup.

Transitions are monotonic for one attempt. Retrying creates a new attempt under the same logical operation. A late response cannot replace a cancelled, expired, superseded, failed-final, or already accepted result.

## Operation security and metadata

- Each operation has an opaque local identifier, type, provider/model preset version, prompt/schema version, attempt count, coarse state/error, timestamps, and user-visible recovery action.
- Resume, job, answer, key, and full-URL content never enters ordinary operation logs.
- The provider key is read from the OS vault only when the confirmed attempt begins and remains memory-only for the call.
- Reopening the overlay reconnects to the current local operation instead of creating a duplicate request.
- User-visible errors include a stable local reference that can be included in a scrubbed diagnostic bundle.

## Retry and cancellation rules

- Authentication failure, rejected/exhausted key, malformed output, factual-validation failure, prohibited content, unsupported input, or permanent provider error is not retried automatically.
- A remote operation gets no more than one automatic retry, and only when the provider contract and implementation can do so without duplicating a non-idempotent request unexpectedly. Otherwise retry requires user confirmation.
- Retrying never silently switches provider, model, key, prompt preset, or transmitted-content scope.
- ORT must explain that cancellation cannot guarantee provider-side cancellation or prevent charges after dispatch.

## Tracker-save operation

- Tracker metadata and all selected structured snapshots are validated before commit.
- The save is atomic when the local store permits. If multiple files or stages are required, the implementation uses a journal/transaction and recoverable commit marker.
- Partial success is never shown as complete. On restart, ORT either completes the same save idempotently or restores the prior valid workspace and tracker state.
- Only after commit may Finish Application clear unselected workspace content.

## Backup, restore, migration, and update states

Longer local work is reconnectable and visibly follows prepared, running, validating, succeeded, recoverable failure, or final failure states.

- Backup success requires a completely written archive, authentication tag/checksum, and final atomic rename.
- Restore validates before replacing canonical data and creates or offers a safety copy first.
- Schema migration records its source/target versions and never marks success until integrity checks pass.
- An update that requires migration verifies recovery material before replacing the running version.
- Interrupted work is detected at next launch and offers the correct resume, rollback, retry, or support action without guessing success.

