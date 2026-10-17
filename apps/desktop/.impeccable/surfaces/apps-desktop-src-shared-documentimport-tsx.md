---
version: 1
slug: "apps-desktop-src-shared-documentimport-tsx"
primary_target: "apps/desktop/src/shared/DocumentImport.tsx"
related_targets: ["apps/desktop/src/shared/AiImportPanel.tsx","apps/desktop/src/shared/styles/import.css","apps/desktop/src/shared/ai-import.ts"]
---

# Resume import

Scope: base import page only; review, mapping, app shell, and provider settings remain unchanged. Mode: Operate. Job seekers choose a local or AI file import, then review before saving.

## Direction contract

THESIS: One quiet, rounded import workspace with two directly comparable methods and only decision-relevant copy.

OWN-WORLD: Inherit Open Folio's Hanken typography, teal actions, white surfaces, mist canvas, fine rules, and 12px panel corners. Keep all styling scoped to import.

STORY: Read the title, choose local parsing or AI, see provider readiness, then select a file. Errors explain the problem in one plain statement.

FIRST VIEWPORT: A centered white panel has a new upload-document icon beside “Import your resume.” Below, Parse Locally sits left and Import with AI right, separated by one rule. Provider, model, and reasoning form a compact aligned group. Matching file buttons sit at the bottom of each column. One shared review reassurance closes the panel.

FORM: User-pinned two-column composition; no concept seed applies to this precisely specified request. Equal method emphasis unless the user steers otherwise. No raster assets or comp needed for the pinned local redesign.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Implemented surface note

The shipped base import view matches this contract in `DocumentImport.tsx`, `AiImportPanel.tsx`, `ai-import.ts`, and scoped `styles/import.css`: a centered rounded white panel with the upload-document SVG and “Import your resume”, equal local/AI columns, a green readiness dot with provider/model/reasoning metadata, matched file actions, and one shared review reassurance. Errors remain one plain statement; no “Check connection” action is present. Desktop and default captures in `.impeccable/review/import/` are the visual evidence. Existing advisory drift is intentionally retained: 13px type is used across the app, `#fff2f2` is already a controls token, the 23px mobile title is intentional, and review styling keeps its legacy 3px rule and `#d2dae3`.
