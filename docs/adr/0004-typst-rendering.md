# ADR 0004: Pinned Typst rendering boundary

- Status: accepted in principle; implementation gated
- Target milestone: M2

## Decision

Use a pinned Typst toolchain and reviewed local fonts/templates for PDF rendering. User content is structured data and never executable Typst source.

## Consequences

The precise Typst embedding strategy and version are selected with M2 license, reproducibility, accessibility, and golden-render evidence. M0 contains no renderer.
