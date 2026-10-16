# Codex page documentation check

This check follows the Impeccable Documenter contract and records the shipped
Codex page as implemented. It checks `PRODUCT.md`, the Codex paragraph in
`DESIGN.md`, `.impeccable/design.json`, `.impeccable/surfaces/codex.md`,
`ChatGptPlanPage.tsx`, `CodexAccountSettings.tsx`, `CodexPageIcon.tsx`, and
`styles/chatgpt-plan.css`, together with the scoped behavior documents.

Documented structure: the **ChatGPT** navigation page keeps Codex runtime
terminology inside the page without a separate Codex subtitle. Its persistent header shows the
OpenAI/ChatGPT mark, installation state, runtime status, Refresh connection,
and Launch or Quit. A running signed-out runtime
shows the large white Log in with OpenAI control. A connected account shows
Model, Reasoning, Account and plan, Sign out, Usage limits, and Usage reserve;
usage and reserve share a row at roomy widths and stack at the supported
minimum. Detailed connection, installation, privacy, and reserve explanations
remain in native keyboard-accessible disclosures.

Evidence recorded for this pass: 32 targeted tests passed; TypeScript and
production build passed; 26 synthetic screenshots across Codex states and
desktop sizes passed overflow, page-error, and axe checks. Persisted verification
confirms all panels stay 852px wide at 1080px and 608px wide at 720px, with a
stable scroll gutter while visible. Account, sign-in, reserve, and reserve-toggle
states preserve those widths. Evidence is stored under this directory and in
`/private/tmp/ort-codex-ui-verification.json`.
The captures use existing Open Folio and sourced OpenAI SVG assets; no raster
asset or runtime behavior was changed.

System summary:

- Palette: inherited Open Folio deep teal, pale canvas, white paper, blue-grey rules, and semantic status colors; black OpenAI branding is the user-directed mark treatment.
- Type ramp: Hanken Grotesk; group headings 16px, values 14px, and supporting labels 12px; the broader system display remains 28px.
- Layout: persistent branded runtime header, centered login state, then model/account/usage sections in a width-100%-capped-960px shell; minmax grid tracks prevent intrinsic content expansion, and usage/reserve columns stack at the minimum.
- Depth and shape: flat white workspace, restrained rules, inherited 12px panel corners, 40px controls, and no new global tokens.
- Named rules: Teal Anchor, Quiet Type, and Flat-by-Default remain the applicable system guardrails.

Drift not canonized or repaired: stale references to Start server and Codex as
the navigation label were updated to Launch and ChatGPT; stale finish-review
claims were not used as approval evidence. Native installer,
runtime, and live account behavior remain outside this UI documentation check.
