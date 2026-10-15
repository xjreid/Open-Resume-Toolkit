---
version: 1
slug: "apps-desktop-src-shared-chatgptplanpage-tsx"
primary_target: "apps/desktop/src/shared/ChatGptPlanPage.tsx"
related_targets: ["apps/desktop/src/shared/styles/chatgpt-plan.css","apps/desktop/src/shared/CodexAccountSettings.tsx","apps/desktop/src/shared/CodexRuntimeSetup.tsx"]
---

# Main app Codex connection

Mode: Operate. Scope: AI / Codex in the main desktop window. The user starts or stops a local Codex server, signs in through their browser, and manages model, reasoning, account usage, and a reserve. Preserve runtime verification, session cleanup, operation locks, and the established Open Folio system.

## Direction contract

THESIS: Make the everyday Codex controls easy to scan, with short guidance and detailed help available on demand. Keep server operation and account authorization independently legible.

OWN-WORLD: Inherit Open Folio's Hanken typography, teal actions, white paper, pale canvas, and fine rules. Use one white workspace, section spacing, and a compact status strip. Avoid separate cards for each settings group.

STORY: Check installation, install only when needed, explicitly start the server, then sign in. Connected users see their plan and inference controls before account-wide usage and the reserve.

FIRST VIEWPORT: A Codex heading with Refresh and Start server / Stop server actions at the upper right. Directly below, dedicated server and account status fields; plan and Sign out live with the account status. Model and reasoning controls follow without a repeated heading. Account usage and a reserve switch share an asymmetric two-column section at roomy widths and stack at the desktop minimum. Connection & privacy and How the reserve works open native disclosures.

FORM: Precisely specified local extension; code-led within the approved visual world. No concept seed applies. Signature interaction: start becomes stop, server status resolves before sign-in appears, and stopping removes account controls and restores the start action.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Verification

October 12, 2026: 76 focused component tests, TypeScript, and the production web build passed. The mock browser verification covered eight states at 1080 × 760 and 720 × 520, with 22 captures, no horizontal overflow, no page errors, and no WCAG A/AA axe violations. Keyboard disclosure and reserve-switch behavior passed. Captures and the inline finish review live in `.impeccable/review/codex-page/`. No real runtime, installer, or account was used.
