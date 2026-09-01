# Offset Open Frame logo system

## Selected mark

The Offset Open Frame is the sole approved Open Resume Toolkit logo direction. Two offset open squared frames create an O-like negative space while suggesting an editable document frame and coordinated work surfaces. The mark is intentionally abstract; it does not draw a literal resume, person, briefcase, cloud, or AI symbol.

The geometry is flat, single-color, and compatible with Precision Workbench's crisp rules and compact technical character.

## Source files

- [`open-frame-icon.svg`](open-frame-icon.svg) - primary Quiet Navy icon.
- [`open-frame-icon-reversed.svg`](open-frame-icon-reversed.svg) - white icon for approved dark solid backgrounds.
- [`open-frame-lockup-horizontal.svg`](open-frame-lockup-horizontal.svg) - primary full-name lockup.
- [`open-frame-lockup-horizontal-reversed.svg`](open-frame-lockup-horizontal-reversed.svg) - reversed full-name lockup.
- [`open-frame-lockup-stacked.svg`](open-frame-lockup-stacked.svg) - stacked alternative for square or centered contexts.
- [`export-logo-assets.mjs`](export-logo-assets.mjs) - deterministic raster-export script; requires the `sharp` package in the active Node.js environment.
- [`exports/manifest.json`](exports/manifest.json) - generated raster-size inventory.

The SVG geometry is the source of truth. Do not resize a PNG to create another official size when an SVG can be rendered directly.

## Color

| Variant | Value | Use |
|---|---:|---|
| Primary | Quiet Navy `#102A4C` | White, Canvas, Mist, and other sufficiently light neutral backgrounds |
| Reversed | White `#FFFFFF` | Quiet Navy, Ink, or another approved sufficiently dark solid background |
| Monochrome | Black or White | One-color printing, legal notices, or environments where brand color is unavailable |

Do not introduce gradients, multiple colors inside the mark, opacity effects, outlines around the finished mark, or status colors. Warning and Success are interface semantics, not logo colors.

## Wordmark typography

The lockups use Inter with Arial as a provisional fallback. The production website and application should bundle and use the approved licensed interface font before relying on the text-based SVG lockups. If final font review selects a different typeface, update every lockup from one controlled source and regenerate all exports together.

Do not manually typeset a new wordmark beside the icon in individual surfaces.

## Clear space

Keep clear space around the icon equal to at least one quarter of the icon's total width. For a lockup, apply the same minimum clear space around the complete icon-and-wordmark bounds.

Platform-required icon containers may impose their own safe area. In those cases, preserve the mark's proportions and optical center rather than stretching it to touch the container.

## Minimum sizes

- Icon-only digital minimum: 16 x 16 px using the supplied 16 px export.
- Horizontal lockup recommended minimum: 160 px wide.
- Stacked lockup recommended minimum: 128 px wide.
- Print minimum: validate with the final output process; do not assume a screen export is suitable for print.

At 16 and 20 px, use the supplied export or the source SVG rendered at the exact target size. Do not add internal detail.

## Context guidance

### Website

- Use the horizontal SVG or a matching 240/320 px raster export in the global header when the full project name is useful.
- Use the icon SVG or 32/48 px export for favicons and compact navigation.
- Use a 512 or 1024 px icon export as source material for social-preview composition; do not stretch the small favicon.
- Use the reversed lockup only on a solid approved dark background.

### Desktop application

- Use icon exports from 16 through 1024 px as the source set for platform packaging.
- Generate `.ico` and `.icns` packages from the controlled size set during the application build; do not hand-edit individual frames.
- Window chrome and compact navigation normally use the icon only.

### Browser extension

- Use the 16, 32, 48, and 128 px primary icon exports required by supported browser manifests and stores.
- Keep the mark visually separate from connection, warning, and capture-state badges.

### Documentation and print

- Prefer SVG or single-color vector output.
- Use the full lockup when project identity might otherwise be ambiguous.
- Keep the logo outside resume and cover-letter documents.

## Raster exports

### Primary icons

`exports/icons/` contains 16, 20, 24, 32, 48, 64, 128, 256, 512, and 1024 px transparent PNGs.

### Reversed icons

`exports/reversed/` contains white transparent icons at 16, 24, 32, 48, 64, 128, 256, and 512 px, plus reversed horizontal lockups.

### Lockups

`exports/lockups/` contains horizontal lockups at 160, 240, 320, 480, 640, and 960 px widths and stacked lockups at 128, 256, 512, and 1024 px widths.

## Prohibited modifications

Do not:

- stretch, compress, skew, rotate, or crop the mark;
- change the relative offset of the two frames;
- round the frame corners independently;
- add shadows, gradients, glow, bevels, texture, or animation;
- place the mark inside an arbitrary badge or circle;
- combine it with a resume-page illustration, AI symbol, browser logo, or provider logo;
- use Warning or Success colors as branded variants;
- recreate the wordmark with inconsistent type, spacing, or capitalization;
- place ORT branding inside generated resumes or cover letters.

## Release validation

Before treating the mark as release-final:

1. Perform trademark and visual-distinctiveness review.
2. Confirm the selected wordmark font and its distribution license.
3. Inspect the SVG and exact PNG exports at 16, 20, 24, 32, 48, 64, 128, 256, 512, and 1024 px.
4. Test Windows, macOS, Chrome, Edge, favicon, forced-colors, monochrome, and print behavior.
5. Verify optical centering inside actual platform icon masks.
6. Record the asset source, license, and export procedure in the release asset inventory.
