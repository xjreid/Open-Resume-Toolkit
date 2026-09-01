# Master-resume editor and structured schema

## Purpose

The desktop application provides a simple structured resume builder that feels like editing a normal formatted resume rather than maintaining an abstract career database. Manual creation and editing do not require an AI provider or internet connection.

The master resume is the user's complete resume-information source. It may contain more entries and pages than a normal job-specific resume, but users are not expected to rank content, maintain multiple profiles, or apply special AI tags. The separate tailoring workflow creates a concise job-specific resume from the published master.

## Starting a master resume

The initial screen presents:

1. **Build from scratch**
2. **Import an existing resume**

Both update the one master-resume draft. Import never creates a second named resume.

Build from scratch begins with contact information and a small set of suggested sections. The user may choose an optional starting profile:

- Student/Recent Graduate
- Experienced Professional
- Technical
- Academic/Research
- Sales/Marketing
- Custom

A starting profile changes suggested sections only and never restricts later content.

## Draft and published state

- Ordinary edits autosave to the one local draft.
- A persistent status distinguishes **Draft saved**, **Saving**, **Save failed**, and **Published master resume**.
- Undo and redo are available for current editing.
- The user explicitly publishes the validated draft when ready.
- Publishing replaces the previous published snapshot atomically and does not create another master resume.
- If unpublished changes exist, tailoring continues to use the last published snapshot and says so clearly.
- A save or publish failure never discards the last valid draft or published snapshot.

## Primary editor layout

- A narrow, collapsible section navigator appears on the left.
- The center shows the complete continuously rendered resume using the selected style.
- Selecting visible content opens a focused editing panel or drawer.
- Closing the panel returns to the complete document rather than leaving a permanent wall of fields.
- Add-entry controls appear within or beside relevant sections.
- Sections, entries, bullets, skills, links, and other repeatable values can be reordered through accessible controls. Drag-and-drop may supplement but never replace keyboard controls.
- Long entries may collapse in navigation/form views while remaining complete in document preview.
- Preview supports page scrolling and zoom without changing exported layout.
- The user can see page count, overflow, missing font, broken link, and validation status.

## Suggested sections

- Contact Information
- Professional Summary
- Education
- Experience
- Internships
- Projects
- Skills
- Certifications and Licenses
- Awards and Honors
- Leadership
- Volunteer Experience
- Research
- Publications
- Coursework
- Activities and Organizations
- Languages
- Accomplishments
- Portfolio and Professional Links
- Custom Section

Users may add, delete, rename, and reorder sections. A custom section supports information that does not fit a suggested type.

## Entry types and fields

### Dated entry

Suitable for experience, internships, projects, research, leadership, volunteering, activities, and accomplishments. Optional fields may include:

- Title or name
- Organization
- Subtitle or role
- Location
- Start date and end date
- Ordered labeled links
- Ordered technologies or inline details
- Description and achievement bullets
- Custom labeled detail

### Education entry

Optional fields may include:

- Institution
- Degree
- Field of study
- Location
- Start date
- Graduation or expected-graduation date
- GPA
- Honors
- Coursework
- Activities
- Links
- Description/details

### Skill collection

- User-defined group names such as Languages, Frameworks, Backend and Data, Security, Tools, or Cloud.
- Ordered skill items.
- Groups and items may be added, renamed, reordered, or removed.

### Achievement, award, or certification

Optional fields may include:

- Name
- Issuing organization
- Date or date range
- Location
- Credential ID
- Verification link
- Metrics
- Description bullets

### Simple text or list

Suitable for summaries, interests, languages, coursework, and concise custom information.

### Custom entry

Provides title, subtitle, dates, location, labeled links, inline details, bullets, and user-labeled optional fields without allowing executable or renderer-control content.

## Project-entry example behavior

A project may include project name, role/subtitle, ordered technologies, any number of bounded labeled links such as Live/GitHub/Demo/Documentation/Portfolio, location, start/end dates, reorderable bullets, and one or more bounded custom details.

Only the name is needed to identify the entry. Empty technologies, links, dates, locations, and bullets disappear cleanly. Separators appear only between visible values.

## Optional-field rendering

- Common fields appear first. Uncommon optional fields remain behind **Add optional detail**.
- Empty fields are omitted entirely from preview and export.
- Separators are generated only between visible values; missing information cannot create repeated or dangling separators.
- Removing a location, date, link, technology, or subtitle causes surrounding content to realign automatically.
- Templates must render every valid combination of completed and missing fields without broken alignment.
- Placeholder examples guide entry but are never saved or rendered as real content.

## Ordering and stable identity

- Every section, entry, bullet, link, skill, date, repeatable field, and custom value has a stable opaque identifier and explicit order.
- Renaming a section or link label does not change its identity.
- Reordering presentation does not duplicate or lose content.
- Unknown future fields remain recoverable/exportable and do not crash older readers.

## Dates, links, and values

- Dates support year-only, month-and-year, present/current, expected dates, and bounded ranges without inventing missing precision.
- Invalid ranges produce an understandable warning without blocking legitimate incomplete knowledge.
- Links store a display label separately from a validated destination.
- Rendering allows only approved web/email protocols and prevents script, data, local-file, or executable schemes.
- Custom labels and values are bounded by the central configuration register and escaped in every preview/export format.

## Editor convenience and validation

- Add, duplicate, reorder, and delete entries and bullets.
- Confirm destructive section or entry deletion and support undo where safe.
- Detect malformed links, invalid dates, likely duplicates, unsupported characters, layout overflow, missing fonts, and content that cannot render cleanly.
- Importing into an existing draft presents possible duplicates with merge, keep-both, and discard choices.
- Do not require users to tag, rank, pin, or categorize information for AI selection.
- Validation should preserve work and identify the exact field rather than rejecting an entire draft without guidance.

## Styles and templates

Initial style categories are:

1. **Technical/Engineering** — the default; a compact, ATS-conscious single-column direction based on the supplied Jake's Resume reference and intended to match that familiar professional structure as closely as licensing permits.
2. **Professional/Business** — polished and spacious professional direction.
3. **Modern/Marketing and Sales** — more visual personality while preserving readability and export quality.

Changing style changes presentation only. It never flattens, deletes, rewrites, or requires re-entry of structured content. Resume and cover-letter exports never inherit the ORT application/website color theme, logo, iconography, or brand language. Exact typography, spacing, colors, assets, and template layouts belong in the aesthetic workspace. Exact reuse of Jake's Resume source or assets requires a documented compatible upstream license; otherwise ORT independently implements the common professional structure without copying protected source or branding.

## Import review

The import flow follows `AI_and_Import.md` and uses the same schema as manual entry.

- Show original/extracted content beside the proposed structure.
- Associate an internal confidence/uncertainty signal with extracted fields without presenting it as objective truth.
- Highlight missing, uncertain, unfamiliar, or duplicate mappings.
- Let the user edit, move, relabel, accept, or reject every proposed section, entry, and field.
- When an unfamiliar heading appears, the user may select an existing type or create a custom section.
- Only confirmed proposal content enters the draft.

## Master and tailored outputs

- The editor renders every nonblank draft field in selected order and may span multiple pages within the central limits.
- The published master is the factual snapshot used for tailoring.
- Tailored results use the same structured primitives wherever possible, remain directly editable, and record their source published revision.
- The deterministic local renderer controls typography, alignment, spacing, headings, links, bullets, and page breaks.
- Renderer layout changes never alter stored factual content.
