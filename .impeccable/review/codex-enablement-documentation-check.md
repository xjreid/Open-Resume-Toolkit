# Codex enablement documentation check

## Scope

This check covers the narrow Codex enablement extension in the main AI
workspace: the always-visible **Enable Codex** preference, provider exclusivity,
sign-in and runtime lifecycle, and the disabled My Keys state. The incumbent
Open Folio design and product authority remain unchanged.

## Evidence checked

- `PRODUCT.md`, `DESIGN.md`, and `.impeccable/design.json` for product scope,
  Open Folio visual authority, light-only UI, Hanken typography, and compact
  controls.
- `apps/desktop/src/shared/ChatGptPlanPage.tsx` for the always-visible toggle,
  saved-preference copy, runtime/login gating, sign-out behavior, and disabled
  runtime handling.
- `apps/desktop/src/shared/AiWorkspace.tsx` for Codex provider exclusivity,
  inert/grayed My Keys controls, the preserved active API key, and automatic
  resumption after disablement.
- `apps/desktop/src/shared/styles/chatgpt-plan.css` for the existing panel,
  checkbox, fieldset disabled treatment, and Open Folio spacing/typography.
- `docs/ai/codex-plan-runtime.md` for the saved profile-scoped permission,
  disabled-runtime guard, sign-out/app-quit lifecycle, exclusive routing, and
  API-key restoration semantics.
- `docs/application-behavior/application-overlay.md` for the overlay’s Codex
  labeling and provider transition behavior.

## Extension-versus-system verdict

**Ordinary extension; design authority preserved.** The enablement control is
part of the existing Codex settings panel and uses its established checkbox,
teal accent, helper copy, fieldset, and disabled opacity/grayscale treatment.
No new visual language or replacement surface is introduced. My Keys remains
present for orientation but becomes noninteractive while Codex is enabled;
the active API key remains saved and is restored as the selected provider when
Codex is disabled.

## Behavior consistency

The implementation and documentation agree on the requested behavior:

- Enable Codex is always rendered and its preference is saved independently of
  account sign-in or runtime installation.
- Disabled status reads and stale queued work cannot start the runtime; login
  and workflow acquisition recheck enablement under the runtime lock.
- While enabled, all AI requests route exclusively through Codex, including
  while signed out. My Keys is disabled, inert, and visibly grayed, with the
  explanatory “Codex must be disabled to use API keys” message.
- Sign out ends memory-only authorization but keeps Codex enabled. Quitting ORT
  ends the in-memory session while retaining enablement and saved model,
  reasoning, reserve, and activity settings.
- Disabling Codex signs out/stops the runtime and restores the selected API key
  without clearing a manual key pause.

## Documentation obligation

**Satisfied.** The lifecycle and provider boundary are documented in
`docs/ai/codex-plan-runtime.md`, and the overlay transition is documented in
`docs/application-behavior/application-overlay.md`. No behavior documentation
gap required correction, and no source, product, design, or existing review
artifact was changed by this check.

## Findings and limits

The evidence does not certify installed-app launch, live account sign-in,
native runtime process teardown, or real provider routing beyond the supplied
source and documentation. Existing detector/style drift in the incumbent
compact overlay and AI workspace remains outside this narrow handoff and was
not repaired.

