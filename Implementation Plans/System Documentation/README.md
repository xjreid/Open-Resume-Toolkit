# System documentation

This folder owns cross-component technical decisions. Component plans may specialize these contracts but may not bypass them.

- [`Architecture.md`](Architecture.md) — processes, modules, dependency rules, data flows, and error boundaries.
- [`Repository_and_Build.md`](Repository_and_Build.md) — repository structure, generated contracts, build tools, dependencies, and CI.
- [`Development_and_Deployment_Outline.md`](Development_and_Deployment_Outline.md) — shared platform/browser ownership, build artifacts, environment identities, deployment order, release gates, rollback, and concrete M0 handoff.
- [`Security_and_Threat_Model.md`](Security_and_Threat_Model.md) — trust boundaries, mitigations, and security evidence.
- [`Delivery_Roadmap.md`](Delivery_Roadmap.md) — milestones, release gates, and completion evidence.
- [`Requirement_Traceability.md`](Requirement_Traceability.md) — stable technical requirement IDs mapped to product authority, implementation owner, and evidence.
- [`Technical_Reference_Baseline.md`](Technical_Reference_Baseline.md) — official upstream documentation used for the initial selections and items that must be reverified.

Future implementation work should add architecture decision records under `adrs/`, protocol schemas under the source tree described in `Repository_and_Build.md`, and release runbooks under `runbooks/`. Documentation and fixtures must never contain real resumes, credentials, signing secrets, or sensitive exploit instructions.
