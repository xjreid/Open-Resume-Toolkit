# M2 focused-editor layout checkpoint

Date: 2026-09-06. Baseline: M1-qualified `65518eb` and the local Step 5 changes.
Status: local frontend implementation; M2 is incomplete and native qualification
is pending. M1 remains complete for its recorded macOS-arm64 development scope.

The editor now has a collapsible section navigator, continuously updated full
resume reading view and a focused panel for contact or one section. Selecting a
section in navigation or its heading in the reading view opens its fields. The
focused panel receives keyboard focus; Close editor removes it and returns focus
to the reading heading. Section creation opens the new section. Contact remains
the initial focus, including the existing first-build Full name behavior.

Reading zoom affects only the HTML reading view. The selected style applies
approximate typography/spacing to that view; its description explicitly states
that it includes unsaved edits and that exact pages/fonts/export layout belong
to PDF preview. This is not continuous exact PDF pagination or output parity.
The reading view keeps the full resume visible while another section is edited,
and internal document titles and empty entry headings are omitted there.

Desktop layout places navigation, reading view and focused fields side by side;
narrow layouts stack them. Section/date/link editing, upgrade controls and existing
save/validation behavior are reused. The view renders text only, and stored links
remain non-navigating in the privileged interface. No native commands, contracts,
backend persistence or dependencies changed in this slice.

Validation: `CI=true pnpm check` passed, including 81 desktop tests, 23 contract
tests, TypeScript, formatting, builds, static security and licenses
(`target/m2-focused-editor-web-check.log`). The new live test verifies section
selection, focused-panel keyboard focus, immediate draft reading updates, Close
focus return, selecting content from the reading view, navigator collapse, no
implicit PDF render/publication and axe accessibility. Existing start/date/link
workflow tests were updated to navigate the focused forms. Native WebView layout,
zoom, screen-reader behavior and visual style qualification remain pending.

Next Medium work: finer entry-level focus/collapse, complete add/duplicate/remove
conveniences with confirmations, and navigation to fields needing correction.
Final style layouts, exact continuous document pagination, historical regeneration,
sandboxed import, lifecycle completion and native M2 acceptance remain outstanding.
Do not mark Step 5 complete. Nothing was installed, committed or pushed, and no
test-account switch or Keychain access was required.
