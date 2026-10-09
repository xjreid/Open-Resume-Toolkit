# Finish review: Codex enablement

## 1. Disposition

**ship**

The narrow enablement extension is visually and behaviorally ready at this
scope. No material fix, rebuild, or recapture is required.

## 2. Visual contract

The captures preserve the incumbent Open Folio system: deep teal rail, pale
canvas, white bordered workspace, Hanken hierarchy, restrained rules, and
compact native controls. The Codex page is clearly surfaced at the top of AI &
monitoring, and the 720px capture remains readable without horizontal overflow.
The signed-out overlay keeps the fixed Codex label, model, and reasoning controls
in the same 360×760 composition.

## 3. Product and state coverage

The supplied states cover the always-visible saved Enable Codex preference,
connected and signed-out plan states, bottom sign-out behavior, disconnected
runtime guidance, and the API-key restoration path. The My Keys captures show a
clear “Codex must be disabled to use API keys” message, an inert/grayed key
surface while enabled, and the saved active key retained for restoration. The
signed-out overlay identifies Codex and exposes no API model picker.

## 4. Accessibility and interaction

The enable control has an explicit action label and explanatory copy. The
disabled My Keys surface preserves structure and communicates why actions are
unavailable. Native buttons, checkbox, selects, status messaging, and disabled
states remain keyboard-compatible. Browser verification covered sign-out while
retaining Enable Codex, disabling to restore keys, inert controls, and narrow
layout behavior. The 290 UI tests pass.

## 5. Findings and limits

No material finish defect requires a fix or recapture. OAuth account wording
such as “Connect ChatGPT account” remains accurate authentication copy and is
outside the requested Codex plan-title rename. Review is limited to the supplied
synthetic native fixtures and browser verification; it does not certify live
account authentication, runtime launch, or provider inference.
