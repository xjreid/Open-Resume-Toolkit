# Core user workflows

## Local profile and startup

- The first launch creates a local profile and explains that user content is stored on this device.
- No ORT registration or login is required.
- The user is prompted to configure backups and may optionally configure an AI provider.
- The app reports whether Chrome and Edge extensions are installed and whether their native-messaging connections work.
- Multiple operating-system users receive separate data by default. Running ORT on another computer creates an independent profile unless the user imports a backup.

## Master-resume workflow

ORT provides one familiar structured resume editor rather than requiring users to manage an abstract career database.

### Draft and publish

- Each local profile has exactly one autosaved master-resume draft and at most one published master-resume snapshot.
- All ordinary editor changes save locally to the draft.
- Undo and redo are editing conveniences, not a permanent user-visible version history.
- The user deliberately selects **Publish master resume** when the draft is ready for tailoring.
- Publishing replaces the previous published snapshot after confirmation; it does not create a second master resume.
- Tailoring always uses the published snapshot. If unpublished changes exist, the application states clearly that they will not be included.

### Starting and editing

The user can:

1. Build a resume from scratch.
2. Import a text-bearing PDF or DOCX and review an AI-assisted structured proposal when an AI provider is configured.

The formatted resume is the primary editor. A section navigator and focused editor allow users to add, rename, reorder, edit, duplicate, or delete sections, entries, fields, links, bullets, and skills. Common fields appear first; optional details appear on request. Blank optional values never produce empty labels or separators in the rendered document.

Suggested sections include contact information, summary, education, experience, internships, projects, skills, certifications, awards, leadership, volunteer experience, research, publications, coursework, activities, languages, accomplishments, links, and custom sections.

Supported entry patterns include dated entries, education, skill collections, achievements/certifications, simple text or lists, and flexible custom entries. Dates may be partial, links store labels separately from destinations, and repeatable values have stable ordering.

### Styles

Initial style directions remain:

- Technical/Engineering
- Professional/Business
- Modern/Marketing and Sales

Changing style changes presentation only. Exact appearance belongs in the aesthetic workspace, and every distributed font, icon, template, and asset requires a compatible license.

## Capture and application-workspace workflow

The overlay header shows the user's name from the published master when available, the current AI provider/model or **AI not configured**, and browser-connection state. It has no account tier or subscription badge.

1. The user explicitly activates the Chrome or Edge extension on a job page.
2. The user selects the relevant job-description text or application question.
3. The extension sanitizes the page URL and presents the text and URL for review.
4. The extension sends the approved capture through native messaging to the desktop app.
5. The overlay lets the user edit or reject the capture before it becomes workspace context.

Only one application workspace is active at a time. A new job capture cannot silently combine with or replace an existing workspace. The user must finish, discard, or explicitly replace the current workspace after seeing what temporary material will be removed.

The workspace is persisted locally so an application or computer restart does not unexpectedly destroy work. It remains temporary in product meaning: intermediate versions are deleted when the workflow is finished or discarded unless deliberately retained in the tracker.

## Resume tailoring

1. The user reviews the job description and any instructions.
2. ORT verifies that a published master exists and shows the selected AI provider/model.
3. The desktop app sends only the necessary published structured resume, reviewed job description, selected style/page target, and instructions to the chosen provider.
4. The provider returns validated structured resume data, not a PDF-only result.
5. ORT compares the result with the published master and displays approximately three concise, verified bullets describing important selections, rewrites, or omissions.
6. The user reviews the formatted result and material differences, edits it directly, and may request a refinement.
7. A refinement may return a structured patch when safe or a full structured resume when required.
8. The user deliberately selects a final version for export or tracker retention.

The AI may select, condense, reorganize, and truthfully rewrite confirmed information. It may not fabricate facts or silently change the published master. All intermediate versions remain within the current application workspace.

## Cover letters and application answers

After job context is reviewed, the user may request:

- A structured cover letter using selected contact information, the published master, job description, and instructions.
- A draft answer to a deliberately captured application question, optionally constrained by word or character limits.

Both results remain editable and temporary until deliberately selected for retention. The user can copy answer text or render a selected cover letter locally.

ORT refuses to generate or infer answers requiring personal or legal attestation, including citizenship, visa or work authorization, criminal history, medical or disability information, protected characteristics, signatures, consent, salary history not supplied by the user, or declarations of truthfulness. It explains that the user must answer such questions personally.

## Rendering and export

- Preview and export occur locally using the same structured input, template, fonts, layout rules, locale, page target, and renderer version.
- PDF is the primary resume and cover-letter export.
- DOCX and plain text are secondary options.
- Export validation checks selectable text, links, font embedding or safe substitution, page boundaries, clipping, and readable ordering.
- Rendering may adjust spacing within approved bounds but cannot remove content or continually shrink typography merely to force a page count.
- A user-selected export destination is outside the canonical ORT database and remains under the user's control.
- Where the operating system and browser permit, the user may drag a completed local export into a browser file-upload field. This is a local file interaction and never authorizes automatic submission.

## Finish Application

1. The user selects **Finish Application**.
2. ORT summarizes the selected final resume, optional reviewed cover letter, approved answers, tracking fields, and temporary items that will be deleted.
3. ORT asks whether to add the job to the local application tracker. No entry is created without confirmation.
4. The user can correct company, title, location, date, status, sanitized source URL, and retained-material selections.
5. ORT stores the confirmed entry and selected materials atomically or recoverably. A save failure preserves the workspace.
6. After a successful save—or after the user explicitly finishes without saving—ORT deletes unselected temporary versions and resets the overlay.

Exporting a document alone does not create a tracker entry. Declining tracker retention creates no permanent application record.

## Application tracker

The local tracker supports unlimited ordinary use subject only to device capacity and protective input limits. It provides:

- Company
- Job title
- Location
- Date applied
- Status
- Sanitized captured job URL
- Selected final tailored-resume snapshot
- Selected cover-letter snapshot
- Approved application-question and answer set

Initial statuses are Applied, Online Assessment, Interview, Accepted, Rejected, Withdrawn, and Other. Status values are extensible and unknown future values must remain readable.

Fields may remain blank when the user does not know them. Selecting Other permits a bounded custom status label without replacing the stable underlying status code.

Users can create entries manually, edit tracking fields, replace retained materials deliberately, delete individual materials, delete an entry with confirmation, filter/search locally, and export tracking data as CSV. Opening a retained resume or cover letter renders its structured snapshot on demand. Opening an answer set shows ordered questions and answers with copy/export controls.

## Settings and local status

The desktop settings/status area shows without exposing secrets:

- Application version, installation/update channel, signing status where available, last update check, and Check for updates.
- Local profile storage use, storage location explanation, schema version, and data export/deletion actions.
- Backup destination, encryption status, last successful backup, restore, and backup test/reminder state.
- Configured AI provider/model preset, connection test, provider usage/cost metadata when available, and replace/remove-key actions. The key is never displayed in full.
- Chrome and Edge extension/native-host connection, component versions, compatibility, installation links, disconnect, and repair.
- Links to documentation, GitHub Issues/Discussions, privacy information, the GPL license, copyright and canonical-source attribution, third-party notices, the trademark policy, build provenance and official/preview status, and private security reporting.

## Deletion and uninstall

- Discarding an application workspace permanently removes its temporary local content after confirmation.
- Deleting a retained material leaves the tracker entry and other materials intact.
- Deleting a tracker entry removes its associated snapshots after confirmation.
- The settings area provides an explicit **Delete all local ORT data** action with a clear backup reminder and destructive confirmation.
- Uninstall behavior must disclose whether local user data remains. Where platform packaging permits, uninstall and local-data deletion remain separate so accidental uninstall does not silently destroy the only copy.
