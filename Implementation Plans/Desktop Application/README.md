# Desktop application implementation

- [`Desktop_Application_Plan.md`](Desktop_Application_Plan.md) defines the Tauri/React shell, functional information architecture, command boundaries, application state, overlay, accessibility, and OS integration.
- Data, AI, IPC, and updater behavior are owned by their sibling plans and exposed to the desktop through typed use cases.

This folder intentionally contains no theme, palette, typography, illustration, or polished component design. Those decisions belong in `../../Aesthetic/`.

## Current implementation

The Open Folio main window, master editor, AI My Keys/Codex/Data pages, tracker,
Settings, and application overlay are implemented. The tracker loads bounded
pages and native reads run in background work. Profile replacement retires
window/cache state and pending saves. See the
[behavior reference](../../docs/application-behavior/README.md) and
[current implementation map](../../docs/development/current-state.md).

[M2.5](M2_5_Usable_Desktop_and_Resume_Designs.md) is a completed milestone,
not the next delivery. Remaining work is tracked by the
[roadmap](../System%20Documentation/Delivery_Roadmap.md).
