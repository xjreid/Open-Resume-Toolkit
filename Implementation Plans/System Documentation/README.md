# System documentation

This folder owns cross-component technical decisions. Component plans may specialize these contracts but may not bypass them.

- [`Architecture.md`](Architecture.md) — processes, modules, dependency rules, data flows, and error boundaries.
- [`Repository_and_Build.md`](Repository_and_Build.md) — repository structure, generated contracts, build tools, dependencies, and CI.
- [`Development_and_Deployment_Outline.md`](Development_and_Deployment_Outline.md) — shared platform/browser ownership, build artifacts, environment identities, deployment order, release gates, rollback, and development and production qualification boundaries.
- [`Security_and_Threat_Model.md`](Security_and_Threat_Model.md) — trust boundaries, mitigations, and security evidence.
- [`Delivery_Roadmap.md`](Delivery_Roadmap.md) — milestones, release gates, and completion evidence.
- [`../Next_Milestones.md`](../Next_Milestones.md) — current M5–M7 acceptance and completed milestone references; the roadmap remains authoritative.
- [`Requirement_Traceability.md`](Requirement_Traceability.md) — stable technical requirement IDs mapped to product authority, implementation owner, and evidence.
- [`Technical_Reference_Baseline.md`](Technical_Reference_Baseline.md) — official upstream documentation used for the initial selections and items that must be reverified.

Architecture decision records are in `docs/adr/`, generated contracts in
`packages/contracts/generated/`, behavior references in `docs/application-behavior/`,
and current setup/CI guidance in [DEVELOPMENT.md](../../DEVELOPMENT.md) and
[docs/development/](../../docs/development/current-state.md). The component plans
include future release design; those sections are not proof of implemented features. Documentation and fixtures must never contain real resumes, credentials, signing secrets, or sensitive exploit instructions.
