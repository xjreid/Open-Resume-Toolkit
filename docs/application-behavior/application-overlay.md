# Application overlay

The application workspace opens as a fixed 360 × 760 logical-pixel rail at the
left edge of the monitor work area, vertically centered. Its height is clamped
on smaller displays. Drag the header to move it within the monitor work area;
the rail cannot be resized.
The header shows request activity and a red Stop text control while a request
is running. The native red window control requests an app quit and shows the
existing confirmation for unsaved work. Opening the overlay leaves the main
window visible; the native yellow control minimizes only the overlay.

## Provider controls

The fixed top banner identifies the active connection as **API key** or **Codex**.
API key connections show the active key's model selector. Connected Codex accounts show
model and reasoning selectors using the qualified runtime's available options;
unavailable options remain disabled. Selecting a model preserves the current
reasoning when supported, otherwise selects the first supported level. Changes
are saved through the same settings boundary as the main app and broadcast to
both windows immediately. Controls are locked during AI work and settings saves.

When Codex reports account-wide remaining usage, the banner shows each reported
window and remaining percentage, including zero. Unknown usage is omitted. Usage
refreshes on opening the overlay, every 30 seconds, and after AI work finishes.

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
banner keeps showing Codex, with “AI is disabled until an account is connected.”
instead of model/reasoning controls. Remaining usage and API key selectors are
also hidden while signed out. Controls return when the account connects.
Sign out keeps this choice; disabling Codex
restores the selected API key automatically without unpausing it.
