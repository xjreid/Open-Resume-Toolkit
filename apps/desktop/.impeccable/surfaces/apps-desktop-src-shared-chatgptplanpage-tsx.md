---
version: 1
slug: "apps-desktop-src-shared-chatgptplanpage-tsx"
primary_target: "apps/desktop/src/shared/ChatGptPlanPage.tsx"
related_targets: ["apps/desktop/src/shared/styles/chatgpt-plan.css","apps/desktop/src/shared/CodexAccountSettings.tsx","apps/desktop/src/shared/CodexRuntimeSetup.tsx"]
---

# Main app Codex connection

Mode: Operate. Scope: AI / Codex in the main desktop window. The user starts or stops a local Codex server, signs in through their browser, and manages model, reasoning, account usage, and a reserve. Preserve runtime verification, session cleanup, operation locks, and the established Open Folio system.

## Direction contract

THESIS: Make server operation and account authorization independently legible, with one place to refresh the connection.

OWN-WORLD: Inherit Open Folio's Hanken typography, teal actions, white paper, pale canvas, and fine rules. Use section spacing and a compact status strip rather than nested cards.

STORY: Check installation, install only when needed, explicitly start the server, then sign in. Connected users see their plan and inference controls before account-wide usage and the reserve.

FIRST VIEWPORT: A Codex heading with refresh and start/stop actions at the upper right. Directly below, dedicated server and account status fields. A separate account panel holds plan, sign-out, model, and reasoning. Usage and reserve share an asymmetric two-column section at roomy widths and stack at the desktop minimum.

FORM: Precisely specified local extension; code-led within the approved visual world. No concept seed applies. Signature interaction: start becomes stop, server status resolves before sign-in appears, and stopping removes account controls and restores the start action.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
