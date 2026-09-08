# Step 5 native proposal snapshots and review flow

Date: 2026-09-07. Uncommitted development source on M1-complete `65518eb`.

Native review snapshots now project the retained extraction, exact base draft,
contacts, existing section IDs, mapping version, fixed mapping explanations and
suggested destinations. A snapshot never accepts a decision or substitutes
renderer-provided source text. Section headings do not target themselves; text
can suggest the earlier heading proposal. The generated response schema and
TypeScript interface describe the same fields.

The main-window-only read handler checks the shared disabled-import gate,
request metadata, token, native owner and expiry on a blocking worker. Typed
frontend read/apply/cancel clients validate replies and never automatically
retry an ambiguous operation. The proposal validator rejects extra fields,
invalid identifiers, unsupported mapping versions, invalid Unicode/controls,
invalid page/destination indices and count/character limits.

`ImportReviewFlow` connects those clients to the panel for a supplied native
review ID. It rejects mismatched returned IDs, discards late load results,
excludes duplicate in-flight operations, and suppresses callbacks after unmount.
An unconfirmed save disables resubmission while cancellation stays available.
The panel starts all suggestions pending, including suggested section targets.
No flow is mounted in the app until a production native import-begin route can
safely create a review; that route and parser containment are still unfinished.

Verification: full workspace Rust suite and strict Clippy passed. The full
`CI=true pnpm check` passed with 111 desktop tests, 26 contract tests and the
unchanged 728 Rust / 167 JavaScript dependency inventory. New cases cover native
snapshot nonmutation, malformed/oversized frontend snapshots, client no-retry
behavior and the live uncertain-save/cancel flow. Logs:
`target/m2-import-snapshot-workspace-tests.log`,
`target/m2-import-snapshot-clippy.log`,
`target/m2-import-snapshot-web-check.log`. `git diff --check` passed.

No real profile, Keychain, document or installed build was used. Browser visual
and native WKWebView checks remain pending. Outer Tauri message allocation is
not bounded by the nested decision decoder. Audit summary persistence, native
picker/parser creation, hard resource and whole-tree containment, product-route
integration and Step 6 qualification are not completed by this checkpoint.
