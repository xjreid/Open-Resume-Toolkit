# Finish review: Codex runtime installer

## 1. Disposition

**ship**

The supplied 1080×760 and 720×520 captures show a narrow, opt-in extension of
the existing ChatGPT plan page. The installer remains subordinate to account
connection, keeps the main app closed, and does not introduce a second installer
entry point or a new visual system.

## 2. Visual contract

The deep teal rail, pale canvas, white bordered plan panel, Hanken hierarchy,
muted slate copy, teal active tab, and restrained rules match Open Folio and the
incumbent plan surface. The missing, downloading, administrator-approval, and
failure states preserve the same reading order and spacing. At 720px the
scrolling app pane keeps the runtime copy, progress, retry/cancel controls, and
details disclosure usable without horizontal overflow; the clipped leading
content in the 720px plates is consistent with the supplied scrolled fixture.

## 3. Product and state coverage

The missing state clearly explains that installation is optional, pins the
qualified Codex release and approximate download size, and states that install
does not connect an account or enable plan usage. Downloading exposes native
progress and cancellation. Approval explicitly names the macOS administrator
prompt and the protected-copy verification boundary. Failure keeps the retry
action adjacent to a concrete recovery message. The component and fixture
evidence cover cancellation, unsupported platform, blocked states, completion,
and recoverable installer errors; the native implementation context supports the
claims about pinned download, verification, approval, and atomic protected copy.

## 4. Accessibility and interaction

The install, retry, cancel, refresh, and details controls are native keyboard
controls with visible Open Folio focus styling inherited from the plan surface.
The progress element has an accessible label, and status/error copy uses
`role="status"` or `role="alert"` according to state. Busy and login/operation
locks prevent competing actions while preserving the current page context.
The approval state correctly directs the user to the OS prompt instead of
pretending the page can authorize it. Error copy names the problem and gives a
safe retry or refresh path; no state implies automatic account login or
enablement.

## 5. Findings and limits

No material finish defect requires a fix, rebuild, or recapture. The detector's
three 13px findings are advisory and pre-existing helper-label usage; they do
not justify changing the shared type ramp for this scoped extension. Review is
limited to the eight supplied screenshot plates, the existing plan baselines,
the changed UI sources, detector output, and reported fixture/component checks;
it does not certify a live macOS prompt, network download, installed-app launch,
or provider behavior.
