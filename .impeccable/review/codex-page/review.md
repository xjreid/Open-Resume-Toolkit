disposition: ship

Finish review performed inline; no independent subagent was used. This is a code-led refinement of the existing Open Folio surface, with no new concept comp or shipping raster asset.

## persistence

Pass. PRODUCT.md and DESIGN.md retain the established palette, typography, product behavior, and local ownership constraints. The Codex surface brief now records the simplified layout and progressive disclosure.

## fidelity

| Element | Result | Evidence |
| --- | --- | --- |
| Type | Match | Hanken, 20px surface title, 16px group headings, 14px values, 12px help. |
| Material | Match | Flat white workspace, fine rules, native form controls; no invented physical effects. |
| Ground | Match | Existing pale canvas and teal rail are preserved. |
| Connection status | Match | Server and account remain separate; account plan and Sign out share the account row. |
| Everyday controls | Adaptation | Model labels replace the repeated account/settings heading; usage and reserve move higher. |
| Detailed help | Adaptation | Native disclosures retain session, privacy, installation, and exact reserve behavior. |
| Responsive layout | Match | Usage and reserve stack at the desktop minimum; all content remains reachable through the existing scroll area. |
| State handling | Match | Start/stop, login/cancel, missing runtime, unavailable usage, runtime error, and operation locks remain functional. |

## ceiling

Reached for this scoped simplification. Screenshots were inspected in one initial batch and one confirmation batch, including 1080 × 760 and 720 × 520. The supported desktop minimum deliberately scrolls.

## material_fixes

None remaining. The confirmation captures resolve the oversized secondary headings and redundant settings heading found in the initial inspection. The design detector reports no findings.

## keep

Keep brief labels, independently readable connection states, visible transmission guidance, usage meters, the reserve switch, and keyboard-accessible detailed help.

## Verification and capture provenance

- 76 focused Codex and AI workspace tests passed.
- TypeScript and the production web build passed. Vite retains its existing large-chunk warning.
- `tools/ui-revamp/verify-codex-page.mjs` exercised eight states at both supported desktop sizes, producing 22 captures without horizontal overflow, page errors, or axe WCAG A/AA violations.
- Browser checks covered keyboard disclosure open/close, reserve enable/disable, model saving, reserve saving, sign-out, sign-in cancellation, server start/stop, and operation locks.
- PNGs in this directory were captured on October 12, 2026 from the development-only preview with synthetic, memory-only native responses. They use the app's existing font and brand assets. No live account, runtime, or native installer was used.
- No measured usability or live account integration claim is made by this UI review.
