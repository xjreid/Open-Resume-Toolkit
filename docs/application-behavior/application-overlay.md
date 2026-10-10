# Application overlay

The application workspace opens as a fixed 360 × 760 logical-pixel rail at the
left edge of the monitor work area, vertically centered. Its height is clamped
on smaller displays. Drag the header to move it within the monitor work area;
the rail cannot be resized.
The compact header keeps the logo beside a one-line Open Resume Toolkit title
and request activity. A red Stop text control
appears beside the activity while a request is running. A fixed bottom footer
shows browser connection status on the left and the provider, model, and
applicable reasoning on the right. AI readiness stays in the header.
The native red window control requests an app quit and shows the
existing confirmation for unsaved work. Opening the overlay leaves the main
window visible; the native yellow control minimizes only the overlay.

## Provider controls

The fixed footer summarizes a usable provider and model, for example
**API key: Gemini 3.5 Flash** or **Codex: GPT-5.6 Sol · High**. Reasoning appears
only for a selected Codex model. Long summaries wrap within the footer.
When AI is unavailable, the footer shows **No model connected** and the header
shows **Connect API key or Codex**. Enabled Codex without a signed-in account
instead shows **Sign in to use Codex** in the header.
Model and reasoning changes are made in the main app. The overlay has no
provider settings controls and uses the current settings for AI work.
Connection changes in the main app refresh the overlay's status immediately.
Codex usage is not displayed in the overlay; account-wide usage remains
available in the main app.

## Capture and tailor

Stage 1 shows Capture and Tailor, then Job Description and Job URL cards with
Waiting/Complete status. View opens a separate editable popup. Clicking outside
the popup hides it; Escape and its close button also close it. Text edits save
locally. No resume style selector appears in Stage 1.

Capture is enabled only with an active browser bridge. The current unsigned
native host rejects browser connections, so this build reports Offline and
disables Capture. The request command also fails explicitly if invoked without
a bridge. Users can enter text and a URL through the card popups. Existing
browser captures still require the review/accept step before replacing job text.

Tailor requires a ready API key or Codex connection, a published master resume,
and a nonempty job within the existing input limits. It saves reviewed job details before starting
the request. No live API request is sent just by capturing or editing text.

## Review, edit, export

Stage 2 shows the company and role above Finish Application when either is
available; otherwise Finish Application appears first. Resume, Cover letter,
and Answers tabs follow. The resume tab contains tailoring notes and qualification
alerts, a style selector, PDF/Word format selection, Download and Drag me,
View/Edit controls, and an AI refinement input.

The qualification alert panel is completely hidden when the alert list is empty,
including when only the candidate-truncation flag is present.
Validated alerts form a static list of brief points such as “Missing C language.”
The panel has no hide, dismiss, or reopen controls and displays alerts even if an
older workspace had hidden them. Job excerpts and resume evidence stay in the
validated data rather than being quoted in the UI. Points use the validated
qualification target, with requirement labels as a fallback for older workspaces.
Existing alerts remain intact during resume refinement. Empty results
do not certify that every job qualification is met. See the
[qualification alerts audit](qualification-alerts-audit.md) for validation scope
and current limitations.

View uses the desktop's HTML/CSS `PublishedResume`. Edit uses its inline
`ResumeCanvas`, including all existing structured entry fields. AI output already
maps into this shared resume schema; no PDF is used as the editing surface.
Edits affect the application copy, not the published master.

Edits autosave serially. Later typing is preserved while an earlier save is in
flight. Popup transitions flush their final edited value before another view or
operation can replace it. Save failures retain the edited draft and provide a
retry; export stays unavailable until the edited revision is saved.

Both PDF and DOCX are prepared eagerly from each saved revision using the same
native renderers as the desktop workspace. Switching format uses those prepared
bytes. Editing or changing style invalidates readiness immediately; Download
and Drag me re-enable after the updated files are prepared. Native commands also
check the workspace revision before serving a cached file. The cache retains
only the latest revision for each material and clears on Finish.

Native file drag currently supports macOS; other platforms return an explicit
unavailable error and can use Download. Drag files are private temporary files,
retained through an active drag and cleared when finishing. Cover letters use
the same export controls and an editable text popup. Answers retain generation,
editing, copying, and saved-answer collection. A generated answer can be refined
with instructions and is retained once when Reset question, a new browser
question capture, or Finish Application ends that question. Finish Application
saves the current resume, cover letter, and final answers to the tracker with
found role details and today's date. The pencil opens editable tracker details;
the red X confirms discarding the application without a tracker entry.

Codex enablement is the exclusive provider choice even while signed out. The
footer shows “No model connected”, with “Sign in to use Codex” in the header.
The status returns to
Ready when the account connects and AI is available.
Sign out keeps this choice; disabling Codex
restores the selected API key automatically without unpausing it.
