# M2 section-specific entry guidance

Date: 2026-09-07. Source baseline: M1-complete `65518eb`, with the current
uncommitted Step 5 changes. Routine editor work performed on Medium.

Education, projects, skills, certifications/licenses, awards/honors, research
and publications now have contextual heading/subheading labels and example
placeholders. Other section names retain the general form. Guidance depends
only on the visible section heading; renaming a section never rewrites existing
entries or optional details.

The optional-detail disclosure now offers appropriate suggestions, such as GPA,
coursework, technologies or credential ID. Selecting one adds a normal editable
label with a blank value and focuses the value. The custom-field action still
adds a blank label/value and now focuses the label. Skill suggestions use the
existing skill flag and respect the document-wide skill limit. No example
values, new schema, migration, IPC command or inferred factual content are added.
The existing stable IDs, ordering, undo, validation and autosave remain in use.

## Verification

- The live regression exercises the education form, blank examples, suggested
  GPA, focus, undo/redo identity preservation, renaming to a custom section,
  saved payload and unchanged input fixture. Axe reports no semantic findings.
- A synthetic Chrome fixture verified the expanded picker, blank-value focus
  and narrow focused-panel layout. Its backend was mocked; no installed app,
  Keychain or user profile was opened. The temporary tab and server were closed.
- `CI=true pnpm check` passed: 102 desktop tests, 24 contract tests, the remaining
  workspace tests, TypeScript, builds, static security and license checks
  (728 Rust / 167 JavaScript packages). `git diff --check` passed. Full log:
  `target/m2-entry-guidance-check.log`.

This improves manual-entry usability. It does not qualify native WKWebView,
VoiceOver, installed quit/interruption behavior, final document styles or import
containment. Those M2 release gates remain open; hosted CI is pending for this
uncommitted checkpoint.
