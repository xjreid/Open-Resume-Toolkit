# M2 exact PDF pages in the editor

Date: 2026-09-06. High-reasoning integration on the uncommitted M2 tree based on
qualified M1 `65518eb`. Step 5 remains incomplete.

## Behavior and boundaries

The center pane offers Live reading and editing or Exact PDF pages. One
`PdfPreviewPanel` owns the preview session and mounts its controls into the center
through a React portal. Switching views does not request another render or
replace the receipt. Section navigation and the focused editor remain available.
Semantic preview text corresponds to the captured source, not subsequent edits.

Exact pages reuse the native saved-revision command, validated source/style/
revision response and verified PDF bytes. Unsaved edits remain clearly labeled.
Opt-in automatic refresh waits 650 ms after a valid saved source is eligible,
runs only while Exact PDF pages is selected, and attempts each document/revision/
style once. Failure requires a new identity, explicit retry or toggling refresh.
Published and historical previews turn automatic refresh off. Existing previews
retain their actual style even after the selected style changes.

Late ordinary-render responses are discarded when the source revision, document
identity, style or draft dirty state changes. Their native bytes are released.
Discard pending preview also suppresses display/export and releases the result.
It **does not interrupt native Typst compilation**: the operation remains busy
until rendering returns, as stated in the UI. Unmount releases successful late
responses. No new native cancellation guarantee is claimed.

## Continuous pages

The existing local PDF.js worker renders every page sequentially into a scroll
region, with keyboard-operable page navigation and Fit width, 100%, 150% and
200% zoom. Fit width responds to pane width and clamps scale to two. Neither
zoom nor resizing changes PDF bytes. Loading retains byte count, SHA-256,
PDF-signature and page-count checks and local-worker restrictions.

The validated contract permits at most five pages. Each canvas is capped at two
million pixels and an aggregate ten-million-pixel check bounds the page set.
This allows approximately 40 MB of canvas pixel backing at maximum zoom, plus
PDF.js overhead; it is not a process-memory bound. Each complete page pass has
a ten-second deadline. Readiness requires every page to finish. Failures/timeouts
clear partial canvases; zoom invalidates readiness. Disposal cancels work, zeros
canvases, disconnects the observer and disposes the worker/loading task/bridge.
No PDF script, link, remote resource or annotation authority was added.

## Validation

- Full `CI=true pnpm check` passed: 101 desktop, 24 contract, 30 tooling and two
  extension tests; formatting/lint, builds, security and dependency licenses.
  Log: `target/m2-center-pdf-check.log`.
- Deferred-response regressions cover edits during render, explicit discard,
  native release, no exposed canvas/export and no overlapping ordinary render.
- Editor checks cover one-session view switching and automatic-refresh failure
  suppression/hidden-view suspension without saving or publishing content.
- Mock-engine checks cover sequential pages, later-page failure, oversized
  geometry, cancellation, unmount, resizing and stale zoom completion.
- A separate local browser fixture used real PDF.js with the audited synthetic
  two-page Technical v2 PDF. Both pages reached readiness. Page navigation,
  200% zoom and fit width worked; screenshots were visually inspected beside
  the editor and at 650-pixel viewport width. An initial horizontal-scroll issue
  led to the Fit width default. Fixture files are under
  `target/m2-center-pdf-browser`. Native commands were mocked. The temporary
  browser tab/server were closed and viewport override reset.

No Rust, IPC contract, renderer/template/font, export authority, installed app or
real profile changed. Earlier output-byte audits remain applicable. Hosted CI,
installed WKWebView, VoiceOver, all-style native readers, native dialogs/lifecycle
and import containment remain open. PDF content hit-testing into fields is not
implemented; section navigation and the live reading view supply editing targets.
