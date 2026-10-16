verdict

- resolved — `stopped` now shows the text-only `Launch` action at both 1080×760 and 720×520; the launch control no longer carries an icon, while the running state retains the distinct red `Quit` action and refresh icon.
- resolved — Connected and stopped captures show the same shared 852px content width at 1080 and 608px width at 720, with the server frame and account/model sections aligned across state changes.
- resolved — The source uses explicit full-width/minimum track constraints for the shared panels and a stable scrollbar gutter, preventing signed-in, account, reserve, and stopped layouts from shifting.
- resolved — Final verification reports all 26 captures with consistent measured widths, zero axe violations, zero overflow, no page errors, reserve-disable coverage, 32 targeted tests, and passing typecheck.

remaining

clear — No regression is visible in the four final captures, and the requested follow-up behavior is implemented and verified.

disposition: ship
