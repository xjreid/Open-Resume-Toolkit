# Application overlay

The application workspace opens as a fixed 360 × 760 logical-pixel rail at the
left edge of the monitor work area, vertically centered. Its height is clamped
on smaller displays. Drag the header to move it; the rail cannot be resized.
The model preset selector uses the active key's signed catalog choices and the
same preset-setting command as My Keys. The header shows request activity and
lets the user cancel overlay generation.

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

Tailor requires an active AI key, a published master resume, and a nonempty job
within the existing input limits. It saves reviewed job details before starting
the request. No live API request is sent just by capturing or editing text.

## Review, edit, export

Stage 2 shows the company/role, Finish Application, and Resume, Cover letter,
and Answers tabs. The resume tab contains tailoring notes and qualification
alerts, a style selector, PDF/Word format selection, Download and Drag me,
View/Edit controls, and an AI refinement input.

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
editing, copying, and approved-answer collection. Finish retains the existing
tracker and material-selection workflow.
