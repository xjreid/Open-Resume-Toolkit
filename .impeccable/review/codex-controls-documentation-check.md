# Codex controls documentation check

## Scope

This evidence check covers the ordinary Operate-mode extension that adds Codex
connection labeling, model and reasoning controls, and known account-wide
remaining usage to the fixed 360 × 760 application overlay. It also covers the
main ChatGPT plan page title terminology change to Codex. The incumbent Open
Folio system remains authoritative; this is a narrow extension of the existing
surface rather than a new visual world.

## Evidence checked

- `PRODUCT.md`, `DESIGN.md`, and `.impeccable/design.json` — product, visual,
  and token authority.
- `docs/application-behavior/application-overlay.md` — fixed rail geometry,
  provider controls, disabled/working states, synchronization, and omission of
  unknown quota.
- `docs/application-behavior/resume-tailoring.md` — provider/model boundary,
  overlay progress, and request behavior.
- `docs/ai/codex-plan-runtime.md` — qualified runtime and plan terminology.
- `apps/desktop/src/shared/ApplicationCodexControls.tsx` and
  `ApplicationOverlay.tsx` — provider branch, selectors, fallback options,
  quota rendering, lock state, and existing API-key selector preservation.
- `apps/desktop/src/shared/application-overlay.css` — incumbent overlay
  spacing, typography, native select treatment, status colors, and compact
  quota rows.
- `apps/desktop/src/shared/use-chatgpt-plan.ts`, `ChatGptPlanPage.tsx`,
  `AiWorkspace.tsx`, and `chatgpt-plan-presentation.ts` — plan state boundary,
  page title terminology, refresh/save behavior, and Codex-facing copy.
- `.impeccable/review/codex-controls-overlay.png` — connected Codex controls
  with reported remaining windows.
- `.impeccable/review/codex-controls-overlay-api.png` — API key branch,
  confirming the existing model selector remains in place.
- `.impeccable/review/codex-controls-overlay-unknown.png` — connected state
  with unknown quota omitted.
- `.impeccable/review/codex-controls-overlay-busy.png` — locked controls and
  active request status.
- `.impeccable/review/plan-1080.png` and `plan-720.png` — main Codex plan page
  at the default and minimum supported workspace sizes.

## Extension-versus-system verdict

**Ordinary extension; incumbent design preserved.** The overlay keeps the
Open Folio white header, Hanken compact labels, blue-grey rules, native select
controls, restrained status color, and fixed rail geometry. The new provider
label and Codex controls occupy the existing connection region. API-key model
selection remains on its original path. Codex model changes preserve reasoning
when supported and otherwise choose the first supported level; unsupported
options remain visibly disabled. The same settings boundary and event path
update the main window and overlay. Quota rows render only reported windows,
including zero; absent usage produces no invented value.

## Documentation obligation

**Satisfied.** The requested behavior is represented in the application overlay
and resume-tailoring references, with Codex runtime qualification in the AI
runtime reference. The supplied captures cover connected, API-key, unknown
quota, busy/locked, default desktop, and minimum-width plan states. No UI
source, `PRODUCT.md`, `DESIGN.md`, `.impeccable/design.json`, or existing
artifact was changed by this check.

## Findings and limits

The one mechanical detector pass reports advisory design-system drift across
the incumbent overlay stylesheet: compact 9–23px helper/control sizes, several
small radii, and state colors outside the documented core palette. These values
are part of the dense fixed overlay and existing status language; they do not
indicate a Codex-specific visual break, so they were recorded rather than
repaired. The evidence does not certify installed-app launch, live Codex
connectivity, quota accuracy from a real account, native event transport, or
runtime security beyond the supplied source, docs, and captures.

No material documentation gap was found for this scoped UI extension. Future
changes to quota semantics, runtime qualification, or provider synchronization
should update the corresponding behavior/runtime references alongside code.

