# Codex runtime installer documentation check

## Scope

This handoff covers the optional, user initiated post-install runtime setup
inside the existing ChatGPT plan page:

- `apps/desktop/src/shared/CodexRuntimeInstaller.tsx`
- `apps/desktop/src/shared/ChatGptPlanPage.tsx`
- `apps/desktop/src/shared/styles/chatgpt-plan.css`
- `apps/desktop/src/shared/chatgpt-plan-presentation.ts`
- `docs/ai/codex-plan-runtime.md`

The comparison authority is the incumbent Open Folio system in `PRODUCT.md`,
`DESIGN.md`, and `.impeccable/design.json`: Hanken typography, deep teal
rail, pale canvas, white bordered panels, blue-grey rules, teal active state,
native controls, compact spacing, and restrained elevation. The main installer
menu remains unchanged.

## Evidence checked

- `.impeccable/review/installer-finish-review.md` — disposition `ship`,
  visual contract, product/state coverage, accessibility, and limits.
- `.impeccable/review/installer-detector.json` — three advisory 13px findings
  in the incumbent plan stylesheet; no blocking finding.
- The eight supplied plates:
  `.impeccable/review/installer-missing-1080.png`,
  `installer-missing-720.png`,
  `installer-download-1080.png`,
  `installer-download-720.png`,
  `installer-approval-1080.png`,
  `installer-approval-720.png`,
  `installer-error-1080.png`, and
  `installer-error-720.png`.
- Source and documentation inspection confirmed the opt-in installer is
  subordinate to plan connection, reports progress and cancellation, names
  administrator approval, keeps retry/recovery adjacent to errors, and does
  not claim account login, plan enablement, or automatic installation.
- Reported UI tests, pinned download verification, and contained signed-out
  startup checks passed. No runtime installation, browser login, inference, or
  app launch was performed.

## Incumbent comparison and drift

The installer uses the existing plan panel and inherits its Open Folio reading
order, rules, controls, focus treatment, and responsive stacking. It does not
introduce a second installer entry point, new colors, new typography, or a
separate surface language.

The only recorded drift is pre-existing helper-label sizing: the detector flags
13px at the existing stylesheet lines 52, 134, and 160. These findings are
advisory and were not expanded into a type-system change for this scoped
extension. No other pre-existing drift was repaired or reclassified here.

## System documentation decision

**No system documentation changes are needed.** The installer is an ordinary
extension of the already documented ChatGPT plan surface, and
`docs/ai/codex-plan-runtime.md` records the qualified release, protected
installation rules, installer boundaries, containment assumptions, and
verification evidence. `PRODUCT.md`, `DESIGN.md`, and
`.impeccable/design.json` remain preserved.

## Documentation obligation

**Satisfied.** This report records the implementation boundary, incumbent
comparison, screenshot evidence, advisory pre-existing drift, verification
scope, and explicit limits. The evidence supports shipping the optional
post-install setup while leaving live installation, account, inference, and
app-launch behavior for user initiated checks.
