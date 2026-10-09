# ChatGPT plan documentation check

## Scope

This evidence check covers the ordinary Operate-mode ChatGPT plan extension in:

- `apps/desktop/src/shared/ChatGptPlanPage.tsx`
- `apps/desktop/src/shared/styles/chatgpt-plan.css`
- `apps/desktop/src/shared/AiWorkspace.tsx`
- `apps/desktop/src/shared/ApplicationOverlay.tsx`

The check was performed against the incumbent Open Folio system documented in
`PRODUCT.md`, `DESIGN.md`, and `.impeccable/design.json`. The incumbent
system remains authoritative: Hanken typography, deep teal rail, pale canvas,
white paper panels, blue-grey rules, teal state accent, native controls, compact
spacing, and restrained elevation.

## Evidence checked

- `.impeccable/review/plan-finish-review.md` — disposition `ship`, visual
  contract, state coverage, accessibility notes, and review limits.
- `.impeccable/review/plan-detector.json` — three advisory 13px type findings
  in `chatgpt-plan.css`; no blocking detector finding.
- `.impeccable/review/plan-1080.png` and `plan-720.png` — default desktop and
  minimum-width settings states.
- `.impeccable/review/plan-bottom-1080.png` and `plan-bottom-720.png` —
  lower quota, reserve, runtime, and disconnect continuation.
- `.impeccable/review/plan-disconnected.png` — disconnected/sign-in state.
- `.impeccable/review/plan-data-1080.png` and `plan-data-720.png` — ChatGPT
  plan data filter and unpriced usage language.
- `.impeccable/review/plan-keys-1080.png` and `plan-keys-720.png` — paused
  active-key warning and return link.
- `.impeccable/review/plan-overlay.png` — 360px overlay treatment with the
  `Using ChatGPT plan` label and no model picker.

Source inspection confirmed that `AiWorkspace` owns the `general`, `plan`,
and `data` tabs and passes plan status into `ChatGptPlanPage`; the data view
preserves the plan filter and monetary-cost caveat. `ApplicationOverlay` uses
the plan connection source to show the plan label and routes plan errors through
the existing presentation helper.

## Documentation obligation

**Satisfied.** The finish review records the visual and interaction evidence,
the screenshot set covers the requested desktop, minimum, scrolled,
disconnected, data, keys-warning, and overlay states, and this file records the
evidence boundary and incumbent-system comparison. No implementation,
`PRODUCT.md`, `DESIGN.md`, or `.impeccable/design.json` changes were made.

## Findings and limits

The three 13px detector findings are advisory and match incumbent helper-label
usage; they do not warrant a design-system change for this scoped extension.
Any unrelated incumbent drift remains reported by the existing system artifacts
and was not repaired here. This documentation check does not certify installed
app launch, live ChatGPT/Codex connectivity, provider quota accuracy, or native
runtime behavior beyond the supplied source and headless Chromium captures.

## Post-review recheck

After the evidence capture, two narrow implementation changes were rechecked:
runtime-untrusted recovery copy now points to the protected runtime location
described on the page, and the existing official runtime guidance action now
invokes the dedicated no-argument native command
`open_chatgpt_plan_runtime_guidance`. The backend opens the same official
release page in the system browser. These changes affect security and recovery
wording/command routing only; no layout or style changed, so the supplied
captures remain applicable. The incumbent PRODUCT, DESIGN, and sidecar files
remain preserved.
