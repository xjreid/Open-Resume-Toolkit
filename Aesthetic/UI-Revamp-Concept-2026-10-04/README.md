# Open Resume Toolkit — UI revamp concept

Created October 4, 2026 using the frontend-design and impeccable skills.
This is a design proposal with synthetic data, not an implemented app revamp.
Application code, features, existing visual plans, and current logo assets are
unchanged. The screenshots are rendered from isolated HTML so labels are crisp
and the controls can be traced to the current implementation.

Open [the gallery](source/index.html) in a browser to review the six screens and
logo. It also includes the existing Storage and deletion and Browser connections
sections as supplemental mockup states. Mockup controls do not call providers or
modify the native application. Only screen navigation, version indication, and
settings disclosures are interactive demonstrations.

## Current UI evaluation

The evaluation uses current source and six current-screen captures in `review/`,
rendered with memory-only synthetic fixtures. Older milestone screenshots were
also inspected, but they differ from current implementation and did not determine
the final feature inventory.

| Current observation | Proposed visual change | Why it helps |
| --- | --- | --- |
| Editor borders and tiny labels surround nearly every field. | Keep document text quiet; emphasize the active inline field and optional-field affordances. | The user can scan the resume while still recognizing edit targets. |
| Main destinations and page modes both look like similar buttons. | Move main destinations to a persistent rail; use a compact mode switch and clearly scoped tabs. | Global navigation and local editing choices have distinct hierarchy. |
| Saved/published selection exists, but its relationship to tailoring is not visible in the editor header. | Put the published-resume rule beside save state and retain the explicit version switch. | Autosaving and publishing communicate different outcomes. |
| Key management contains nested panels, buckets, cards, and budget rows. | Use one selected active row and flatter available-key rows. | Key identity, preset, spend, limit, and actions align for comparison. |
| Data puts filters above and below a large framed chart, with activity settings far down the screen. | Group filters above the graph and tighten chart-to-settings spacing. | Scope and related data-management actions become easier to scan. |
| Backup guidance occupies long paragraphs before the main form. | Keep action-critical guidance beside the form and put exclusions/restoration context in a side column. | Important instructions remain present with less competition for the first action. |
| The overlay is dense and repeatedly framed. | Use simpler tabs and dividers while preserving the qualification alert and export group. | Capture-to-review controls keep their identity at a narrow width. |

These are design judgments from the inspected screens, not user-research findings.

## Functionality preserved in the proposal

| Surface | Existing capability and proposed location |
| --- | --- |
| Main shell | Master resume, AI & monitoring, Application tracker, Settings, overlay visibility control, storage health status. Tracker behavior remains in its existing workspace; it is not redesigned by this request. |
| Master Edit | Edit/View/Import; style selection; publish; autosave feedback; undo/redo; name/contact editing; contact addition/removal/formatting/links and divider; section rename/reorder/delete/add; entry title, role, details, location, dates, extra, and information; bold/italic/link/clear; paragraph/bullet mode; add/remove items. Some field subcontrols appear when editing the corresponding field, as in the existing app. No entry duplicate or reorder operation is invented. |
| Master View | Saved and published resume selection, style selection, publish, PDF and Word export of the displayed version. No fit score, cloud sync, or additional export format is introduced. |
| My Keys | Add/name/provider/key entry with show/hide; secure-vault explanation; active-key selection by drag or keyboard; available and paused keys; model presets; test/pause/unpause/remove through the existing options control; rename; per-key lifetime estimates and spending caps; shared spending cap; restart/remove limit. Dialogs, disclosure, and confirmation semantics remain feature-owned. |
| Data | All-key and individual-key filters; Price/Tokens; currency; Week/Month/Year/All time; operation/attempt totals; partial/unknown usage and unresolved-reservation explanation; provider-billing caveat; JSON activity export; clear selected months; removed-key data deletion; 30 days/90 days/one year/retain-until-cleared; apply retention and its existing confirmation. |
| Settings | Backup/recovery, storage/deletion, and browser sections; encrypted backup passphrase/confirmation; backup checking; authenticated inventory; safety-copy rollback/deletion; replace-from-backup; storage inventory/refresh; typed deletion confirmation; development Chrome enable/disable. Existing native file dialogs and restart activation are preserved. |
| Overlay | Captured-job Resume state after tailoring; AI preset/readiness/Stop when working; browser connection; draggable header; company/role; Finish Application; tracker detail editing; end without saving; Resume/Cover letter/Answers; tailoring notes; qualification gaps; resume style; PDF/Word; Download; Drag me; View/Edit popups; AI refinement; save/export feedback and retries. Capture review, paste, URL editing, Tailor, cover generation, and answer workflows remain in their existing states. |

The overlay image intentionally shows the Resume tab after the captured job has
been accepted and tailored. The job-capture review state is earlier in the same
existing flow; no new job-details panel is introduced in this Resume state.

## Deliverables

- `screens/edit.png`, `view.png`, `keys.png`, `data.png`, `settings.png`, `overlay.png`: six high-resolution PNGs, with all application and provider logos omitted.
- `logo/logo-presentation.png`: the separate identity board.
- `logo/open-folio-mark.svg`, `open-folio-reversed.svg`, `open-folio-lockup.svg`, `open-folio-app.svg`: editable, font-independent vector masters.
- `logo/open-folio-mark.png`, `open-folio-reversed.png`, `open-folio-lockup.png`: transparent high-resolution artwork.
- `logo/app-16.png` through `app-1024.png`: application icon sizes.
- `logo/OpenFolio.icns`: macOS icon bundle.
- `logo/OpenFolio.ico`: Windows icon bundle, containing 16–256px sizes.
- `source/`: self-contained mockup source, fonts/licenses, and rendering scripts.
- `review/`: small-desktop proposal captures and review evidence. The prior current UI captures moved to `../previous refrence/earlier-current-ui-captures`. Fresh before-change captures are in `../previous refrence/current-ui-2026-10-04`.

The proposed token system and visual direction are recorded in [DIRECTION.md](DIRECTION.md).
The review-only proposed system is recorded in [DESIGN.md](DESIGN.md), with the
Stitch-compatible extension sidecar at [.impeccable/design.json](.impeccable/design.json).
The documenter verified the source token declarations and mockup function inventory
against `source/mockups.css`, `source/mockups.js`, `apps/desktop/src/shared/AppShell.tsx`,
`SettingsWorkspace.tsx`, `AiWorkspace.tsx`, `ApplicationOverlay.tsx`, and the shared
`app.css`. This folder records the original proposal. The user subsequently approved implementation; the active app design system is now at `../../DESIGN.md`, and the implementation evidence is in `../UI-Revamp-Implementation-2026-10-04`.
The Hanken Grotesk font was downloaded from the Google Fonts repository, is bundled
locally, and ships with its OFL license. Liberation Serif ships with its existing license. Logo lettering
is outlined; viewing the SVG does not require the font.
