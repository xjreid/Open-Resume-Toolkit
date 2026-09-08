# M2 date and link editor checkpoint

Date: 2026-09-06. Baseline: M1-qualified `65518eb` plus the local Step 5 schema
and style foundations. Status: implemented and tested locally; not installed or
natively qualified. M1 remains complete. M2 remains incomplete.

## Editor behavior

An existing v1 draft offers **Enable structured dates and link ordering**. The
explanation identifies the compatibility change and preservation of date text
and publications. Selecting it uses the existing v2 upgrade helper and ordinary
edit/autosave flow. Loading and creating an empty workspace still do not silently
upgrade it. The action is disabled during busy/conflict states or invalid edits.

Undo is disabled at the format-upgrade boundary so it cannot return the draft to
v1, which the existing save layer refuses. The UI explains this before the
upgrade. Subsequent content changes continue to support Undo and Redo. No reducer,
contract, backend storage or native lifecycle policy changed in this UI slice.

Each upgraded entry supports identified, ordered date records with optional
labels, year-only or month-and-year precision, expected flags, Present/current,
and omitted endpoints. Users can add, reorder, remove or clear dates. Invalid or
missing required years show field-associated feedback and prevent save through
the existing validator. Reversed valid ranges show a nonblocking warning.
Partially entered invalid years are not displayed as real formatted dates.

Legacy date text stays editable until the user chooses date fields. A separate
inline confirmation offers **Replace date text** or **Keep date text**. Replacement
starts empty fields and guesses no dates. The user can undo this content change.
The legacy field is disabled while structured date records exist, preserving the
single-representation rule. Add-date respects the shared 200-record document bound.

Contact and entry links can be reordered after upgrade. Edits, additions, removals
and reorder use the existing normalization and stable IDs. The UI uses those IDs
as React keys. Original v1 link editing remains available before upgrade.

## Validation

`CI=true pnpm check` passed (`target/m2-date-ui-web-check.log`): 80 desktop tests,
23 contract tests, TypeScript, formatting, builds, static security and dependency
license checks. No dependencies were added.

The live synthetic workflow covers explicit upgrade, unchanged legacy text and
published source, disabled Undo across upgrade, replacement cancel/confirm,
year-only and Present dates, invalid-year save refusal, nonblocking reversed
ranges, expected June 2027, date/link reordering with stable IDs and canonical
order, removal plus Undo, and axe accessibility. It uses mocked native commands;
this is not evidence of an installed native save or WebView interaction.

The existing schema foundation's 202 Rust tests, independent export checks and
backup compatibility evidence are recorded separately in
`m2-schema-v2-foundation.md`. They were not rerun for this frontend-only change.

The installed app remains the M1-qualified artifact. No test account, Keychain,
installation, commit or push was used. Native keyboard/zoom/focus checks remain
part of the later M2 acceptance matrix.

Next Medium work is the document-centered editor and focused editing experience,
including complete entry conveniences and validation guidance. Final style
layouts, historical regeneration, sandboxed import, lifecycle completion and
native M2 qualification remain outstanding. Do not mark Step 5 complete.
