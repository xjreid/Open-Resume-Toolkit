# Manual overlay walkthrough for the unsigned local build

This is a user-run check of the Stage 1 and Stage 2 overlay. It does not require
a Chrome or Edge extension. Browser capture, native messaging, and browser file
drop are outside this walkthrough. Codex has not run this walkthrough or tested
the installed application.

## Start

1. Open `/Applications/Open Resume Toolkit Dev.app`. This is the installed
   ad-hoc-signed development build with the current overlay changes.
2. In the main window, prepare and publish a master resume. In Direct AI setup,
   add and test a provider key and select a working model. Generation requires a
   configured AI mode and a published master.
3. Open **Application workspace** from the main window. The overlay should be
   separate and always on top. Its header should identify the active AI mode
   and state that browser capture is unavailable in this unsigned preview.

## Stage 1: typed job description

1. Paste a short job description into **Reviewed job description**. Optionally
   add a source URL and choose a resume design. No AI request should start yet.
2. Close the overlay window, then reopen it with **Application workspace**.
   The reviewed job, URL, and design should still be there.
3. Choose **Continue and tailor resume**. The result should open the Resume
   tab. A failed AI request should leave the reviewed job available for retry.

## Stage 2: materials

1. Review the change points and any Required Qualification Alerts. Dismiss and
   reopen an alert if one appears; alerts should not prevent export or Finish.
2. Choose **Preview and edit** on the tailored resume. The overlay should grow.
   Change a contact field, section heading, or resume text. The PDF should say
   it shows the last saved version while the edit is pending. Choose **Save
   edits**; the PDF should refresh and remain beside the editor. Try **Compact
   window** and **Download**.
3. Open **Cover letter** and choose **Generate cover letter**. Preview it, edit
   its text, save, and confirm the PDF refreshes. Download it if desired.
4. Open **Answers**. Type a non-personal question, save the reviewed question,
   then choose **Generate answer**. Edit or copy the answer, add it to the
   answer set, save, and try **Reset and capture new question**. A personal or
   legal attestation question should be refused rather than drafted.
5. With saved Stage 2 work, close and reopen the overlay. The resume, cover
   letter, question, and approved answers should return. If you make an
   unsaved edit and request app quit, the quit dialog should call out overlay
   work; **Keep editing** should return to it.

## Finish and tracker

1. Choose **Finish Application**. Review the material summary and the
   retention checkboxes. Enter any known company, title, date, status, and
   source URL; blank tracking fields are allowed.
2. Choose **Save to tracker and finish**. The overlay should reset to Stage 1.
   In the main window, open **Application tracker**, find the entry, and reopen
   any retained resume, cover letter, and approved answers.
3. Optionally repeat with **Finish without saving**. It should clear the
   temporary workspace without creating a tracker entry.

Record the launch method, macOS version, AI provider/model, each step's result,
and any visible error text. Do not include API keys or private resume content
in a shared issue report.
