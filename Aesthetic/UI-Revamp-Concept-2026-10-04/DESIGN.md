---
name: Open Folio UI Revamp Concept
description: Review-only calm desktop workspace proposal for Open Resume Toolkit.
colors:
  ink: "#1C3135"
  muted: "#566C70"
  teal: "#28676C"
  deep-teal: "#1D4F53"
  navigation-rail: "#204D52"
  rail-muted: "#B9D2D4"
  workspace-ground: "#F2F6F7"
  document-surface: "#FFFFFF"
  selected-field: "#EDF7F6"
  divider: "#D8E3E5"
  warning: "#775021"
  warning-ground: "#FCF4E7"
  danger: "#A13B3B"
typography:
  display:
    fontFamily: "Hanken, sans-serif"
    fontSize: "28px"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.025em"
  title:
    fontFamily: "Hanken, sans-serif"
    fontSize: "20px"
    fontWeight: 650
    lineHeight: 1.35
    letterSpacing: "-0.015em"
  body:
    fontFamily: "Hanken, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Hanken, sans-serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1.5
  resume:
    fontFamily: "ResumeSerif, serif"
    fontSize: "11.5px"
    fontWeight: 400
    lineHeight: 1.55
rounded:
  control: "6px"
  button: "7px"
  panel: "12px"
  overlay: "14px"
  pill: "20px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.teal}"
    textColor: "{colors.document-surface}"
    typography: "{typography.label}"
    rounded: "{rounded.button}"
    padding: "0 15px"
    height: "40px"
  button-secondary:
    backgroundColor: "{colors.document-surface}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.button}"
    padding: "0 15px"
    height: "40px"
  input:
    backgroundColor: "{colors.document-surface}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "40px"
  panel:
    backgroundColor: "{colors.document-surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.panel}"
    padding: "24px 26px"
  active-field:
    backgroundColor: "{colors.selected-field}"
    textColor: "{colors.ink}"
    rounded: "8px"
    padding: "12px 13px"
---

# Design System: Open Folio UI Revamp Concept

## Overview

**Proposal status:** review-only visual system derived from the isolated mockup source and six captured screens. It does not describe a production implementation and does not replace the current application system.

**Creative North Star: "The Open Folio"**

The proposal treats the desktop app as a calm document workspace: a deep blue-green rail establishes orientation, quiet white papers hold resume content, and muted blue-green accents identify actions and selected state. Hanken Grotesk carries the interface; the sample resume remains a black Liberation Serif document so the document reads as an artifact inside the tool.

The system makes the distinction between autosaved draft, deliberately published resume, and the application overlay visible. It favors familiar desktop controls, restrained borders, and tonal layering over decorative effects. Logo and provider marks are intentionally omitted from the six interface images; the independent Open Folio mark is documented as a separate identity deliverable.

**Key Characteristics:**
- Persistent deep teal navigation rail with local-storage status.
- Quiet white document surfaces with one visibly active inline editing field.
- Muted blue-green action color, warm qualification warning, and restrained dividers.
- Four-pixel base rhythm with 6–14px control and surface radii.

## Colors

The palette is calm and blue-green, with warm amber reserved for qualification and unknown-usage states.

### Primary
- **Muted Teal** (`#28676C`): primary actions, selected tabs, progress, and active editing affordances.
- **Deep Teal** (`#1D4F53`): strong headings, selected navigation text, and hover treatment.

### Secondary
- **Navigation Teal** (`#204D52`): persistent shell rail and the strongest structural surface.

### Neutral
- **Interface Ink** (`#1C3135`): primary interface text.
- **Muted Slate** (`#566C70`): support text and secondary labels.
- **Workspace Ground** (`#F2F6F7`): desktop canvas behind papers and panels.
- **Document White** (`#FFFFFF`): papers, forms, and raised panels.
- **Selected Field Mint** (`#EDF7F6`): active key row and selected editing state.
- **Quiet Divider** (`#D8E3E5`): borders and separators.
- **Rail Mist** (`#B9D2D4`): secondary rail text.

### Tertiary
- **Qualification Amber** (`#775021`) on **Warm Warning Ground** (`#FCF4E7`): qualification gaps, partial usage, and unknown reservation notices.

### Named Rules
**The One Accent Rule.** Teal carries action and selection meaning; keep it concentrated so a selected field or primary action is immediately legible.

## Typography

**Display and Body Font:** Hanken Grotesk, with a sans-serif fallback.
**Resume Font:** Liberation Serif, with a serif fallback, for the document artwork only.

**Character:** Hanken is compact, legible, and workmanlike at desktop control sizes. Liberation Serif gives the sample resume a familiar editorial document voice without making the application chrome ornamental.

### Hierarchy
- **Display** (650, 28px, 1.2): interface page titles.
- **Title** (650, 20px, 1.35): panel headings and workspace sections.
- **Body** (400, 14px, 1.5): descriptions and form values.
- **Label** (600, 12px, 1.5): controls, field labels, and compact metadata.
- **Resume** (400/700, 11.5–28px, 1.55): resume content and name within the paper.

### Named Rules
**The Two Voice Rule.** Use Hanken for the tool and Liberation Serif for the resume artifact; do not blur document typography into the shell.

## Layout

The desktop shell reserves a 214px navigation rail and a 38px native-window presentation bar in the source mockup, then gives the main workspace the remaining width. At smaller widths the rail contracts to 180px and 148px, the resume section panel contracts to 180px and 156px, and content grids collapse to one column where needed. Main surfaces use 24–32px outer padding, 16–26px panel padding, and a 4px base rhythm.

Master Edit places section navigation beside a centered paper; View places saved/published selection and exports above the paper. AI screens put filters and summaries above the chart or key rows. Settings pairs the backup form with a narrower safety-notes column. The overlay is a narrow, centered 392px window intended to remain readable alongside browser work.

## Elevation & Depth

Depth is primarily tonal: white papers and panels sit on cool gray-blue workspace ground, with borders carrying structure. Shadows are restrained and reserved for paper, active segmented choices, and the narrow overlay window. No hard offset shadow is a system rule.

### Shadow Vocabulary
- **Paper lift** (`0 5px 18px #244E5810`): quiet separation for resume papers.
- **Selected segment** (`0 1px 3px #183C4314`): slight lift for the active version or mode choice.
- **Overlay lift** (`0 12px 40px #1A474626`): floating application overlay against its artboard.

### Named Rules
**The Tonal Layer Rule.** Establish hierarchy with surface color and dividers first; use shadow only where a paper or overlay must separate from its canvas.

## Shapes

Controls use gently curved 6–8px corners; panels use 12px; pills use 20px. The overlay window uses a 14px silhouette. Borders are 1px and cool blue-gray. The resume paper itself stays rectangular so it reads as a page. Focus uses a 3px teal outline with a 3px offset, while selected fields use a pale mint surface and 8px radius.

## Components

### Buttons
- **Shape:** compact rounded rectangle (7px), 40px default height, 32px compact height.
- **Primary:** muted teal background, white text, 0 15px horizontal padding.
- **Hover / Focus:** primary darkens to deep teal; all actionable controls receive a 3px teal focus outline with 3px offset.
- **Secondary / Quiet:** white with a quiet divider, or transparent teal text for low-emphasis actions.

### Chips
- **Style:** 4px 9px padding, 20px radius, small semibold text; green is used for saved/local state, neutral gray-blue for paused state, and amber for warnings.
- **State:** a dot reinforces status meaning; chips remain compact and non-interactive in the proposal.

### Cards / Containers
- **Corner Style:** 12px panels; 8–10px selected rows and disclosures.
- **Background:** white panel over the workspace ground.
- **Shadow Strategy:** borders first, with the paper and overlay shadow vocabulary above.
- **Border:** 1px quiet divider.
- **Internal Padding:** 24–26px for panels; 34–46px for the edit paper.

### Inputs / Fields
- **Style:** white fill, 1px `#C5D5D8` stroke, 6px radius, 40px height for inputs and selects.
- **Focus:** 3px teal outline with offset; active editing block uses pale mint background and a 1px stronger teal border.
- **Error / Disabled:** disabled controls use muted text, pale gray fill, and a quiet border; warm amber carries partial or unknown usage.

### Navigation
- **Style:** persistent deep teal rail with 44px minimum link height, 7px radius, and 11px icon-to-label gap. Active destination uses a lighter teal tonal block and white text. Local page modes use compact segmented controls and tabs.

### Signature Component: Resume Paper
The paper is a centered white document with a serif name, contact line, uppercase section labels, and thin section rules. Edit mode keeps document text quiet and exposes the active inline field with a mint block and a small Hanken toolbar.

## Do's and Don'ts

### Do:
- **Do** keep the distinction between draft saved locally and published resume visible near resume actions.
- **Do** use teal for action, active selection, and progress so its meaning remains consistent.
- **Do** preserve keyboard focus visibility with a 3px outline and retain existing confirmation semantics for destructive settings actions.
- **Do** keep the qualification alert and provider-billing caveat visible in the overlay and AI data states.
- **Do** omit provider and application logos from the review screens; use the separate Open Folio identity assets where branding is needed.

### Don't:
- **Don't** promote the synthetic data, mockup navigation, or illustrative native window bar into production behavior.
- **Don't** add fit scores, cloud sync, new export formats, or invented resume item operations to this proposal.
- **Don't** use the serif face for interface chrome or use decorative shadows to compensate for weak hierarchy.
- **Don't** canonize the one small desktop selector-clipping issue recorded during review; the finish verdict marked it resolved at the approved single-fix scope.
