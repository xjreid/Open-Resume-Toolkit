# Live UI audit and update — October 4, 2026

The original compact text editor is restored with × (clear), B, I, and Link. Information retains its Paragraph/Bullet points toggle. Autosave, text formatting, link validation, Escape, and outside-click dismissal remain available. The corrected add-field positions and empty-date styling are preserved.

Workspace headings now sit outside the content scroll area. Master resume mode buttons and AI My Keys/Data remain at the top of that area. Resume sections stays below the resume toolbar and has independent vertical scrolling when its contents exceed the available height.

## Live coverage

The audit used the installed macOS app and native candidate bundles with the actual development profile, through native accessibility controls and screenshots. The tested sizes were 1920 × 960, the default 1080 × 760, and the supported minimum 720 × 520.

- Master resume Edit: field placement, compact Role and Skills/details editors, four formatting controls, Link validation, fixed heading and toolbar, document scrolling, independent section-list scrolling.
- Master resume View: document layout, version and export controls, and fixed top panel.
- AI My Keys and Data: real content, narrow layouts, title/tab placement, scrolling, and arrival at the top when changing workspaces.
- Application tracker: real table contents, narrow layout, filters, and fixed title panel.
- Settings: visible Backup and recovery form layout at wide and minimum sizes.
- Overlay: native empty/offline layout and controls.

## Confirmed audit findings

| Severity | Finding | Impact | Resolution |
| --- | --- | --- | --- |
| P2 | Scroll position carried between main workspaces | A newly selected workspace could open halfway down its content. | Reset the content scroll area on destination changes while retaining mounted feature state. Confirmed live and by a regression test. |
| P2 | Link address overflowed its compact popup at minimum width | The URL control extended past the popup boundary. | Allow the input to shrink within the popup. Confirmed live at 720 × 520. |
| P2 | Expanding Link could hide Apply below the window | Users had to scroll to reach the action after opening the popup. | Scroll the expanded editor into view when Link opens. Confirmed live with Apply visible at 720 × 520. |

No additional blocking layout issue was observed in these reviewed states. Title and section-panel scrolling were corrected as requested, rather than treated as a new visual redesign.

## Scoped health assessment

| Dimension | Score | Evidence and limit |
| --- | --- | --- |
| Accessibility | 3/4 | Formatting controls retain action names and pressed states; live editor focus is visible. Native accessibility snapshots intermittently omitted Settings/Data descendants, so this is not a full VoiceOver certification. |
| Performance | 3/4 | Native navigation and scrolling remained responsive during inspection. No frame-time profiling was performed. |
| Responsive layout | 3/4 | Main surfaces and compact popup checked at the native default and minimum sizes; tracker retains its horizontal table layout. |
| Theming | 3/4 | Existing Hanken/teal workspace and document typography remain consistent; document styles are unchanged. |
| Implementation integrity | 4/4 | Changes are limited to presentation and scroll behavior; existing functionality and mounted work are retained. |
| **Total** | **16/20** | **Good within this audit's scope.** |

The resolved findings were local containment and scroll-state problems. Shared CSS and the app shell were corrected at their source. The review did not exercise provider requests, credential changes, data deletion, recovery operations, browser capture, or Windows rendering. Settings storage/browser subsections and generated overlay states were not re-tested live in this pass.

## Verification and installation

238 desktop tests in 40 files, TypeScript, the production frontend build, formatting, and diff checks pass. The native optimized development bundle and its preserved sandboxed import helper pass deep, strict signature verification. All 25 installed bundle files match the verified build.

The live app was closed through its Quit menu before replacement. The update was installed without reopening it. The previous bundle is retained for rollback; [installation.json](installation.json) records the paths, hashes, and verification result. The installer does not modify the development profile or credentials.
