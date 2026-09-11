# Desktop workspace structure

The desktop uses the Precision Workbench direction in
[Aesthetic](../../Aesthetic/Precision_Workbench_Visual_Direction.md). The app
identity and controls are separate from the exported document styles.

- `AppShell.tsx` owns the shared brand, destination navigation, status slot, and
  development footer. Add a destination to `WORKSPACE_DESTINATIONS` only when
  its workspace is implemented. Future application, tracker, and monitoring
  workspaces should use this shell rather than add tools to the resume editor.
- `App.tsx` currently owns the resume session, its reducer, save/publish boundaries,
  validation navigation, and Write / Review & style / Export views. Destination
  changes keep this session mounted, including unsaved edits and undo history.
  Typed commands remain in `command-client.ts`; the shell does not call IPC.
- `SettingsWorkspace.tsx` accepts feature-owned backup and storage panels. Its
  section navigation preserves panel state and is blocked during operations.
  Future connection/settings panels should own their typed state and operations
  in the same way, while sharing shell navigation and styling.
- `app.css` defines the shared desktop controls and workbench layout. The named
  Navy, Ink, Mist, Canvas, border, and success tokens represent application
  roles. Keep readable focus/disabled states and responsive layouts. Do not
  hide status or destructive-action recovery at smaller sizes.
- `Brand` is reused by the overlay. The overlay describes its current purpose;
  application and browser controls should appear only when their workflows exist.

Document styles belong to the local PDF/DOCX renderer, not to application theme
variables. Style changes must preserve the structured draft, and PDF export must
continue to use the exact reviewed native render rather than the live HTML view.
