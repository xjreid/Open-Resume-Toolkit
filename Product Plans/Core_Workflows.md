# Core user workflows

## Local profile and startup

- The first launch creates a local profile and explains that user content is stored on this device.
- No ORT registration or login is required.
- The user is prompted to configure backups and may optionally choose Direct API, Connect Codex, or No AI.
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
2. Import a text-bearing PDF or DOCX, review deterministic local mappings into existing fields/custom sections without AI, and optionally request an AI-assisted structural proposal when a provider is configured.

The formatted resume is the primary editor. A section navigator and focused editor allow users to add, rename, reorder, edit, duplicate, or delete sections, entries, fields, links, bullets, and skills. Common fields appear first; optional details appear on request. Blank optional values never produce empty labels or separators in the rendered document.

Suggested sections include contact information, summary, education, experience, internships, projects, skills, certifications, awards, leadership, volunteer experience, research, publications, coursework, activities, languages, accomplishments, links, and custom sections.

Supported entry patterns include dated entries, education, skill collections, achievements/certifications, simple text or lists, and flexible custom entries. Dates may be partial, links store labels separately from destinations, and repeatable values have stable ordering.

### Styles

Resume and cover-letter documents do not inherit the ORT application/website theme, colors, logo, iconography, or marketing language. They must appear like ordinary professional documents when exported.

Initial style directions are:

- Technical/Engineering — the default, based on the supplied Jake's Resume reference and intended to match its familiar single-column structure as closely as licensing permits
- Professional/Business
- Modern/Marketing and Sales

Changing style changes presentation only. Exact appearance belongs in the aesthetic workspace, and every distributed font, icon, template, and asset requires a compatible license. Exact reuse of Jake's Resume source or assets is conditional on documenting the upstream source and compatible license; if exact reuse is not permitted, ORT recreates the common professional structure independently without copying protected source or branding.

## Overlay application workflow

The main desktop window does not contain a job-specific workspace route or tailoring editor. The separate overlay owns the current application workflow and can switch between a compact working size and a large editing/preview size. Its persistent header shows the active direct provider/model, **Codex / model**, or **AI not configured**, browser-connection state, and an always-available **Finish Application** action whenever a workspace exists. A plan type may appear only inside Codex connection/usage details, not as an ORT tier or entitlement badge.

### Stage 1 — capture and review job description

1. The empty overlay presents **Capture job description**.
2. The user selects the relevant job-description text in Chrome or Edge and deliberately invokes the capture bridge. Where a separately approved browser permission allows overlay-initiated capture, the overlay button may request the current selection directly; otherwise it arms the request and directs the user to the extension action or keyboard shortcut required by the browser.
3. The extension returns only the selected text, sanitized page URL, and bounded page title through native messaging/local IPC.
4. The overlay displays the captured description for review. The user can edit it, remove the URL, reject it, or choose **Capture again**.
5. **Continue** becomes available only after the user accepts nonempty job text. Continue verifies a published master and configured AI mode, then begins the initial tailoring operation.

No AI request occurs during capture or review. Browser content never opens a main-window workspace and the extension has no tailoring UI.

### Stage 2 — application materials

After the initial tailoring operation succeeds, the overlay opens the **Resume** tab and provides sibling **Cover letter** and **Answers** tabs. Switching tabs never discards another tab's temporary work.

Only one application workspace is active at a time. A new job capture cannot silently combine with or replace it. The workspace is persisted locally so an application or computer restart does not unexpectedly destroy work, but intermediate versions remain temporary and are removed by Finish Application, explicit discard, or confirmed replacement unless deliberately retained in the tracker.

## Resume tailoring

1. Continuing from Stage 1 sends only the necessary published structured resume, reviewed job description, selected document template/page target, and operation instructions to the chosen provider.
2. The provider returns validated structured resume data, no more than three sharp change-summary points, and bounded Required Qualification Alert candidates in one logical tailoring operation, not a PDF-only result or a second qualification-analysis request.
3. ORT validates the structured result and alerts, renders the initial PDF locally, and opens the overlay's Resume tab.
4. The Resume tab contains a PDF card with filename/status, **Preview and edit**, **Download**, and an operating-system drag handle for dragging the materialized PDF into a compatible browser file-upload field.
5. **Preview and edit** expands the overlay to occupy a significant portion of the screen. The user edits structured resume content beside the large PDF preview; ORT re-renders after changes. The binary PDF itself is never treated as the editable source.
6. **Regenerate resume** requires a nonempty user instruction explaining what was incorrect or what should change. It creates a new validated workspace version and never modifies the published master.
7. The selected current workspace version is the one used for preview, download, drag-out, Finish Application, and optional tracker retention.

The AI may select, condense, reorganize, and truthfully rewrite confirmed information. It may not fabricate facts or silently change the published master. All intermediate versions remain within the current application workspace.

### Required Qualification Alerts

- ORT evaluates only job qualifications explicitly stated as required, mandatory, minimum, or must-have. Preferred, recommended, desired, bonus, and nice-to-have criteria do not create alerts.
- A requirement must map meaningfully to published-resume content, such as an explicitly stated skill, technology, degree, graduation date, certification, license, experience duration, or role/domain experience. Work authorization, sponsorship, citizenship, disability, protected characteristics, criminal or medical history, signatures, consent, and other personal or legal attestations are outside this feature and are ignored.
- **Confirmed mismatch** requires an explicit job requirement and an explicit conflicting fact in the published master. ORT does not infer a mismatch from silence, names, dates unrelated to the requirement, or demographic assumptions.
- **Not found** means supporting information was not located in the published master. The interface says **Not found in your published resume** and never claims that the user lacks the qualification outside the resume.
- Each alert includes a concise normalized requirement, its classification, a bounded supporting excerpt or location from the reviewed job text, and the relevant published-resume evidence for a confirmed mismatch. Provider-generated evidence references must resolve to the local inputs before display.
- Matched requirements, ambiguous requirements, and requirements that cannot map safely to the resume produce no alert.
- Alerts are advisory and non-blocking. They never change the tailored or published resume, create an eligibility/fit score, recommend that the user not apply, or prevent editing, PDF download/drag-out, Finish Application, or tracking.
- The user may dismiss an alert, ignore all alerts for the current workspace, and reopen dismissed alerts. These choices affect presentation only and do not rewrite job or resume content.
- Alerts and their dismissed/ignored state are temporary workspace data. They are cleared when the workspace is finished, discarded, or replaced and are not retained with the tracker entry.

## Cover letters and application answers

### Cover letter tab

- The tab begins with **Generate cover letter**; it does not generate automatically with the tailored resume.
- Generation uses selected contact information, the published master, reviewed job description, and optional user instruction.
- A successful result appears as a PDF card with **Preview and edit**, **Download**, and an operating-system drag handle matching the resume card.
- Preview and edit expands the overlay and exposes the structured cover-letter editor beside a large PDF preview. Edits re-render locally; the PDF binary is not the editable source.
- The current reviewed cover letter remains temporary until Finish Application optionally retains its structured snapshot.

### Answers tab

1. The tab presents **Capture question**. Capture uses the same explicit browser bridge as Stage 1 but creates question context rather than replacing the job description.
2. The overlay shows the captured question for editing and confirmation before an AI call.
3. **Generate answer** sends the reviewed question, allowed job/resume context, and optional word/character limit.
4. The response appears in an editable plain-text box with **Copy answer**.
5. **Reset and capture new question** clears the current unretained question/answer after confirmation and returns to question capture. A reviewed answer may be added to the ordered answer set before resetting.

Answer text is not rendered as a PDF or dragged as a file.

ORT refuses to generate or infer answers requiring personal or legal attestation, including citizenship, visa or work authorization, criminal history, medical or disability information, protected characteristics, signatures, consent, salary history not supplied by the user, or declarations of truthfulness. It explains that the user must answer such questions personally.

## Rendering and export

- Preview and export occur locally using the same structured input, template, fonts, layout rules, locale, page target, and renderer version.
- PDF is the primary resume and cover-letter output in the overlay. Each current PDF has both a visible drag source/card and a **Download** button.
- DOCX and plain text are secondary options.
- Export validation checks selectable text, links, font embedding or safe substitution, page boundaries, clipping, and readable ordering.
- Rendering may adjust spacing within approved bounds but cannot remove content or continually shrink typography merely to force a page count.
- A user-selected export destination is outside the canonical ORT database and remains under the user's control.
- A drag begins only after ORT materializes the current validated PDF in a private temporary export location. Where the operating system and browser accept file drops, the user may drag it from its overlay card into a browser file-upload field. **Download** always remains available, and save/open-folder guidance is the fallback when a site rejects file drag. Dragging or downloading never authorizes automatic submission.

## Finish Application

1. The overlay keeps **Finish Application** available throughout Stage 2.
2. ORT summarizes the selected final resume, optional reviewed cover letter, approved answers, tracking fields, and temporary items that will be deleted.
3. ORT asks whether to add the job to the local application tracker. No entry is created without confirmation.
4. The user can correct company, title, location, date, status, sanitized source URL, and retained-material selections.
5. ORT stores the confirmed entry and selected materials atomically or recoverably. A save failure preserves the workspace.
6. After a successful save—or after the user explicitly finishes without saving—ORT deletes unselected temporary versions and generated drag files, then resets the overlay to Stage 1 with **Capture job description**.

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

## AI Monitoring

The desktop application provides an aggregate **AI Monitoring** view for understanding and controlling ORT's use of the user's configured providers. It is not an individual-call inbox.

- A period control switches between **Week**, **Month**, **Year**, and **All time**.
- The primary content is a token-over-time graph, an estimated-cost-over-time graph for direct API use, and total tokens/estimated cost for the selected period. Week and month use daily buckets; year and all time use monthly buckets unless all-time density requires an adaptive bucket.
- Secondary aggregate breakdowns may group activity by provider, requested/effective model or preset, operation type, or status. Logical operations and attempts remain separately labeled so retries do not look like new user operations.
- Direct API setup selects OpenAI, Anthropic, or Gemini, adds/tests a key, chooses a tested Economy/Balanced/Quality model, reviews current pricing, and optionally sets spending caps. Codex setup uses managed ChatGPT sign-in, chooses a tested returned model, and optionally sets provider-quota thresholds.
- Codex monitoring shows available ORT thread tokens and provider-reported account token/quota windows with clear provenance instead of inventing an API-equivalent dollar cost. Mixed-mode data never adds Codex activity to direct-API cost.
- Missing or unpriced measurements are marked partial or unavailable rather than zero. Provider billing links remain available, and estimated cost is never represented as an invoice or complete account statement.
- Ordinary CSV/JSON export contains the selected aggregate buckets and non-content breakdowns. Attempt-level records remain internal for accounting, guardrails, recovery, and an explicitly requested scrubbed diagnostic export; they are not a primary browsable call table.
- Users can clear activity by date range and configure age-based retention. Clearing monitoring history is visually separate from guardrail controls and never resets cap counters or deletes generated documents.

AI Monitoring covers every remote provider call made through ORT. Direct-API totals do not claim to discover calls made by other applications using the same API key, provider-side adjustments, credits, taxes, account-level tiers, or charges that a provider does not expose to ORT. Codex account quota/token summaries may include other Codex clients and are never presented as ORT-only.

## Settings and local status

The desktop settings/status area shows without exposing secrets:

- Application version, installation/update channel, signing status where available, last update check, and Check for updates.
- Local profile storage use, storage location explanation, schema version, and data export/deletion actions.
- Backup destination, encryption status, last successful backup, restore, and backup test/reminder state.
- Active AI mode; direct provider/credential identity/model preset and caps, or Codex account connection/model/quota caps; connection test, current pricing-catalog or quota-refresh status, link to AI Monitoring, and replace/remove/sign-out actions. API keys and Codex tokens are never displayed.
- Chrome and Edge extension/native-host connection, component versions, compatibility, installation links, disconnect, and repair.
- Links to documentation, GitHub Issues/Discussions, privacy information, the GPL license, copyright and canonical-source attribution, third-party notices, the trademark policy, build provenance and official/preview status, and private security reporting.

## Deletion and uninstall

- Discarding an application workspace permanently removes its temporary local content after confirmation.
- Deleting a retained material leaves the tracker entry and other materials intact.
- Deleting a tracker entry removes its associated snapshots after confirmation.
- Clearing an AI Monitoring date range removes only the corresponding content-free activity records; it does not delete generated materials, reset guardrails, or affect provider-side billing records.
- The settings area provides an explicit **Delete all local ORT data** action with a clear backup reminder and destructive confirmation.
- Uninstall behavior must disclose whether local user data remains. Where platform packaging permits, uninstall and local-data deletion remain separate so accidental uninstall does not silently destroy the only copy.
