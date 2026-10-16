# ChatGPT page verification

The main-window AI tabs are My Keys, ChatGPT, and Data. The page keeps a
persistent ChatGPT brand header with installation status, launch or text-only
Quit, runtime state, and icon-only refresh. The Codex subtitle is removed.

Verification on the final source: 32 targeted Codex tests passed; TypeScript
and production build passed (existing bundle-size advisory); 26 synthetic
state/scroll captures at 1080×760 and 720×520 passed axe, overflow, page-error,
and sticky-header checks. The browser exercise covers launch, browser login
cancellation, quit, sign-out, model selection, and reserve edits. See
`verification.json` and `finish-review.md` for the evidence and independent review.

No live runtime, installation, provider request, or real account was exercised.
Existing sourced OpenAI SVG reused; no new shipping raster assets.
