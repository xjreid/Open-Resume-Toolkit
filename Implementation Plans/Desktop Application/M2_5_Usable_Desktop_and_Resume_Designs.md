# M2.5 — Usable desktop and resume designs

## Goal and place in delivery

**Next milestone, before M3.** Turn the implemented offline capabilities into a
coherent, usable resume application. M0–M2 remain technically accepted; that
acceptance did not establish visual readiness. M2.5 owns the overall layout,
interaction design, approved aesthetic and professional resume output.

Authority: [resume editor](../../Product%20Plans/Resume_Editor_and_Schema.md),
[desktop plan](Desktop_Application_Plan.md),
[Precision Workbench direction](../../Aesthetic/Precision_Workbench_Visual_Direction.md)
and [roadmap](../System%20Documentation/Delivery_Roadmap.md).

## 1. Application layout and visual system

Implement Precision Workbench using the approved Quiet Navy palette and Offset
Open Frame identity. Apply consistent typography, spacing, borders, control sizes,
icons, focus states and restrained status colors to the actual application.
Use the approved aesthetic reference for visual character, with product workflows
controlling placement. A palette change alone does not complete this work.

- Establish clear main navigation and a consistent page hierarchy. Resume editing
  is the primary offline workspace; settings, backup/recovery and storage management
  belong in dedicated, clearly labeled locations.
- Group actions by their task. Keep primary actions obvious; place secondary and
  destructive actions in appropriate menus/settings instead of scattering buttons
  across the editor. Keep status understandable without exposing engineering details.
- Use a collapsible section navigator, central continuous document preview, and
  a focused editing panel/drawer for the selected content. Closing editing returns
  to the complete document. Keep add/edit controls beside their relevant content.
- Make dialogs, empty states, save/error messages and import review use the same
  visual system. Preserve keyboard access, readable labels and visible focus.
- Support ordinary and smaller desktop window sizes without overlapping controls,
  clipped panels or inaccessible actions. Preview retains usable scroll and zoom.
- Give the existing overlay a coherent appearance and clear purpose. Future AI,
  application and browser features must not appear as working controls before they
  exist. Their complete workflows remain in M3–M5.

## 2. Practical resume creation and editing

A person should be able to build a real resume without understanding the internal
schema or searching through a wall of fields.

- Provide clear Build from scratch and Import choices. Offer the approved optional
  starting profiles as section suggestions, without restricting later edits or
  inserting fabricated employment content.
- Make contact details, experience, education, projects, skills and custom sections
  straightforward to enter. Use human-readable field names and reveal optional
  details only when useful.
- Support adding, editing, deleting and reordering sections, entries, bullets and
  links. Provide keyboard alternatives to drag operations and useful undo/redo.
- Make dates, current roles, expected graduation and links understandable. Show
  validation beside the relevant field, with a direct way to correct it.
- Keep the document preview connected to the draft. Show saving/saved/failed status,
  draft versus published state, page count and relevant overflow information clearly.
- Make publish, style selection, preview and export discoverable in one coherent
  workflow. Preserve draft data when switching styles and retain saved work on reopen.
- Integrate import review naturally: inspect/edit/keep/reject content and cancel
  without modifying the saved resume. Preserve the existing explicit apply boundary.

## 3. Professional document designs

Ship three deliberately designed resume styles, not minimally different development
fixtures. All three should look credible for a real application. The app theme is
separate: exported documents do not inherit ORT branding, logo or interface chrome.

| Style | Visual target |
| --- | --- |
| Technical/Engineering — default | Clean Jake's Resume-style structure: compact single column, strong name/contact header, clear section rules, aligned role/date rows, readable achievement bullets and restrained black typography. Closely follow the familiar professional layout while keeping text selectable and reading order sensible. |
| Professional/Business | Equally clean single-column professional resume, with more breathing room, refined typography and clear hierarchy. Distinct from Technical without decorative clutter. |
| Modern/Marketing and Sales | The same readable foundation with restrained accent color and stronger typographic personality. Maintain clear chronology and content order; avoid dashboards, skill meters and oversized ornamental blocks. |

Design concrete previews for all three in the aesthetic workspace during
implementation and render the same representative content through them. Exact
reuse of Jake's Resume source/assets requires compatible licensing; otherwise
independently implement the common structure. Do not imply an exact clone without
an inspected, licensed reference.

Provide sensible margins, bullet indents, line spacing, heading hierarchy and page
breaks. Avoid orphan headings, accidental blank pages and clipped content. Support
both a concise one-page resume and natural longer content without shrinking text
into unreadability or silently dropping facts. Style changes affect presentation,
not stored content. PDF should match preview; DOCX should preserve the intended
hierarchy and readable layout, allowing normal reader-specific pagination variation.

## Implementation order

1. Consolidate visual tokens/components and redesign the app shell/navigation.
2. Complete the document-centered creation/editing flow and organize secondary pages.
3. Refine the three templates and integrate consistent preview/style/export controls.
4. Walk through the complete offline flow, fix visible rough edges and present the
   actual app plus sample exports for the user's visual review.

## Lightweight completion check

Follow the [focused testing policy](../../Product%20Plans/Quality_Accessibility_and_Verification.md).
Run the relevant fast build/component checks and one short create/edit/publish/
export/reopen walkthrough with synthetic content. Include adding/reordering an
entry, one field correction, switching all three styles without losing content,
and cancelling an import review. Inspect PDF and DOCX samples for each style;
use one longer sample to spot obvious pagination problems. Check keyboard focus
and a smaller desktop window. Reuse evidence from this walkthrough; no additional
account matrix, expiry waits, fuzz campaigns or extended acceptance sessions.

**Done means usable and visibly aligned with the approved direction.** Present
screenshots of the real application and the three sample document designs for
user review. Address reported material layout/workflow/design problems before
moving to M3. A passing build or screenshots of a mockup alone cannot close M2.5.
Keep the result note concise and preserve any remaining minor limitations.
