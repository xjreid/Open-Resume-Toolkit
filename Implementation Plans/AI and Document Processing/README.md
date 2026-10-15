# AI and document-processing implementation

- [`AI_and_Document_Processing_Plan.md`](AI_and_Document_Processing_Plan.md) defines direct-provider and external Codex adapters, operation orchestration, accounting/guardrails, prompts and validation, Required Qualification Alerts, import, rendering, and export.
- Storage and UI representation are defined by the sibling local-data and desktop plans.

Provider endpoints, model IDs, prices, and Codex compatibility are versioned runtime/catalog facts. They must not be hard-coded as permanent product policy in this folder.

Direct-provider adapters, structured generation, durable accounting, rendering,
reviewed metered-Wasm import, and the optional macOS Codex integration are
implemented. UI keys select explicit catalog model IDs; Codex uses Start/Stop
server and memory-only authentication. See the
[current implementation map](../../docs/development/current-state.md) and
[Codex runtime guide](../../docs/ai/codex-plan-runtime.md) for tested boundaries.
