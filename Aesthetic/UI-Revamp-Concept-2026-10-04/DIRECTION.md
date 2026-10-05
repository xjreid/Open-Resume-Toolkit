# Open Folio — proposed UI direction

Scope: six review-only mockup images and a separate replacement logo. This is an
Operate surface: helping a job seeker maintain one master resume and review
job-specific application materials. The user approved exploring a calm desktop
workspace with a muted blue-green accent. No production UI, application logic,
resume templates, existing design plans, or default branding are replaced.

The mechanism is the distinction between an autosaved master draft, a deliberately
published resume, and the separate application overlay. A person uses the editor
alongside a browser while preparing an application; the design makes those
boundaries and states easy to see.

## Direction

The open folio borrows the quiet hierarchy of well-made document collections:
precise typography, white document surfaces, a blue-green shell, and visibly
selected editable areas. Its signature is the contrast between a quiet document
and one active inline field, with section controls grouped beside the document.
Navigation, forms, tabs, and key controls remain familiar desktop components.

The exploration considered document collections, professional correspondence,
public-library graphic systems, transit wayfinding, university publications,
career-portfolio presentation, and accessible information signage. The skill's
external roll catalog was unavailable on both permitted attempts. No external
challenger boards were used. The user's approved direction governs the result.

## Tokens

| Role | Value |
| --- | --- |
| Navigation rail | `#204D52` |
| Primary actions and selection | `#28676C` |
| Strong brand ink | `#1D4F53` |
| Interface text | `#1C3135` |
| Secondary text | `#566C70` |
| Workspace ground | `#F2F6F7` |
| Document and form surfaces | `#FFFFFF` |
| Selected field surface | `#EDF7F6` |
| Dividers | `#D8E3E5` |
| Qualification / unknown usage text | `#775021` |
| Qualification / unknown usage ground | `#FCF4E7` |

Hanken Grotesk is the interface family, with 28px page titles, 20px panel headings,
and 12–14px controls and support text. The sample technical resume remains a black
serif document, using locally bundled Liberation Serif for the artwork. Export
templates are not modified by this proposal. Standard controls use 6–8px corners,
workspace panels 12px, and native window artwork 14px. Spacing uses a 4px rhythm.

## First viewport and reach

The main window has the existing four destinations in a persistent left rail,
plus the existing overlay toggle and encrypted-storage status. The content header
names the workspace; a separate row holds page modes and relevant actions.
Master editing keeps its actual inline editing model, with section ordering,
renaming, addition, and deletion beside the document. The View surface exposes
saved/published selection and explicit exports. My Keys uses rows with a distinct
active-key drop target. Data puts filters above their graph. Settings separates
backup actions from explanatory copy. The overlay carries the same control and
type grammar at its narrow desktop width.

Tradeoffs: the left rail uses some width that formerly belonged to the document,
and reducing persistent field outlines requires clear focus and hover treatment
when implemented. These images show selected happy-path and partial-usage states;
they do not approve changes to error handling or native lifecycle behavior.

## Identity

The independent Open Folio mark uses two upright document leaves and an open
spine. The raised right leaf gives the silhouette direction without a literal
arrow. The logo masters contain paths only. The app icon places the reversed
mark on a deep teal rounded square with transparent outer space. No app or
provider marks appear inside the six interface images.
