# Step 5 gated native review dispatch and cleanup

Date: 2026-09-07. Uncommitted source on M1-complete `65518eb`.

The desktop now links the existing application layer and registers apply/cancel
review handlers. Both require the native main-window label and immediately
return `IMPORT_DISABLED` while the shared parser gate is false. No picker,
parser launch or production review creation was added. The handlers accept no
owner, source extraction, arbitrary path or proposed saved document.

New generated request schemas/types validate command metadata, a UUIDv7 review
identifier and the 512 KiB encoded decision-payload ceiling. Decisions remain a
bounded JSON string so the existing decoder can check bytes before decoding
its nested collection. The outer Tauri JSON allocation is not covered by that
inner limit; total IPC message limiting remains an integration requirement.
Token parsing recovers only an opaque review identity. It cannot construct or
substitute the separately retained native owner.

Apply and cancel use the existing exclusive-operation lease that also excludes
exports, restore/deletion and final quit approval. They dispatch through a
blocking worker. Apply holds the desktop storage lock, checks retained review
ownership, loads the current saved draft, validates/replaces all decisions,
prepares against that draft, and invokes the real SQLCipher revision-CAS save.
It returns the actual saved document/revision only after confirmed success.
Failures use fixed error codes; no source text or parser diagnostics are logged.

Each desktop state owns one review manager. Main-window destruction and app exit
clear it. Taking storage for deletion clears reviews first; restore and rollback
staging also clear them before attempting profile replacement. A failed restore
may therefore require restarting import review. A weak-reference expiry worker
checks every five seconds without keeping the desktop state alive. The fixed
30-minute deadline is checked on access; idle reclamation is subject to the
worker interval and scheduler/lock delays. This is not secure memory erasure.

Lock order for apply/restore is storage then review. Teardown releases the review
lock before acquiring storage; the expiry worker never touches storage. The
lease excludes quit during a commit, and invalid/stale tokens cannot affect a
new review. A new main-window creation route would need a new native owner;
currently no such route or production review-begin route exists.

## Validation

Synthetic native tests cover actual apply/save/replay, malformed decisions,
source removal during storage teardown, stale and expired review refusal, and
all-window refusal while import is disabled. Tests use a temporary encrypted
profile with an in-memory vault. Application tests also round-trip a visible
token and prove it does not authorize another owner.

The full workspace Rust suite and `CI=true pnpm check` passed before the final
stale/expiry and token tests. The final desktop and session suites and strict
workspace Clippy are recorded in `target/m2-import-dispatch-desktop-tests.log`,
`target/m2-import-dispatch-session-tests.log`, and
`target/m2-import-dispatch-clippy.log`. Full-suite logs are
`target/m2-import-dispatch-workspace-tests.log` and
`target/m2-import-dispatch-web-check.log`. No new third-party dependency was
introduced; the desktop adds the existing first-party application crate.

## Remaining work

The proposal-response contract, production begin/read route, frontend command
wiring and audit summary are unfinished. Native parser launch, hard resource
bounds, parent-death/tree containment and final qualification remain blockers.
These handlers cannot qualify containment or enable the import UI. No installed
build or test account was changed. Step 6 has not begun.
