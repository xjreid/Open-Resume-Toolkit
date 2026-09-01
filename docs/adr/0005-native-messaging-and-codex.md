# ADR 0005: Native messaging and external Codex are gated adapters

- Status: accepted boundary; implementations gated
- Target milestones: M5 and M6

## Decision

Chrome and Edge will share one Manifest V3 source and communicate through a separately authenticated native host. Optional Codex support may use only a separately installed, provenance-verified runtime inside a proven OS containment boundary.

## Consequences

The M0 extension is inert and permission-free. Native messaging stays disabled until M5 authentication and platform tests pass. Codex remains absent unless every M6 containment requirement passes.
