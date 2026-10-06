# Desktop workspace structure

The desktop uses the user-approved Open Folio direction in
[Aesthetic](../../Aesthetic/README.md) and the [design system](../../DESIGN.md). The app
identity and controls are separate from the exported document styles.

- `AppShell.tsx` owns the shared brand, destination navigation, status slot, and
  persistent navigation rail. Add a destination to `WORKSPACE_DESTINATIONS` only when
  its workspace is implemented. Future application, tracker, and monitoring
  workspaces should use this shell rather than add tools to the resume editor.
- `App.tsx` composes the workspaces and resume workflow controls.
  `use-resume-session.ts` owns loading, editing, health, save/publish and export.
  `ResumeCanvas`, `PublishedResume` and editor controls are leaf modules shared
  by the main window, import review and application overlay.
- `ProfileBoundary` remounts all profile owners after native profile retirement.
  `SaveCoordinator` owns coalescing, optimistic acknowledgement and retirement
  of pending application writes. Native writes also check profile identity.
- `desktop-client.ts` pairs AI/application/tracker command names with generated
  argument/value types and decodes unknown responses before state updates.
  Serialized types and decoder schemas come from native Rust models; existing
  semantic validators remain in `command-client.ts` for resume/export flows.
- `SettingsWorkspace.tsx` accepts feature-owned backup and storage panels. Its
  section navigation preserves panel state and is blocked during operations.
  Future connection/settings panels should own their typed state and operations
  in the same way, while sharing shell navigation and styling.
- [`styles/index.css`](../../apps/desktop/src/shared/styles/README.md) loads
  tokens/shared controls followed by each workspace owner. Responsive rules
  and state selectors stay with the corresponding owner.
- `Brand` is reused by the overlay. The overlay describes its current purpose;
  application and browser controls should appear only when their workflows exist.

Document styles belong to the local PDF/DOCX renderer, not to application theme
variables. Style changes must preserve the structured draft, and PDF export must
continue to use the exact reviewed native render rather than the live HTML view.

Canonical application models belong to `ort-domain`; typed encrypted workflow
services and artifact-source projection belong to `ort-application`. Tauri
commands retain authorization and native handles. Application artifact cache
keys include the active profile UUID, revision and material kind. Tracker lists
project bounded metadata pages; retained content is fetched by indexed ID only
when a detail view or point operation needs it.

Contract generation requires the installed workspace Prettier binary for stable
TypeScript output. Rust serializes synthetic wire fixtures that the TypeScript
boundary tests accept alongside malformed-response rejection cases.
