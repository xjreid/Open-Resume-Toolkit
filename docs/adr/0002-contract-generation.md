# ADR 0002: Rust-owned generated contracts

- Status: accepted
- Milestone: M0

## Decision

Rust domain records own cross-process field definitions. A workspace generator emits checked-in JSON Schema, TypeScript types/runtime validation, and a compatibility manifest. CI regenerates these files and rejects drift.

## Consequences

Generated files are usable without Rust in extension builds but must never be edited manually. Cross-process changes require an explicit contract-version decision.
