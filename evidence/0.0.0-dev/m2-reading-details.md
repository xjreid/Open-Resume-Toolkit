# M2 reading and optional-detail checkpoint — 2026-09-06

Medium UI work on the M1-complete baseline. Step 5 remains incomplete and no
application was installed.

Selecting an entry heading in the live reading view opens that specific entry and
focuses its heading field, including the section's initially expanded entry.
Contact/section navigation and read-only publication/PDF semantics remain intact.
Optional custom fields and skill groups now sit behind native **Add optional
detail**, with a count and examples that are guidance only. Validation navigation
opens enclosing details before focusing a field. Existing optional content stays
visible in the reading view while its editing controls are collapsed.

The reading view omits blank bullets and custom-field values, removes dangling
link separators, and hides the editor-only skill marker. These are derived display
changes; source documents are untouched. Bullet controls wrap below their text so
duplication does not crowd the input into a narrow column.

Advisory matching-entry hints compare completed content across sections while
ignoring identity and surrounding whitespace. They identify exact matches rather
than claiming semantic duplicate detection. Empty entries are ignored; different
organizations or date precision remain distinct. Review buttons open the matching
entry. Hints neither rewrite nor merge data and do not block save/publication.
Import-specific duplicate decisions remain part of the gated import work.

`CI=true pnpm check` passed: 91 desktop tests, 24 contract tests, TypeScript,
formatting, builds, static security and licenses. Live checks cover entry selection,
keyboard focus, native details toggle, hidden-field error navigation, matching
hints and undo, plus axe accessibility. Unit checks cover non-mutating duplicate
comparison and empty-content rendering. Log: `target/m2-reading-details-web-check.log`.
No Rust, contracts, output templates, storage, dependencies or native boundaries
changed in this slice. Native WebView layout and VoiceOver acceptance remain open.

Next requires High reasoning: PDF/DOCX layout parity and final style qualification,
including versioned renderer/receipt compatibility and output correctness. The
first Step 5 subsection is not complete; sandboxed import and lifecycle/cleanup
work remain separately High-routed. Changes remain uncommitted and unpushed.
