# ADR 0005: Native messaging and external Codex are gated adapters

- Status: accepted boundary; implementations gated
- Target milestones: M5 development bridge, M6 optional Codex, M7 signed production bridge

## Decision

Chrome and Edge will share one Manifest V3 source and communicate through a separately authenticated native host. Optional Codex support may use only a separately installed, provenance-verified runtime inside a proven OS containment boundary.

## Consequences

Amended by the user's October 5, 2026 signing decision and October 7 roadmap
reorder: M0–M6 do not require paid Apple signing of ORT. M5 may qualify an explicitly identified current-user development
bridge with authenticated messages and documented limits. M7 owns required ORT
Developer ID signing/notarization, signed desktop/helper authentication and
production vault controls, and final Store distribution qualification. The
original default-production gate remains closed until that M7 work passes.
Official Codex-runtime provenance and containment requirements remain unchanged.

The M0 extension is inert and permission-free. Development native messaging is
enabled only in an explicit development build whose authentication and bounded
platform checks pass. Default production transport remains disabled until the
M7 identity and distribution checks pass. Codex remains absent unless every M6
containment requirement passes.
