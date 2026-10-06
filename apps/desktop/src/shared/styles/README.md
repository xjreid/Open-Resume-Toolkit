# Workspace stylesheet ownership

`index.css` loads tokens and shared controls first, followed by workspace owners.
Each property has one active owner rather than a later override sheet.

- `foundation.css`: fonts, colors, spacing, base elements, shared dialogs and forms.
- `exports.css`: PDF previews, export choices and native drag affordances.
- `application.css`: application capture and shared material workflow controls.
- `controls.css`: button variants and shared interaction states.
- `shell.css`: shell, rail and workspace navigation.
- `resume.css`: imports the resume workspace, navigation, reading document, canvas,
  field, contact and date owners. Each family retains its document-style variants.
- `import.css`: import mapping and review.
- `tracker.css`: tracker table, detail views and row controls.
- `ai.css`: imports the workspace, key/spending, key dialog, monitoring and activity
  settings owners.
- `settings.css`: settings navigation, storage and backup panels.

Keep media queries and state selectors with their owner. Change the owning rule
when adjusting behavior; avoid introducing a final override sheet. Shared
controls should use the common variants, with owner rules only for local layout.
