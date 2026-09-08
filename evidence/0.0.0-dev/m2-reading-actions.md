# M2 reading-view entry actions

Date: 2026-09-06. Medium-reasoning editor work on the uncommitted M2 development
tree based on qualified M1 `65518eb`. Step 5 remains incomplete.

The live reading view now offers an add-entry button beside each section.
Adding opens the relevant focused editor and places keyboard focus in the new
entry's heading. It uses the existing draft edit/normalization and undo pipeline,
observes the global entry limit and busy/reload state, and does not publish.
Untitled entries have a direct edit button so their existing content remains
reachable without first opening the section editor. These controls and labels
are absent from the published snapshot and PDF text views and never become
stored factual content.

Collapsing section navigation now releases its grid column to the reading view.
The show-sections control remains reachable above the document. Wide layouts
retain a neighboring focused editor; narrower layouts put it below the document.
Empty reading-view paragraphs and list containers no longer reserve whitespace.
These CSS changes do not change the PDF/DOCX renderer or its output hashes.

Validation: 92 desktop tests pass, including direct creation, heading focus,
untitled-entry reopening, undo/redo, source nonmutation, read-only control
exclusion and the existing accessibility scan. Full repository checks are
recorded in `target/m2-reading-actions-check.log`; focused test output is in
`target/m2-reading-actions-tests.log`. Native WKWebView layout/keyboard and
VoiceOver qualification remain pending. No app was installed or profile changed.

The center still truthfully identifies itself as a live HTML reading view.
Integrating native-backed exact pagination with draft/revision identity,
cancellation and stale-result handling requires High reasoning. It is not
completed by these UI changes. Import also remains disabled pending containment.

## Subsequent workspace presentation checkpoint

After the separate High exact-page integration in `m2-center-pdf-preview.md`,
Medium presentation work moved backup/recovery/storage panels below the editor.
They remain mounted and expanded, with their existing state and safeguards.
Top-level Resume editor and Backup and recovery links target focusable headings.
Text/Word export controls and their destination notice sit inside a native
details disclosure; the style selector remains visible. No command, storage,
export, or recovery behavior changed.

The full repository gate passed with 101 desktop tests; see
`target/m2-workspace-layout-check.log`. A local synthetic browser fixture verified
the editor-first visual order, heading focus for both shortcuts, and opening the
export disclosure. The fixture used mocked native commands; no installed app,
profile, backup, or export was changed. The temporary tab/server were closed.
Installed accessibility and native quit/interruption qualification remain open.
