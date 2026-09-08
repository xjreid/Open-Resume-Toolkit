# M2 entry editing checkpoint — 2026-09-06

Medium UI work on the qualified M1 baseline. M1 remains complete; M2 Step 5
remains incomplete. The installed application remains the M1 build.

Section forms now expand one entry at a time while the reading view keeps all
content visible. Add and duplicate open the new entry and focus its toggle.
Duplication preserves factual text and date precision, creates fresh IDs for all
identified nested entities, and inserts immediately after the source entry.
Legacy entries retain their original date/link shape. Existing normalization
restores order, and count limits disable duplication when it would exceed them.
The existing document validator still enforces total character and byte limits.

Entry and section removal require an inline confirmation. Keep it restores focus
to the removal button. Confirmed entry removal returns focus to the section field;
section removal opens the section manager. Existing undo restores removed content.
No schemas, native commands, persistence, dependencies or output contracts changed.

Validation: `CI=true pnpm check` passed with 84 desktop tests and 23 contract tests,
including content/identity independence, legacy shape preservation, live collapse
and duplication, removal cancellation/confirmation, focus, undo and axe checks.
TypeScript, builds, formatting, static security and license checks also passed.
Log: `target/m2-entry-editing-web-check.log`. An initial static check rejected a
URL fixture under production source; the test was moved into the test directory.
Native WebView layout and screen-reader qualification remain pending.

Next Medium work includes navigation to fields needing correction. Exact document
pagination, final style qualification, import, lifecycle completion and native M2
acceptance remain open. Nothing was installed, committed or pushed; no test-account
switch or Keychain access was needed.
