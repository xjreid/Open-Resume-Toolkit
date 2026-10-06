# Chrome store assets

These SVG exports preserve the approved October 4, 2026 Open Folio identity in
`Aesthetic/Logo`. The store icon fits the existing application master into 96×96
pixels with 16 pixels of transparent padding. Promo tiles use the existing
reversed mark and the current deep teal rail token; no replacement logo is drawn.

Run `node tools/capture-chrome-store-assets.mjs` after the Chrome production build.
Chrome renders the vectors and photographs the actual packaged extension's
selection/highlight UI on a fictional job page. It uses a temporary profile,
loopback server, and mocked native replies. It does not launch the installed ORT
application or use personal data. Screenshots demonstrate the extension rather
than claiming a production-signed companion installation.

Outputs: `artifacts/extension/chrome/store-listing/`, including the PNG assets and
a receipt with dimensions, RGB/alpha format, source provenance, and hashes.
