# Overlay-controlled Chrome rectangle capture

Implemented September 29, 2026, Chrome package 0.2.1.

The extension has no action, popup, shortcut, or send/on/off UI. ORT's Capture
button authorizes an in-memory UUID session and destination. A persistent native
port polls content-free commands; the isolated-world page tool starts only when
armed. First click anchors the top-left; pointer movement previews the rectangle;
second click completes it. Words inside the current box are highlighted using
CSS Custom Highlights and the same geometry rules as delivery. Highlights update
on expansion, shrinkage, and scrolling without modifying text nodes or changing
the page's own text selection/highlights; cleanup removes only ORT's registry entry
and constructed stylesheets. Capture becomes Cancel after the first corner. The
Answers tab also controls question capture. Job results pass through encrypted
pending intake and automatically replace Stage 1’s editable description and URL
using an atomic revision-checked transaction. The description stays 180px high
with vertical scrolling; the URL is a single-line input. Existing save operations
finish before capture application, preventing an old draft from overwriting the
new capture. Question results still wait for editable review. No AI request starts
on capture. The native capture callback shows/focuses the overlay without
repositioning it.

Cancel revokes desktop intake immediately, before the next Chrome poll removes
the box. Late delivery and stale cancellations cannot affect a new generation.
Old asynchronous status replies cannot restore a cancelled generation in the
overlay; a regression test delays a reply until after Cancel. The extension pins
exact main-frame sender/tab/document/URL identity before
showing the box, so a fast first click cannot race the injection response.
Navigation, same-URL reload, tab/window changes, resize,
connection loss, and expiry end capture safely. Capture clicks are swallowed
through the final click event to avoid activating underlying page controls.

Page/panel content coordinates keep the anchor attached while scrolling. Wheel
input passes through the capture shield to the underlying scrollable element.
Final extraction includes selected beginning/middle paragraphs now off-screen;
preview highlights current visible text. Fixed/sticky page chrome is excluded
from a scrolled region. Preview updates coalesce at twenty frames per second and
have a 120-ms geometry budget; failures clear stale highlights. Final extraction
retains its one-second budget and never sends a partial result.

Rendered text extraction uses DOM Range geometry, clipping/visibility checks,
Unicode normalization, line breaks, and block boundaries. A partial rectangle
captures only the enclosed characters. Form values, editable fields, hidden
content, and outside text are excluded. Limits: 128 KiB text, 4 KiB URL, 256 KiB
frame, 30,000 DOM nodes, and a one-second extraction budget. Captures are never
automatically resent after an uncertain acknowledgement.

HTTP/HTTPS host access replaces activeTab because a desktop click cannot grant
Chrome's temporary permission. The extension does not inject on startup or inspect
page text while idle. Users must approve the changed site access when reloading.
No extension storage, telemetry, external network calls, or content logs are used.

Observed checks on the current implementation:

- Extension: 22 unit/manifest/native-client tests passed, including first-click
  injection timing, exact sender authority, cancellation, malformed/oversized
  content, restricted pages, URL cleanup, no automatic retry, and stable dev ID.
- Chrome 154.0.8037.58: actual native binary, authenticated local socket, and shared
  desktop intake/review functions passed in disposable profiles. Synthetic job
  text with inline formatting, a BR line break, and Unicode delivered exactly;
  question capture/acceptance, partial word selection, cancel, empty rectangle,
  reload, session removal, and suppression of page click handlers passed.
  Whole-page and inner-panel wheel scrolling captured all three beginning/middle/
  end paragraphs, including off-screen text. Live highlights matched full and
  partial text, expanded/shrank, and were removed on Cancel/completion while
  preserving a page-owned highlight. The fixture restricts inline styles with CSP.
- Desktop Vitest: 204 tests across 38 files passed (the runner executed the full
  suite despite the requested file filters). The final cancellation/status race
  regression and focused overlay/settings/URL suite passed 30 tests across 3 files.
- Feature-enabled IPC: 12 tests; native host: 2; desktop capture/disconnect: 5 passed.
  Controls reject unknown fields, arbitrary error text, invalid client UUIDs,
  unsupported phases, and incompatible versions. Private-file, HMAC, replay,
  expiry, bounded framing and interrupted-peer tests remain passing.
- Default IPC: 7 tests; native: 4 passed. Default desktop compilation passed;
  production forwarding remains gated. Feature-enabled Clippy with warnings
  denied passed. Desktop TypeScript/web and ad-hoc macOS app build passed.

The installed app/native host and 0.2.1 ZIP are refreshed for manual testing.
Artifact receipts under `artifacts/extension/chrome` record hashes and browser QA.
This scrolling/highlight fix changes extension source and browser QA only; it
requires no app or native-host protocol update. Only one ORT app is kept in `/Applications`; one previous bundle ZIP is kept under
`artifacts/app-backups`. No user data was copied into test profiles or modified.
Installed GUI focus/review and store-installed testing were not automated. No
store upload/publication or production signing qualification was performed.

Limitations: current rendered DOM/main frame only; image/canvas, PDF, closed
shadow roots and embedded frames are not OCR'd. Open embedded job pages in a tab.
The development capability trusts the current macOS account; it does not establish
signed process identity. See the setup guide for the explicit development boundary.

## Automatic job fields and position preservation

The installed development app now fills and replaces the job description and URL
without confirmation. Success uses the stored authenticated payload rather than
renderer-supplied replacement text. Existing encrypted Stage 1 saves finish before
the atomic revision-checked replacement; an application already in progress,
wrong request ID, or stale revision preserves the existing data and pending capture.
Failures offer an inline retry/discard action. No AI generation starts on capture.

Verification for this change:

- Desktop UI full suite: 208 passed. After the additional undo/autosave regression
  fix, the final overlay suite passed all 33 tests.
- Rust capture tests: 5 passed, including automatic replacement, conflict
  preservation, request-ID checks, and rejection of question/application conflicts.
- Real Chrome 154 → native host → authenticated socket → encrypted desktop store
  passed with automatic job application. Disposable browser profile and store.
- Production frontend under the app CSP at 360×760, with mocked desktop IPC:
  automatic replacement, no dialog, real 180px description scrolling, long URL
  horizontal scrolling, later edits autosaved, and no AI/position commands passed.
  Screenshot: `artifacts/extension/chrome/overlay-auto-capture.png` (visually checked).
- TypeScript, feature-enabled Rust Clippy with warnings denied, web security/secret
  scans, formatting, and the final macOS development bundle build passed.
- Installed bundle verified byte-for-byte against the build and signature checked;
  exactly one ORT app in Applications, with one prior ZIP outside Applications.

Native installed-window dragging/capture focus was not automated. The capture
notification no longer calls the native positioning function; startup/show layout
behavior remains separate. Store publication was not performed.
