# PDF dependency and font provenance

This development slice pins Typst, typst-layout, typst-pdf and typst-assets to
0.15.1, comemo to 0.5.1, PDF.js/pdfjs-dist to 6.3.289 and base64 to 0.22.1.
Cargo.lock and pnpm-lock.yaml retain registry integrity values. No system font,
downloaded template, external renderer executable or remote CDN is used at runtime.

- [Typst 0.15.1 upstream release](https://github.com/typst/typst/releases/tag/v0.15.1)
  and its compiler crates use Apache-2.0. `typst-assets-LICENSE.txt` is the
  unmodified package license; `typst-assets-NOTICE.txt` is the complete upstream
  notice, including additional families/resources the World does not expose.
- [PDF.js 6.3.289 upstream release](https://github.com/mozilla/pdf.js/releases/tag/v6.3.289)
  uses Apache-2.0. `Apache-2.0.txt` is the package's complete license text.
- The selected `typst-assets::fonts()` entries are LibertinusSerif Regular,
  Bold, Italic, BoldItalic, Semibold and SemiboldItalic, in that order. They are
  unmodified OFL-1.1 fonts. Their copyright/reserved-name notices and complete
  OFL text are retained in `typst-assets-NOTICE.txt`. No fonts were renamed.
  Test-time family assertions and each render's SHA-256 identify the selected
  concatenated font bytes. The current bundle digest is recorded in the evidence.

The app exposes these full texts under PDF renderer and font licenses; notices
are included in the built frontend. The repository's own template is original
GPL-3.0-only source. Pinned Typst pulls additional text/layout/image/PDF/WASM
implementation crates; their presence does not grant resource access through the
fixed World. This checkpoint is not a complete transitive license/SBOM, RustSec,
release-package or security certification. Those broader release gates remain.
