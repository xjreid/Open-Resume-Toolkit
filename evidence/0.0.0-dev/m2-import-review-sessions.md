# Step 5 native review-session lifecycle

Date: 2026-09-07. Uncommitted development source on M1-complete `65518eb`.

`ort-application::import_session::ReviewSessions` owns one active review, its
retained extraction, decisions, native owner identity and independent token.
Neither identity can be deserialized from renderer input. An active review
cannot be silently replaced. Foreign owners and old tokens cannot read, edit,
commit or cancel the current review. Native owner teardown drops its review.

Reviews have a fixed 30-minute lifetime that reads and edits do not extend.
Every access checks expiry. The explicit expiry method is also intended for a
native cleanup timer; no timer or automatic idle cleanup is claimed yet.
Dropping source text is not secure erasure of allocator copies.

Commit prepares the authoritative review against a freshly loaded complete
saved draft and calls one trusted storage callback. That callback must use the
payload's expected revision in the existing storage CAS transaction. Mutable
ownership serializes review operations while the synchronous callback runs.
Validation failures never call storage. Storage failures retain the review for
inspection. Confirmed success retires it; a success receipt with unexpected
document/revision also retires it because storage might already have committed.

Five regressions passed, covering owner/token isolation, cancellation, exact
expiry, teardown, pending/stale decisions, storage failure, confirmed-save
replay, unexpected receipts, real encrypted-storage edit races and an error
returned after a successful transaction. The last case confirms fresh draft
validation prevents applying the same import again. Encrypted tests use only
temporary synthetic profiles and an in-memory vault; no Keychain was accessed.

The full workspace Rust suite passed before the final additional ambiguous-save
regression; the final focused suite passed all five cases. Strict workspace
Clippy passed after that addition. The full `CI=true pnpm check` passed with
108 desktop tests and the unchanged dependency inventory. Logs:
`target/m2-import-session-workspace-tests.log`,
`target/m2-import-session-tests.log`, `target/m2-import-session-clippy.log`,
and `target/m2-import-session-web-check.log`. `git diff --check` passed.

This is application-layer lifecycle implementation, not a registered native
command or a new authority granted to the renderer. The desktop adapter still
needs native window ownership, serialized blocking dispatch, timer/teardown
wiring, bounded IPC contracts, UI mapping, storage callback integration and
import audit summaries. The parser containment gate remains unresolved; no
production caller starts review from a file and no parser was enabled. No app
build was installed. Step 6 qualification has not begun.
