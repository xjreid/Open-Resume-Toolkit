# Step 5 bounded import decision boundary

Date: 2026-09-07. Uncommitted development source on M1-complete `65518eb`.

The domain now defines a strict decision-only payload and generates its JSON
schema and TypeScript types. It contains no source extraction, path, owner,
revision or storage authority. Section and text destinations are distinct tagged
variants; contradictory destination fields and unknown fields are rejected.
Pending/rejected decisions and new-section targets use empty struct variants so
Serde rejects extra fields even when the variant has no data.

Native byte decoding caps input at 512 KiB before deserialization, then enforces
1,000 choices, 100,000 total Unicode scalar values, allowed controls and bounded
proposed-section indices. Runtime validation remains authoritative; schema/type
generation alone does not establish these aggregate limits or session ownership.
The existing native review character ceiling now references the domain constant.

`ImportReview::replace_choices` validates the entire batch and exact retained
source-block count before replacing decisions. Failed batches preserve all old
decisions. A pending variant explicitly resets its slot. Native preparation
still checks destinations, contact conflicts, document validity and the exact
saved draft. `ReviewSessions::replace_choices` adds existing owner/token/expiry
checks before allowing the batch update.

The review panel now converts local editing state into the generated payload
when submitting. Original source and explanations remain local display values.
The shared synthetic `fixtures/documents/import-review-choices.json` is consumed
by both the frontend serializer test and the native review preparation test.
It verifies a new heading, a bullet attached to that heading and a contact edit.
Adversarial tests cover unknown authority fields, smuggled fields on empty
variants, ambiguous targets, negative/out-of-range indices, disallowed controls,
byte/count/character ceilings, count mismatch, atomic preservation and explicit
pending resets. Existing session tests now also check foreign batch rejection.

Verification passed: full workspace Rust tests (223 passed, 9 opt-in native
tests ignored), strict workspace/all-target/all-feature Clippy, and the full
`CI=true pnpm check` with 109 desktop tests and 24 contract tests. Dependency
inventory remains 728 Rust / 167 JavaScript packages. Logs:
`target/m2-import-choices-workspace-tests.log`,
`target/m2-import-choices-clippy.log`, and
`target/m2-import-choices-web-check.log`. `git diff --check` passed.

This remains a payload boundary, not a registered command or enabled file-import
route. Native request metadata, window/session dispatch, proposal responses,
expiry timer, close/deletion cleanup and the actual storage callback wiring
remain open. Parser containment is still unresolved; production import stays
disabled. No real profile, Keychain, document or installed application was used.
