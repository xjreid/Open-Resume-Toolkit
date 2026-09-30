# Tracker link display and browser startup

Tracker sources stay on one line with an ellipsis when they exceed their column.
An Expand text control appears only for overflowing sources. Expanded sources
wrap within the same column and end with Compress. Both controls use normal
weight text with no border or background; hovering underlines the control.
The complete source and native link target remain intact in both states.
Double-click editing, short sources, and independent row expansion are preserved.

The registered macOS development browser bridge now starts during app setup on
each launch. Disable in Settings applies until the next launch. Missing or
invalid registration, other package identities, and builds without the feature
remain disabled. Overlay readiness still requires live extension communication.

Verification on 2026-09-30:

- Full desktop frontend suite: 223 passing tests. After the final control styling,
  the five tracker tests passed again; TypeScript and production builds passed.
- Rust desktop suite with development bridge: 82 passed, one existing real
  Chrome integration test ignored. Feature Clippy with warnings denied and default
  desktop compilation passed.
- Disposable Chrome checked the production UI under the application CSP with
  mocked desktop IPC: one-line truncation, in-column controls, full native URL
  opening in both states, compression, resize observation, short sources,
  double-click editing, no row saves, normal text weight, transparent controls,
  and hover underline. Screenshots are in `artifacts/tracker-link-qa/`.
- Web security, secret scanning, formatting, and diff whitespace checks passed.
- The installed bundle matches the final ad-hoc signed build; one ORT app remains
  in Applications. The previous bundle ZIP is outside Applications.
- An earlier installed-app launch probe did not observe native bridge readiness;
  the app exited during the probe. Live automatic startup remains unverified.
  No further installed-app launches were performed after the user requested that
  the installed app not be opened.

## Approved delete control

After user approval of the two-state draft, the row Delete button was replaced
with a red SVG cross. Its 34px target has a transparent background and border at
rest; hovering or keyboard focus reveals the soft red square border with no
background fill, per the subsequent refinement. The existing accessible row
label and delete confirmation remain intact.

The five tracker regression tests, TypeScript, production bundle, formatting,
and web security checks passed. Disposable Chrome under the app CSP with mocked
IPC verified both visual states, stable target dimensions, keyboard activation,
Cancel retaining the row, and confirmation deleting only the selected row.
Screenshots are in `artifacts/tracker-delete-draft/implemented-*.png`.
The already-running app was quit normally through its existing close guard,
then updated with a byte-for-byte verified signed bundle and left closed.
The final transparent-background refinement passed focused Chrome hover/focus
checks, TypeScript, production packaging, and formatting checks.
