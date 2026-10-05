---
name: Open Folio
description: A quiet, document-first resume workspace with a deep teal rail and focused paper canvas.
colors:
  rail: "#204d52"
  accent: "#28676c"
  accent-hover: "#1d4f53"
  ink: "#1c3135"
  muted: "#566c70"
  selection: "#dcedef"
  canvas: "#f2f6f7"
  surface: "#ffffff"
  line: "#d8e3e5"
  success: "#2d624f"
  warning: "#775021"
  danger: "#a13b3b"
  focus: "#469298"
typography:
  display:
    fontFamily: "Hanken, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "28px"
    fontWeight: 650
    lineHeight: 1.2
  headline:
    fontFamily: "Hanken, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "20px"
    fontWeight: 650
    lineHeight: 1.25
  title:
    fontFamily: "Hanken, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.25
  body:
    fontFamily: "Hanken, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Hanken, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1.5
    letterSpacing: "0.01em"
rounded:
  xs: "3px"
  sm: "5px"
  md: "6px"
  button: "7px"
  lg: "10px"
  xl: "12px"
  pill: "20px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  section: "32px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.surface}"
    rounded: "{rounded.button}"
    padding: "10px 16px"
    height: "40px"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.button}"
    padding: "10px 16px"
    height: "40px"
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "40px"
---

# Design System: Open Folio

## Overview

**Creative North Star: “The Quiet Folio.”**

Open Folio is a calm document workspace: a deep teal rail gives the product a stable spine while a pale canvas and white paper keep writing, editing, and review in focus. The interface is compact and information-rich without feeling compressed. Hanken Grotesk gives labels and controls a warm, practical voice; restrained borders and tonal surfaces do the hierarchy work.

The built UI is the source of truth. The same grammar carries through Edit, View, Data, Settings, Tracker, AI, backup, browser connection, and the 360 × 760 application overlay. The Open Folio mark is used in the rail and overlay lockups, with the reversed and mark SVG masters in `apps/desktop/src/assets/`.

The implemented editor’s live controls and spacing are approved as the final visual reference. This acceptance supersedes the original static edit mockup as the authority for the shipped editor. The archived strict comparison remains historical evidence only; it does not represent a failed product approval or a passed original-pixel comparison.

**Key Characteristics:**
- Persistent deep-teal navigation rail and focused paper workspace.
- Pale canvas, white surfaces, fine blue-grey rules, and one controlled teal accent.
- Compact rows, explicit section focus, and visible keyboard-accessible controls.
- Tonal depth with rare ambient shadows; no decorative gradients.

## Colors

The palette is deep, cool, and editorial: teal establishes trust and orientation, while pale blue-grey surfaces make long editing sessions easy on the eyes.

### Primary
- **Folio Teal** (`#28676c`): Primary actions, links, active section indicators, chart accents, and selected controls.
- **Folio Teal Hover** (`#1d4f53`): Pressed and hover state for primary actions.

### Neutral
- **Deep Teal Rail** (`#204d52`): Persistent navigation rail and brand anchor.
- **Ink** (`#1c3135`): Main text and high-contrast UI labels.
- **Muted Slate** (`#566c70`): Supporting copy, placeholders, metadata, and secondary labels.
- **Canvas Mist** (`#f2f6f7`): Native window background and page canvas.
- **Paper** (`#ffffff`): Resume paper, cards, fields, menus, and popup surfaces.
- **Rule Blue** (`#d8e3e5`): Dividers, borders, table rules, and inactive outlines.
- **Selection Mist** (`#dcedef`): Selected rows and soft active backgrounds.
- **Focus Blue** (`#469298`): Visible keyboard focus outline.
- **Success Moss** (`#2d624f`): Successful status and completion states.
- **Warning Ochre** (`#775021`): Caution and attention states.
- **Danger Red** (`#a13b3b`): Destructive actions and validation errors.

### Named Rules
**The Teal Anchor Rule.** Keep the rail and primary accent visually related; use accent color for actions and state, never as a blanket fill for every surface.

## Typography

**Display Font:** Hanken Grotesk (`Hanken`) with system sans fallbacks
**Body Font:** Hanken Grotesk (`Hanken`) with system sans fallbacks
**Label Font:** Hanken Grotesk, semibold at compact sizes

Hanken is self-hosted from `apps/desktop/src/assets/fonts/HankenGrotesk.ttf` under the included OFL notice. It is friendly and legible at small sizes, with enough weight contrast for a dense desktop tool.

### Hierarchy
- **Display** (650, 28px, 1.2): Workspace and document titles where a strong page heading is needed.
- **Headline** (650, 20px, 1.25): Panel headings, settings group titles, backup headings, and AI workspace headings.
- **Title** (600, 16px, 1.25): Compact brand and control titles; larger section headings use the display/headline roles.
- **Body** (400, 14px, 1.5): Explanatory copy, resume content, and form values.
- **Label** (600, 12px, 1.5): Navigation labels, metadata, table headings, and control captions.

**The Quiet Type Rule.** Use weight and spacing to establish hierarchy; do not introduce display faces, all-caps decoration, or oversized marketing typography into the document workspace.

## Layout

The native default workspace is 1080 × 760 logical pixels, with a 214px persistent rail, a 96px page header, and a 72px resume toolbar. The editor body places a 206px section navigator beside an approximately 800px paper canvas. The shell uses `grid-template-columns: 214px minmax(0, 1fr)` and keeps the document surface scrollable inside the window.

At 1200px the rail contracts to 180px; at 850px it contracts to 148px; at 760px it becomes a 72px icon rail. At the supported 720 × 520 native minimum, the editor keeps a 148px section navigator beside the paper in a two-column grid; the View surface may stack its reading panel as a deliberate minimum-width exception. Other content grids collapse progressively so forms, AI cards, tracker tables, and backup guidance remain usable. The application overlay is a fixed 360 × 760 logical-pixel rail and follows the same tokens and control language.

Use 4/8/12/16/24/32px spacing for rhythm. Keep primary content aligned to the paper and navigator columns. Avoid full-bleed decorative regions that compete with the resume or active task.

The page heading stays outside `.app-content`, so each workspace title remains visible while its content scrolls. Switching main workspaces resets that scroll area to the top without remounting feature-owned work. Edit, View, Import, and AI My Keys/Data controls stay sticky at `top: 0` within that scroll area; reserve `84px` scroll padding so anchored content clears them. The Resume sections index stays below the 72px resume toolbar and scrolls independently when the list exceeds the available height. Active text fields use the original compact four-button editing popup and scroll into view without widening the entire entry.

## Elevation & Depth

The system is primarily flat and layered. Borders, the rail/canvas contrast, and white paper create most depth. Shadows are rare and ambient, reserved for floating or raised AI surfaces and never used to make every card look clickable.

### Shadow Vocabulary
- **Ambient card** (`0 5px 18px rgb(36 78 88 / 6%)`): Subtle separation for a raised AI or utility surface.
- **Default surface** (`none`): Resume paper, forms, tables, settings groups, and ordinary cards.

**The Flat-by-Default Rule.** Add elevation only when an element floats above document flow or needs to separate from a busy canvas.

## Shapes

Controls use small, consistent corners: 7px for buttons, 6px for fields, 5px for compact tags, and 10–12px for larger cards or grouped workspaces. Pills use 20px only for status or compact metadata. Borders are thin and blue-grey. Resume paper and document surfaces keep square or near-square edges where the page metaphor benefits from it.

## Components

### Buttons
- **Shape:** 7px radius, minimum 40px height.
- **Primary:** Folio Teal fill with white text and 10px 16px padding.
- **Hover / Focus:** Hover shifts to `#1d4f53`; keyboard focus uses a 3px `#469298` outline with 3px offset.
- **Secondary / Quiet:** White fill, Ink text, Rule Blue border; hover uses a pale teal tint and stronger border.
- **Danger:** White fill with Danger Red text and border; hover uses a soft red wash.

### Chips
- **Style:** Compact 5px or pill radius, pale selection background, muted or accent text, and minimal border.
- **State:** Use accent or selection tint for active filter/status; reserve Danger Red for destructive or invalid state.

### Cards / Containers
- **Corner Style:** 10–12px for grouped workspace panels; 5–7px for compact utility cards.
- **Background:** White Paper on Canvas Mist, with Selection Mist for active rows.
- **Shadow Strategy:** Flat by default; Ambient card shadow only for raised/floating surfaces.
- **Border:** 1px Rule Blue where a boundary improves scanning.
- **Internal Padding:** 16px for ordinary groups, 24px for spacious workspace panels.

### Inputs / Fields
- **Style:** White fill, 1px blue-grey border, 6px radius, 40px minimum height, Ink text.
- **Focus:** 3px Focus Blue outline with 3px offset; caret uses Folio Teal.
- **Error / Disabled:** Danger Red for errors; disabled controls retain structure at 55% opacity.

### Resume Entry Editing

Entry layout keeps Title first, sized to its text, with the same decorative `|` separator as View between Title and Skills/details and a 7px gap on either side; neither resting field grows to fill the row. An active editor can grow for its existing formatting controls. Role sits below Title, while Date, Location, and Extra form a right-aligned stack. Empty Date uses the same `canvas-field--empty` treatment as other empty controls: 11px Hanken text in Muted Slate with an Add affordance. The text editor restores the original compact popup with × (clear), B (bold), I (italic), and Link. Information keeps the existing Paragraph/Bullet points toggle. Clicking outside or pressing Escape closes the editor without reverting autosave; Escape returns focus to the field. Link validation and Apply remain intact. The compact Link address fits its popup, and opening Link scrolls the expanded editor into view. Ctrl/Meta+B and Ctrl/Meta+I preserve existing markdown formatting.

### Navigation

The rail is Deep Teal with white brand lockup, compact navigation rows, and a clear active accent/selection state. The section navigator is a narrow document index beside the paper, with focused section titles, reorder affordances, and an explicit delete target. At narrow widths the rail becomes icon-first while preserving tooltips and active state.

### Signature Components

The resume paper is the signature surface: white, centered, bounded by the navigator and toolbar, and intentionally quieter than controls around it. The AI workspace uses charts, provider badges, usage summaries, and prompt/result panels within the same paper-and-rule grammar. The application overlay compresses the same system into a 360px fixed rail for capture, view/edit popups, downloads, and native file drag.

## Do's and Don'ts

- **Do** use `apps/desktop/src/shared/workspace-theme.css` as the canonical token and layout layer.
- **Do** preserve the 214px rail, 206px section navigator, 800px paper relationship at the default workspace size.
- **Do** keep active section focus visible and make keyboard focus visible with the established 3px outline.
- **Do** use the Open Folio reversed/mark SVG assets for brand lockups rather than redrawing the logo.
- **Do** keep export document styles unchanged when extending workspace UI.
- **Do** keep sticky workflow controls at the top of `.app-content` and pair them with `84px` scroll padding.
- **Do** preserve the entry hierarchy and inline editing behaviors described above, including autosave, keyboard shortcuts, and the original four-button popup.
- **Don't** replace Hanken with a new display font or introduce gradients into the workspace.
- **Don't** turn every container into a rounded, floating card; use borders and tonal layering first.
- **Don't** let accent teal dominate the canvas or obscure the white paper/document hierarchy.
- **Don't** treat the archived strict comparison with the original static mockup as a pixel-pass claim; the accepted implemented layout is the visual reference.
- **Don't** infer that the automated component-review workflow is complete from human acceptance; its mechanical plates guard remains open because the approval could not be imported into that store.
