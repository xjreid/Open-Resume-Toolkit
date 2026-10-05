# UI fixes layout review

## Scoped verdict

The correction addresses the reported structure at both supported captures. The editor workflow bar now reaches `y=0` after scrolling, including the 720×520 minimum (`after-geometry.json`: `editorScrolled.navigation.y = 0`, `minimumEditor.navigation.y = 0`). This removes the prior 68px scrolled gap and the minimum viewport state where the bar was at `y=-149` and unavailable.

The entry hierarchy now reads as requested: Title and Skills / details share the first row, Role follows beneath the title, and Date, Location, and Extra form a right-aligned vertical stack. The post-fix geometry confirms the stack at 30px intervals and preserves a 410px information column at the minimum width. Empty Date now uses the same 11px Hanken/muted treatment as the other empty fields instead of the resume serif style.

The focused editor controls are materially clearer in the after captures: visible `Bold`, `Italic`, `Link`, and `Done` labels replace the ambiguous icon-only toolbar, the separate `Clear text` action is explicit, and the autosave message is visible. The information editor also keeps its `Text layout` control grouped with the editing tools. Smoke verification reported successful bold, italic, link, selection restoration, clear, and Escape behavior.

## Final residual check

Both residual findings are resolved. The minimum-width Details and Location captures now retain their field labels in the focused toolbar, and the Details, Location, and Information focused states keep the complete toolbar, input, and autosave/clear footer inside the 720×520 viewport. The active-field scroll behavior uses the sticky-bar offset and is guarded for no-layout test environments. `after-geometry.json` reports all eight interaction groups passing with `errors: []`.

The AI Keys/Data after captures place their 54px tab strip at `y=61`, matching the preview-only banner offset while the page heading is scrolled away. This is intentional surface-specific behavior and does not indicate a remaining sticky gap.

The body-mode smoke script's selector mismatch is test-harness-specific: the native label includes the option text, and it does not indicate a product defect.
