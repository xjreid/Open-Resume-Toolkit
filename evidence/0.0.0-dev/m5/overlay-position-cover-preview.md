# Overlay position and cover letter PDF preview

The overlay stores its physical position in encrypted settings on hide and on an
approved app quit. Reopening uses its current position; after an app restart it
loads the stored position. Placement uses the nearest available monitor, its scale
and work area, and clamps the whole rail into view if a display is removed or its
size changes. A new profile retains the original left-side default placement.
Layout storage failure does not block closing or storage recovery.

After cover letter generation, the bottom controls are **View** and **Edit**.
View opens a read-only document preview styled like the resume preview, with only
the title, close button and PDF pages. Edit opens the existing current-text editor
with its sequenced change and autosave flow. Both use the same native popup, so
only one is visible at a time. View flushes pending edits and prepares the latest
saved PDF before switching modes. Accessible text remains available to screen
readers without extra visible UI. Loading/errors appear only when necessary.

The preview command returns the exact cached PDF bytes and render receipt used by
Download for that workspace revision and material. It accepts only overlay/popup
windows, verifies the current saved revision under the store lock, and refuses
stale or unprepared exports. The bundled PDF.js worker verifies the PDF checksum,
byte count and page count, with the existing bounded rendering and cleanup rules.
The preview request contains only the saved revision and material kind; it accepts
no paths or renderer-supplied PDF.

Verification:

- Desktop frontend: 215 tests passed across 39 files, including revised cover
  controls, failed/mismatched PDF responses and late-response cancellation.
- Desktop Rust: 81 tests passed in the initial full suite; the existing disconnect
  test could not open its local socket in the sandbox and passed when rerun with
  socket access. One real-Chrome integration test was left ignored in this run.
- Position tests reopen the encrypted store, preserve a valid position, avoid
  duplicate preference writes, and handle negative-coordinate/removed monitors
  and scale changes. Cover preview tests prove equality to download bytes and
  rejection of stale revisions, wrong materials and unprepared exports.
- Production popup under app CSP in disposable Chrome, with mocked desktop IPC
  and an actual cached PDF produced by the desktop renderer, passed minimal fit-width rendering and switching from
  actual cover text editing to the freshly rendered saved PDF in the same popup. The local worker loaded successfully; the preview had no extra controls,
  page labels or success message. Screenshot visually
  checked: `artifacts/overlay-cover-qa/cover-preview.png`.
- TypeScript/build, feature-enabled Clippy with warnings denied, formatting,
  whitespace checks and web security/secret scans passed.
- macOS development bundle built, ad-hoc signed and installed; installed files
  match the build byte-for-byte and signature verification passed. Exactly one
  ORT app remains in Applications, with the previous build ZIP outside it.

An installed native-window hide/reopen/restart walkthrough was not automated;
position persistence and placement were verified with Rust tests. Chrome Web Store
publication and extension changes are outside this change.

## Persistent resume and cover popups

The native application-popup blur handler no longer hides the window. Resume View,
Resume Edit, cover View and cover Edit remain visible while the user interacts with
the overlay or another app. The popup X flushes final edits and hides it; selecting
the opposite View/Edit mode flushes pending edits and switches the same window.
Escape no longer dismisses these popups. Explicit application/overlay lifecycle
cleanup remains in place. The Answers question capture button has been removed;
questions are entered directly.

For this update, all 221 frontend tests across 39 files passed, including outside
click/blur/X checks for all four modes, pending-text flushing during mode switching,
and the Answers tab without question capture. Feature-enabled Rust Clippy with
warnings denied, TypeScript/web and macOS bundle builds, web security/secret scans,
formatting and whitespace checks passed. Native focus behavior was reviewed in the
Tauri event handler; an installed macOS outside-click walkthrough was not automated.
The final installed bundle matches the signed build byte-for-byte, with one ORT app
in Applications and the previous bundle ZIP outside it.
