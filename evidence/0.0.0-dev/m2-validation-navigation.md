# M2 validation navigation checkpoint — 2026-09-06

Medium UI work on the M1-complete baseline. M2 Step 5 remains open.

Field-specific validation messages now name their contact/section/entry location
and open the matching editor. Collapsed entries expand and the exact field receives
keyboard focus. Date errors focus the invalid endpoint within the date record.
Repeated jumps work, and normal navigation clears pending validation focus.
Document-wide size/count messages remain descriptive because they have no single
field to correct. Existing validation and save blocking are unchanged.

Bullets now support duplication immediately after their source, with a fresh ID,
canonical order and the same text. The existing bullet count limit applies and
undo removes the duplicate. No native commands, contracts, stored schema or
renderer behavior changed.

Validation: `CI=true pnpm check` passed with 85 desktop tests and 23 contract tests,
plus TypeScript, formatting, builds, static security and license checks. The new
live workflow exercises repeated navigation, closed/collapsed editors, invalid
date focus, ordinary navigation after correction jumps, bullet duplication/undo
and axe accessibility. Log: `target/m2-validation-navigation-web-check.log`.
Native layout and VoiceOver qualification remain outstanding.

Next: historical-output regeneration requires High reasoning before changing
native command/source/receipt handling. Further style layout, pagination, import,
lifecycle and native qualification work remains. Nothing was installed, committed
or pushed, and no test-account switch was needed.
