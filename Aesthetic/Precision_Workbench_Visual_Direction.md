# Precision Workbench visual direction

## Status and intent

Precision Workbench is the approved visual direction for Open Resume Toolkit's application shell, overlay, browser-extension surfaces, and public website. It combines the restraint of a professional document tool with the clarity of a well-maintained open-source utility.

This document defines how the product should **feel, look, and behave visually**. It intentionally avoids dictating that a particular component must occupy a particular edge, column, or coordinate. Actual placement must remain responsive to workflow testing, platform conventions, accessibility, localization, viewport size, and implementation constraints.

The design should be recognizable through its density, rhythm, hierarchy, typography, color behavior, borders, controls, and tone even if two surfaces use different layouts.

## Reference image

The [original Precision Workbench collage](Reference/precision-workbench-original.png) records the initial visual idea across the desktop application, application overlay, and website.

The collage is a **design reference, not a specification**. It may guide overall density, line weight, color restraint, type scale, information hierarchy, and visual tone. It must not be treated as authoritative for:

- exact control placement;
- navigation architecture;
- user-facing terminology;
- workflow states;
- feature completeness;
- responsive behavior;
- platform window chrome;
- release dates, version numbers, example content, or legal text;
- screenshots shown inside the website concept;
- final document-template appearance.

When the reference conflicts with an approved product plan, accessibility requirement, or verified implementation behavior, the plan or requirement takes precedence. Preserve the visual character rather than reproducing the pixels.

## Design thesis

Precision Workbench should look like a tool someone can trust with important personal documents and repeated practical work. It should feel intentionally built, locally grounded, and easy to inspect. The interface should make structure visible without turning every region into a heavy card or every action into a prominent button.

The central impression is **quiet precision**:

- Quiet means low visual noise, restrained color, direct language, limited elevation, and no unnecessary animation.
- Precision means consistent alignment, deliberate spacing, crisp borders, clear states, and predictable interaction patterns.
- Workbench means the product supports focused editing and review. It may be information-dense, but the density is organized, readable, and purposeful.

The product should not resemble a lifestyle brand, an employment marketplace, a gamified assistant, a glossy AI startup, or a generic marketing dashboard. It should visibly behave like open-source desktop software and public technical documentation.

## Core character

### Structured

Information is organized through alignment, grouping, headings, dividers, and repeated patterns. Relationships should be understandable before the user reads every label. Structure should come primarily from spacing and rules rather than large decorative containers.

### Compact

The interface uses space efficiently. It may show forms, document previews, status, and navigation at the same time when the viewport allows. Compactness must never reduce target sizes, remove labels, or create illegible text. It is an information strategy, not permission to make everything small.

### Calm

Only a few elements compete for attention. Primary actions are unmistakable but not oversized. Status colors appear where meaning requires them. Empty space is used to separate tasks and reduce cognitive load, not to create a luxurious marketing aesthetic.

### Technical without being cold

Crisp rules, tabular values, predictable controls, and occasional monospace metadata may communicate engineering discipline. Human-readable labels, comfortable line height, plain explanations, and careful error recovery keep the product approachable.

### Factual

The visual system reinforces direct product language. It should not rely on slogans, motivational copy, artificial urgency, celebratory confetti, scores, streaks, or vague claims. The user is completing work, not participating in a conversion funnel.

## Color system

Precision Workbench uses one light brand scheme. Operating-system forced-colors and high-contrast modes are accessibility behaviors, not alternate branded themes.

| Role | Provisional value | Intended behavior |
|---|---:|---|
| Quiet Navy | `#102A4C` | Primary actions, selected emphasis, logo, important rules, and restrained brand identity |
| Ink | `#172235` | Primary text and high-importance neutral content |
| Mist | `#DCE8F5` | Selected rows, informational grouping, quiet emphasis, and non-blocking contextual surfaces |
| Canvas | `#F7F9FC` | Application and page background separating white work surfaces |
| White | `#FFFFFF` | Primary working surfaces, documents, forms, and content regions |
| Warning | `#C75B12` | Warnings and qualification-alert emphasis with text or icon support |
| Success | `#2F6D58` | Confirmed success and connected or published states with text or icon support |

Exact production values may receive small contrast corrections during component testing. Roles should remain stable even if values change.

### Color behavior

- Navy is a tool color, not a decorative wash. Use it where hierarchy or action requires it.
- Large regions should remain predominantly Canvas, White, and Ink.
- Mist should clarify selection or grouping without making the product look pastel or soft-focus.
- Warning and Success must be sparse and semantic. They never substitute for a written state.
- Disabled elements should remain legible and distinguishable without appearing active.
- Links must remain identifiable independently of hue through underlines, placement, icons, or other established affordances.
- Resume and cover-letter content does not inherit these colors unless an independent professional template explicitly defines its own restrained palette.

## Typography

The preferred interface typeface is Inter if its bundled license, platform rendering, and accessibility tests pass. Otherwise use an approved neutral sans-serif with similar proportions and high legibility.

Typography should feel practical, contemporary, and durable. It should not look futuristic, luxury-oriented, editorially theatrical, or playful.

### Hierarchy

- Page and workspace titles are clear but moderate in scale.
- Section headings rely on weight, spacing, and alignment rather than dramatic size changes.
- Body text is comfortable for sustained reading and form work.
- Labels are concise, consistently placed, and visually subordinate to entered values without becoming faint.
- Metadata may use tabular numerals and, where genuinely helpful, a neutral monospace face.
- Uppercase micro-labels may identify compact regions or table groupings, but long text should remain sentence case.
- Button text should be literal and action-oriented.

No part of the product should require oversized display typography to establish identity. The interface should continue to feel like Precision Workbench when viewed at 200% text scaling.

## Density and spacing

Begin from a 4 px spacing unit. Common relationships may use 4, 8, 12, 16, 24, and 32 px values, refined by component and platform testing.

Precision Workbench is denser than a marketing-oriented interface, but it is not cramped. Density should come from coherent grouping and economical phrasing.

### Desired rhythm

- Related label/value pairs feel tightly connected.
- Separate tasks receive enough distance or a divider to be unmistakable.
- Repeated rows align to a predictable rhythm.
- Toolbars remain compact while preserving accessible hit targets.
- Long-form documentation uses a comfortable reading width and more vertical breathing room than data-entry screens.
- Empty states occupy enough space to be recognized without becoming illustrations or promotional panels.

Avoid arbitrary one-off spacing. Repetition is part of the aesthetic: users should sense a common measuring system even when they cannot name it.

## Geometry, borders, and elevation

The visual system is predominantly flat.

- Use crisp 1 px borders to define working regions, controls, table cells, and separations.
- Use modest radii, generally around 4 px for dense controls and up to 8 or 12 px only where a larger grouped surface benefits from it.
- Avoid pill shapes except where the control's behavior genuinely calls for a compact segmented or status treatment.
- Use shadows rarely and lightly, primarily to clarify a native overlay, modal, popover, or floating relationship.
- Do not create depth through glass effects, blur, glow, bevels, or gradients.
- Prefer clean rectangular geometry and squared icon construction.

Borders should organize rather than decorate. When spacing alone communicates the boundary, an additional box may be unnecessary.

## Visual hierarchy

At any moment the interface should make four things understandable:

1. where the user is;
2. what content is being viewed or edited;
3. what state that content is in;
4. what the next meaningful action is.

Hierarchy should be achieved through a combination of title, selection treatment, grouping, weight, and restrained color. Avoid making several actions equally prominent. Destructive, secondary, and infrequent actions should not visually compete with the primary task.

Status should live close enough to the affected content to be understood, but global state may also appear in a consistent application-status region. A status must not rely on an isolated colored dot.

## Navigation character

Navigation should feel persistent, predictable, and compact. The exact presentation may be a rail, list, toolbar, responsive menu, or platform-appropriate alternative.

Regardless of placement:

- top-level product destinations must remain distinguishable from navigation within a document or feature;
- selection uses a calm Navy or Mist treatment with a text or structural indicator;
- icons support labels rather than replace unfamiliar destinations;
- settings should not become a catch-all location for ordinary working views;
- keyboard focus and selected state must be visually distinct;
- collapsed or narrow navigation must preserve accessible names and discoverability;
- responsive changes may alter orientation or grouping without changing the underlying information architecture.

The initial collage's navigation placement is illustrative only.

## Controls and interaction states

Controls should resemble reliable desktop and documentation controls rather than promotional calls to action.

### Primary actions

Use Quiet Navy with white text when an action is the clear next or committing step. Primary actions should be easy to find but modest in size and frequency.

### Secondary actions

Use white or neutral surfaces with crisp borders and Ink or Navy text. Related secondary actions may sit near the content they affect.

### Tertiary actions

Use text links or compact icon-plus-label treatments when the action is reversible, contextual, or low frequency. Icon-only controls require familiar semantics, tooltips, and accessible names.

### Destructive actions

Do not use Warning orange as a general attention color. Destructive actions need explicit language, confirmation proportional to risk, and focus-safe recovery.

### States

Hover, focus, pressed, selected, disabled, loading, success, warning, and error states must be intentionally designed. Focus indicators should be crisp and high contrast. Loading behavior should preserve surrounding layout and explain longer operations without visual spectacle.

## Iconography

Icons should use a consistent 2 px line style or an optically equivalent weight at the target size. Forms are simple, geometric, and immediately legible.

- Use square or gently rounded geometry compatible with the Offset Open Frame mark.
- Avoid filled cartoon icons, multicolor illustrations, emoji, 3D objects, and decorative spot art.
- Prefer icon-plus-label for important or unfamiliar actions.
- Maintain a coherent metaphor for documents, downloads, drag handles, connections, monitoring, settings, and warnings.
- Do not use sparkles, magic wands, brains, robots, or other shorthand to advertise AI.

AI functions should look like ordinary controlled product operations, with provider and status made explicit.

## Desktop application expression

The desktop application is the primary work environment. It should feel like a focused authoring and administration tool capable of sustained use.

Its visual system should support:

- structured master-resume editing;
- conventional document preview;
- publishing state;
- application tracking;
- aggregate AI Monitoring;
- connection and data settings;
- backup, recovery, updates, and legal information.

The main window should balance structured data and document output without making either feel secondary. Forms benefit from aligned labels, predictable optional-detail patterns, explicit save or validation feedback, and compact repeatable rows. Document previews should feel materially distinct from editable fields: a conventional page on a neutral work surface rather than an ORT-branded card illustration.

Top-level destinations such as Resume, Tracker, AI Monitoring, and Settings should be visually and conceptually distinct from resume-section navigation. The exact location and orientation are implementation decisions.

The main window must not visually imply that current-job tailoring occurs there. Job-specific change summaries, qualification alerts, cover-letter work, application answers, and Finish Application belong to the overlay.

## Application overlay expression

The overlay is a real, separate, always-on-top desktop window. It should look related to the main application without appearing to be a browser injection or a miniature duplicate of the entire desktop app.

The overlay has two broad functional conditions, but **numbered stages are not user-facing titles**.

### Before job capture

The overlay should describe its current state directly, using language such as **Ready to capture** or **Capture job description**. It should make the browser connection and next user action understandable. The UI remains focused and does not expose material tabs or Finish Application before a current application workspace exists.

### After initial tailoring

The overlay should identify the current application using reviewed context such as company and role. Resume, Cover letter, and Answers are sibling working areas. The current material, no more than three concise change points, non-blocking Required Qualification Alerts, document preview/edit/download/drag controls, and Finish Application should be discoverable without presenting the workspace as a numbered wizard.

The overlay may expand substantially for structured editing and large PDF preview. Compact and expanded modes should share the same typographic, border, and action hierarchy. Placement and dimensions may change by platform and viewport as long as the window remains usable and does not trap or obscure essential browser or operating-system controls.

## Tracker expression

The application tracker should feel spreadsheet-like, local, and inspectable. It should not become a sales pipeline, gamified job-search dashboard, or employer CRM.

Use stable row alignment, restrained filters, readable status text, and clear selection. Dense tables may use subtle row rules or alternating neutral emphasis, but avoid heavy grid boxes around every value unless testing shows they improve comprehension. Empty and filtered states should explain what is shown without promotional illustrations.

Saved structured snapshots and application metadata should feel durable. Temporary workspace material and permanent tracker history must remain visually distinguishable.

## AI Monitoring expression

AI Monitoring is an aggregate operational view, not an inbox of model calls.

The interface should present Week, Month, Year, and All time periods; token totals; direct estimated cost; provenance and completeness; and accessible chart alternatives. Charts should use Quiet Navy, Mist, Ink, and limited semantic colors. Avoid rainbow series, glowing plots, large decorative numbers, or finance-dashboard theatrics.

Tables and text summaries are equal parts of the design, not accessibility afterthoughts. Direct estimated cost must look different from provider-authoritative billing. Codex account/quota information must not be assigned an invented dollar value.

## Website expression

The website should resemble a carefully maintained open-source project and documentation site. It is informative before it is persuasive.

Its character should include:

- a compact, durable project header;
- bounded reading widths;
- direct navigation to Download, Documentation, source, security, and contribution paths;
- clear release and platform information;
- restrained tables, notices, code blocks, and checksum presentation;
- a documentation hierarchy that works without client-side JavaScript;
- the same Quiet Navy, border, typography, and icon discipline as the desktop product.

The homepage begins with the project name, a factual description, current support or release status, and direct paths to installation and documentation. It must not use a sales-style hero, slogan, testimonial, customer-logo strip, pricing tier, artificial urgency, download counter, or fabricated social proof.

### Screenshot placeholders

The site may reserve space for future verified product screenshots, but generated or premature product images should not ship as previews.

Until the application is implemented, tested, and configured, use neutral placeholder boxes that state the expected future asset, for example:

- **Desktop master-resume editor screenshot**
- **Application overlay - ready to capture**
- **Application overlay - tailored resume workspace**
- **Application tracker screenshot**
- **AI Monitoring screenshot with accessible table equivalent**

Placeholders should use the standard border, Canvas or White background, and quiet descriptive text. They should reserve realistic aspect ratios and dimensions to prevent later layout shifts. They are functional production placeholders, not skeleton-loading animations or decorative mockups.

When real screenshots replace them, captures must come from a tested release candidate, use synthetic data, reflect current terminology and workflow boundaries, and exclude secrets or personal content.

## Browser-extension expression

Extension surfaces remain compact and status-oriented. They communicate connection, deliberate selection capture, success, and safe error recovery. They do not reproduce the overlay workflow, hold AI settings, show a resume database, or become a browser sidebar.

The extension should visibly belong to Precision Workbench through typography, line icons, Quiet Navy, borders, and the Offset Open Frame icon. Small sizes demand especially concise language and obvious keyboard focus.

## Resume and cover-letter independence

Professional documents are a separate visual system. They must not inherit:

- Quiet Navy brand color by default;
- Offset Open Frame marks;
- application navigation styling;
- website cards or controls;
- product terminology;
- decorative software iconography.

The Technical/Engineering default may follow the familiar Jake's Resume-style professional structure as closely as compatible licensing permits: single-column layout, compact serif typography, ruled section headings, clear left/right alignment, and restrained black-and-white presentation. Exact source or asset reuse requires documented license compatibility.

Changing an application theme token must never silently change an exported professional document.

## Logo expression

The selected Offset Open Frame mark uses two offset open squared frames. Its negative space suggests an open O, a document frame, and coordinated work surfaces without drawing a literal resume.

The logo should feel like part of the interface geometry:

- flat and single-color;
- optically balanced at small sizes;
- sturdy enough for 16 px reproduction;
- compatible with Quiet Navy, black, and reversed white use;
- free of gradients, shadows, mascots, lettering tricks, or decorative detail.

Use the horizontal lockup where the full project identity is needed and the icon where space or platform convention requires it. Follow the asset and clear-space rules in [Logo/README.md](Logo/README.md).

## Motion and feedback

Motion is brief, functional, and optional.

- Use movement to clarify state change, expansion, focus, or progress.
- Avoid parallax, ambient animation, pulsing decoration, celebratory effects, and animated gradients.
- Respect reduced-motion preferences without losing information.
- Streaming or progress updates should not continuously seize attention or overwhelm assistive technology.
- Layout should remain stable while work completes.

The product should feel responsive because actions acknowledge input promptly and states are clear, not because the interface is constantly animated.

## Accessibility

Accessibility is part of the aesthetic, not an exception to it.

Precision Workbench must support:

- keyboard-only operation and logical focus order;
- visible, high-contrast focus indicators;
- programmatic labels, descriptions, errors, and status announcements;
- 200% text scaling without loss of content or action;
- high contrast and forced-colors behavior;
- reduced motion;
- non-color status meaning;
- accessible charts with equivalent tables or text;
- target sizes appropriate to supported platforms;
- clear reading order in compact, expanded, and responsive layouts.

Forced-colors rules may replace brand colors when required. Maintaining usability takes precedence over visually preserving the palette.

## Content voice

Interface language is factual, concise, and respectful.

Prefer:

- **Capture job description**
- **Preview and edit**
- **Download**
- **Browser connected**
- **Published**
- **Estimated cost**
- **Required qualification**

Avoid inflated or ambiguous phrases such as “supercharge,” “unlock,” “perfect fit,” “AI magic,” “beat the ATS,” or “land your dream job.”

Warnings explain the condition and recovery path. Empty states explain what is absent and what action is available. Success messages confirm what actually happened without celebration or exaggerated claims.

## Adaptive behavior and implementation freedom

Precision Workbench is not tied to one screen width or one layout arrangement. Implementation may:

- change navigation orientation or collapse behavior;
- stack or separate editors and previews;
- move actions to platform-appropriate toolbars or menus;
- alter component grouping for keyboard order or screen-reader clarity;
- adjust overlay size and internal arrangement;
- convert tables to responsive lists where necessary;
- modify spacing or typography within the approved scale;
- use native dialogs and platform conventions.

These changes are acceptable when they preserve:

- clear hierarchy;
- compact but readable density;
- restrained Quiet Navy color behavior;
- crisp borders and limited elevation;
- direct language;
- persistent distinction between desktop, overlay, extension, website, and professional documents;
- complete accessibility;
- predictable states and actions.

The test is not pixel similarity to the collage. The test is whether the resulting interface still feels like the same calm, precise, inspectable workbench.

## Prohibited drift

Do not evolve Precision Workbench toward:

- dark branded themes;
- gradients, glassmorphism, glow, blur, or neon;
- oversized marketing typography;
- rounded card stacks around every content group;
- decorative dashboards or metric theater;
- AI mascots, sparkles, robots, brains, or magic-wand imagery;
- gamification, fit scores, progress streaks, or artificial urgency;
- sales copy, testimonials, tiers, or repeated calls to action;
- cloud-storage implications;
- browser-injected application workflow;
- ORT-branded resume or cover-letter documents;
- color-only status;
- inaccessible compactness;
- literal reproduction of outdated reference-image text or layout.

## Review criteria

A Precision Workbench implementation is visually on direction when reviewers can answer yes to the following:

1. Does the product feel calm, technical, and dependable?
2. Is information dense where useful but still readable and navigable?
3. Are hierarchy and relationships visible through alignment, spacing, typography, and restrained rules?
4. Is Quiet Navy used as a controlled tool color rather than decoration?
5. Are borders crisp, radii modest, and shadows rare?
6. Are primary actions clear without dominating the screen?
7. Do desktop, overlay, extension, and website feel related while retaining their separate responsibilities?
8. Do professional documents remain independent of application branding?
9. Does the interface remain coherent at high text scaling, narrow widths, forced colors, and reduced motion?
10. Could placement change without losing the design identity?

If the answer to the final question is no, the implementation is relying too heavily on a particular mockup rather than the Precision Workbench system.
