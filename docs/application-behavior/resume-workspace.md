# Resume workspace behavior

## First-time workspace

An untouched empty workspace starts with **Build from scratch**. The optional
starting profile creates useful empty sections but never invents example work
history. Custom starts with contact information and no suggested sections.
After creation, focus moves to the Full name field and normal autosave begins.

The resume workspace then exposes **Edit** and **View** modes in a secondary
navigation bar. Resume style and Publish remain available in that same top bar.

## Content hierarchy

The editor uses this hierarchy:

```text
Master resume
├── Contact information
└── Section (Experience, Education, Skills, custom, ...)
    └── Entry
        ├── heading, subheading, location, and date information
        ├── bullets
        ├── labeled details or skill fields
        └── links
```

What a user may call a subsection is represented as an **entry** inside a
section. Entries can contain their own bullets, details, skills, dates, and
links.

## Edit mode

Edit mode shows the section navigator and editable resume canvas.

### Contact information

Contact fields are edited directly in the resume. Contact links and the visual
divider are part of the editable presentation. Only user-entered information
is rendered.

### Sections

- Add a section by choosing a suggested heading and selecting Add.
- Click a section title in the navigator to rename it; the new text saves as
  part of the draft when focus leaves the input.
- Drag a section vertically to reorder it.
- Drag a section to the trash target, then confirm, to remove the section and
  all its entries. The edit can still be undone afterward.
- The section editor also supports explicit move and confirmed removal
  controls.

### Entries and details

- Add entries within a section and expand one entry at a time for focused
  editing.
- Edit heading, subheading, location, date range or structured dates.
- Add, duplicate, reorder, and remove bullets.
- Add suggested or custom labeled details, mark fields as skills, reorder
  fields, and remove them.
- Add and edit links.
- Duplicate, move, or remove an entire entry.

Validation messages link back to the relevant section, entry, or field.
Invalid content blocks saving, publication, and export until corrected.
Possible duplicate entries are informational: they prompt review but do not by
themselves block a valid save or publication.

The current document limits include 20 sections, 100 entries, 500 bullets, 25
links, 100 skill fields, 200 structured dates, 30,000 total characters, and a
512 KiB serialized document limit. The generated contracts remain the source
of truth for exact limits.

### Autosave and history

Valid edits autosave after approximately 1.2 seconds without another edit.
The header displays Saving, Saved, Not saved yet, or Save interrupted.
Autosave pauses after a failure that requires user attention and never reports
an uncertain save as successful. Revision conflicts and uncertain transport
results require reloading rather than blindly overwriting a newer revision.

Undo and redo operate on the in-memory editing history. Up to the recent 30
document states are retained for undo. Reloading or discarding unsaved edits is
explicitly confirmed.

## View mode

View mode removes editing controls and shows a reading-oriented version. The
version switch selects:

- **Saved resume**: the current draft view. Exporting it first saves valid
  pending edits when necessary.
- **Published resume**: the most recent immutable published snapshot. This
  choice is disabled until a snapshot exists.

The selected source determines both what is displayed and what the View export
buttons export. A published snapshot never silently changes when the draft is
edited later.

## Resume style

The top Resume style selector changes the presentation used by the workspace
and future PDF/DOCX generation. Current selectable styles are supplied by the
document-style catalog (Technical, Professional, and Modern in the current
build).

Changing style does not rewrite resume content or publish a new content
revision. An already generated PDF preview retains the style with which it was
rendered; generate a new preview to use a newly selected style.

## Publishing

Publish creates an immutable snapshot of the latest successfully saved draft.
It is enabled only when storage is ready, the draft has a saved revision, the
editor is clean and valid, no blocking operation is active, and the same
content is not already the latest publication.

Publishing does not erase or replace the editable draft. Later draft edits do
not mutate the published snapshot.

## PDF and Word export

View mode provides **Export PDF** and **Export Word** for the selected Saved or
Published source.

- PDF export renders the selected revision locally with the selected style,
  then exports the exact rendered PDF bytes through a native Save dialog.
- Word export creates a constrained DOCX from the selected revision and style
  through a native Save dialog.
- If Saved resume has valid unsaved edits, export saves them first and exports
  that confirmed revision.
- Export never publishes the draft or mutates a published snapshot.
- Exported PDF and DOCX files are unencrypted and live outside the encrypted
  application profile.
- Cancelling a Save dialog creates no completed export.

The separate PDF preview workflow binds pages to a specific source revision,
renderer receipt, and style. A stale preview cannot be exported as if it were
current. Published and historical previews remain fixed; saved-draft automatic
refresh follows successful saves only.

## Quit behavior

Closing while work is active or edits are unsaved enters the guarded close
flow. The user can keep editing, save before quitting when possible, or
explicitly discard unsaved edits. Discarding unsaved edits does not delete the
last saved draft or published snapshots.

