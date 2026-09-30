# Opt-in unsigned macOS Chrome development bridge

User-authorized development exception, 2026-09-29. Default production builds
remain gated. No Apple enrollment, Developer ID identity or notarization was
used. The prepared `.app` is ad-hoc signed for local execution.

Implemented feature-gated private socket transport, temporary per-session key,
mutual HMAC, exact extension allowlist, nonce replay rejection, expiry and frame
bounds. Session files are private and cleaned on disconnect/quit. No database
or provider key is exported. Programs under the current macOS user can read the
development capability; this is not signed peer authentication.

The desktop destination remains the existing encrypted pending review store.
Captures do not replace pending work or start AI requests. Successful intake
notifies and focuses the overlay through a native event. Browser settings expose
explicit enable/disable controls only in a feature-enabled dev identity.

Observed automated verification:

- Chrome 154.0.8037.58 in a disposable profile, real native binary and actual
  desktop encrypted intake/review functions: overlay-authorized rectangles delivered Unicode text and clean
  link/title; job/question accepted; partial text discarded; cancellation revoked
  late results; empty boxes and same-URL reloads failed safely; no capture clicks
  reached page controls; disconnect ended the session;
  session removed; no extension storage or runtime exceptions.
- IPC tests: valid roundtrip, live socket ownership, private directory/file
  checks, symlink rejection, tampering, domain separation, session mismatch,
  expiry, oversized frames and replay rejection. Live invalid-packet and incomplete-
  peer disconnect checks pass. The macOS fractional timeout regression was fixed
  using a single nonblocking read deadline; the tests and real browser run then
  passed again.
- UI tests cover explicit enable/disable, refresh, absent feature and setup errors.
  Existing overlay regressions and URL cleanup tests remain passing.
- A disconnect regression verifies that the bridge mutex is released before
  joining an intake callback, avoiding a storage/status lock cycle.
- Feature-enabled Rust Clippy and desktop TypeScript compilation pass.

Final command output and generated receipts under `artifacts/extension/chrome`
record the current package/browser results. Build artifacts are ignored, not
source-controlled. No capture from a real user's job page was used in QA.

The automated browser walkthrough tests real desktop intake and acceptance, not
GUI focus behavior. Manual app walkthrough, dashboard-ID synchronization, Google
review and store-installed testing remain user steps in
`packaging/extension/chrome/development-testing.md`. The exact-ID development host was registered for this account; connection remains
opt-in. No upload or publication was performed. This evidence does not qualify signed production IPC or public release.
