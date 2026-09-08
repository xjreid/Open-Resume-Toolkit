# Step 5 import review presentation

Date: 2026-09-07. Uncommitted development source on M1-complete `65518eb`.

`ImportReviewPanel` provides a separate, in-memory review surface. Its local
TypeScript model is presentation state, not an IPC contract or storage payload.
The panel is deliberately not mounted in the production application. The start
screen's import action remains disabled and no native command is added.

Each extracted block retains its original escaped text, optional page number
and mapping explanation. Every decision starts pending. Users can edit a
suggested value, classify it as contact information, section heading, entry text
or bullet, reject it, or return it to pending. Text can target an existing
section, a kept section proposal, or an explicitly named new section. Rejected
section proposals invalidate dependent destinations rather than silently moving
content. Existing contact information requires an explicit replacement or
keep-existing choice. Section merges retain the existing native review semantics;
the future adapter must map choices to that authoritative model.

Apply remains disabled for pending decisions, invalid destinations, empty kept
values, no kept blocks, the review character ceiling, a changed saved revision
or a busy caller. A different session ID resets local choices. Submission passes
copied choices to a callback; it does not save or report success. Cancellation
also delegates to its caller. Native ownership, authoritative validation,
concurrency checks and optimistic-revision storage remain integration work.

Six new tests cover explicit decisions, destination selection, contact conflict
choices, stale and busy submission refusal, session reset, cancellation, source
text escaping/preservation, removed proposal destinations and axe accessibility.
The full `CI=true pnpm check` passed with 108 desktop tests, 24 contract tests,
formatting, TypeScript, builds, security checks and the unchanged 728 Rust /
167 JavaScript license inventory. Log: `target/m2-import-review-ui-check.log`.
`git diff --check` passed. No Rust code changed in this checkpoint.

This is a tested presentation foundation, not a completed import journey.
Native picker/parser wiring, native review lifecycle and commit integration,
browser visual review, installed WKWebView/VoiceOver checks and the final Step 6
matrix remain pending. No real document, account, vault or installed app was used.
