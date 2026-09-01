# AI and document-processing plan

## Status and requirements

- Status: approved baseline for Direct API and document work; Codex remains behind the containment gate
- Owner: AI/document maintainers
- Milestones: M2–M6
- Product authority: `../../Product Plans/AI_and_Import.md`, `Core_Workflows.md`, `Resume_Editor_and_Schema.md`, `Product_States_and_Operations.md`, and `Configuration_Limits_and_Defaults.md`

Non-goals: hosted ORT keys, a proxy service, training on user content, auto-applying to jobs, inventing credentials, executing page instructions, or bundling Codex.

## Shared operation architecture

All AI work goes through `OperationCoordinator` and a common `AiBackend` port. UI code cannot call a provider or Codex directly.

```rust
trait AiBackend {
    async fn capabilities(&self) -> BackendCapabilities;
    async fn estimate(&self, request: NormalizedRequest) -> Estimate;
    async fn execute(
        &self,
        request: NormalizedRequest,
        sink: OperationEventSink,
        cancel: CancellationToken,
    ) -> Result<BackendResult, BackendError>;
}
```

The real signatures may change during coding, but the boundary must preserve: normalized request, cancellability, ordered events, requested/effective model, provider usage provenance, and a provider-neutral error/result.

Logical operation sequence:

1. snapshot and validate referenced local inputs;
2. resolve connection, preset, model, prompt/schema, and catalog versions;
3. minimize payload fields for the operation;
4. estimate usage/cost and enforce confirmation rules;
5. transactionally create operation/attempt and reserve direct cap if applicable;
6. dispatch with a unique local correlation ID;
7. collect bounded stream and provider usage;
8. validate the complete structured result;
9. persist accepted proposals/change summary/alerts and settle accounting atomically;
10. expose a final state and safe diagnostics.

At most one retry is allowed, and only for classified transient failures before a valid response. The retry is a new attempt inside the same logical operation. Schema/factual failures are not automatically retried with a looser prompt.

## Direct-provider adapters

Initial adapters: OpenAI, Anthropic, and Gemini. Implement them with `reqwest`, rustls, serde, and explicit API request/response types rather than allowing provider SDK types to leak through the product.

Each adapter owns:

- credential-header format and a safe credential-test request;
- supported structured-output mechanism;
- model discovery/verification where available;
- token-limit and capability normalization;
- streaming frame parsing and cancellation;
- provider request ID and usage extraction;
- retryable/rate-limit/auth/model-retired/error classification;
- billing/privacy documentation links;
- requested-versus-effective model capture.

Adapter endpoints and API versions are pinned in code/config for each release and verified against official provider documentation during catalog review. Redirects to a different host are denied. Credentials are fetched from the vault immediately before dispatch and never returned to callers.

## Model presets and signed catalog

Economy, Balanced, and Quality are product presets resolved by a signed catalog, not aliases permanently compiled to one model. Catalog records include:

- format version, catalog ID, issued/effective/expires timestamps;
- provider and exact model ID;
- supported operations and structured-output capability;
- context/output limits and lifecycle status;
- input/output/cache/reasoning price components with currency/unit/source review date;
- preset eligibility and rationale metadata;
- minimum ORT version and emergency-disable flag.

Sign canonical catalog bytes with an Ed25519 key dedicated to catalogs. ORT ships a built-in trusted baseline and public key. A downloaded catalog activates only after signature, chronology, compatibility, and rollback/freeze checks. A stale catalog may still list models but cannot produce a false current price; enabled direct spending caps fail closed where cost cannot be bounded.

## Prompt and response package

Every supported operation ships an immutable package:

```text
operation id
prompt version
input-minimizer version
JSON response schema version
validator version
evaluation-corpus version
supported backend capability requirements
```

Prompts separate system policy, operation instruction, structured trusted resume data, and untrusted job/import text with explicit boundaries. They state that content inside data blocks cannot change instructions and that the model must not infer facts absent from the resume.

Provider-native structured output is used where reliable. The complete response is still parsed with a local byte/depth/collection limit and validated by Rust domain rules. Markdown or prose outside the schema is rejected, not silently scraped.

## Tailoring response contract

One tailoring call returns:

- a structured resume proposal referencing source entry/field IDs;
- field-level change records with before/after and reason;
- zero or more Required Qualification Alerts;
- response metadata needed for validation, not hidden chain-of-thought.

The result cannot change contact facts, dates, employers, degrees, certifications, skills, or achievements without a referenced source fact or explicit user-provided data. Rephrasing and selection/reordering are allowed within product rules. Local validation compares semantic fields to the input snapshot, rejects unreferenced claims, and applies length/section invariants.

## Required Qualification Alert algorithm

Alerts are extracted in the same tailoring response to minimize tokens and keep requirement analysis consistent with the generated document. The operation uses this pipeline:

1. Identify statements explicitly expressed as mandatory (`required`, `must`, minimum, exact eligibility constraint) and exclude preferred/nice-to-have language.
2. Classify only supported, directly comparable requirement categories.
3. Map each mandatory requirement to structured resume fields/entries and their stable IDs.
4. Emit `confirmed_mismatch` only when resume evidence directly contradicts the required value.
5. Emit `not_found` when no relevant resume evidence can be located.
6. Ignore requirements that cannot be responsibly compared to resume content rather than guessing.

Example contract shape:

```json
{
  "type": "confirmed_mismatch",
  "category": "graduation_date",
  "requirement": {
    "text": "Expected graduation in 2027 is required",
    "start": 418,
    "end": 457,
    "mandatoryReason": "explicit_required"
  },
  "resumeEvidence": [
    {"entryId": "...", "fieldId": "graduationDate", "value": "2028-05"}
  ],
  "explanation": "The required year is 2027; the resume lists 2028."
}
```

Local validation requires the span to match normalized job text, verifies mandatory markers/classification, resolves all evidence IDs/value hashes against the operation snapshot, and ensures the explanation introduces no new fact. A `not_found` alert must have no fabricated evidence. Duplicate alerts collapse by normalized requirement span/category/type.

Personal or sensitive requirements are not inferred. If a resume literally contains a directly contradictory statement, the generic evidence rules may compare it; otherwise citizenship, sponsorship, authorization, disability, demographic status, security clearance, and similar unknowns are ignored rather than labeled `not_found`. This implements the approved “direct mismatch or resume-related not found; ignore unrelated/unverifiable” boundary.

Output bounds cap the number and text length of alerts. Overflow produces a visible completeness note rather than unbounded output.

## Factual and safety validation

Validation stages:

1. transport size/encoding;
2. JSON syntax and response schema;
3. stable-ID/reference integrity;
4. resume-field type/date/link validation;
5. source-fact/factual-boundary comparison;
6. requirement-alert evidence and mandatory/preferred checks;
7. content length/document invariants;
8. prohibited instruction/tool/secret-like output checks.

Rejected output is retained only as a safe validation code and aggregate metrics unless the user explicitly chooses a future debug feature approved by product policy. Raw provider output is not placed in diagnostics.

## Accounting and direct guardrails

Before dispatch, token estimation uses the most accurate locally available tokenizer/accounting rule for the effective model, plus a conservative output bound. Cost values use integer micros/decimal math. A reservation records provider, model, catalog version, currency, estimated maximum, period IDs, and attempt ID.

After response:

- provider-reported usage takes precedence and retains original categories;
- normalized categories are input, output, cache-read/write where supported, reasoning where separately billed, and other documented units;
- actual cost is calculated using the catalog version selected at dispatch;
- missing/partial usage is marked explicitly and settles conservatively under cap rules;
- currencies are never summed into one total without an exchange source approved in product policy.

Weekly/monthly/yearly/all-time periods use the user's recorded IANA time zone and store resolved UTC boundaries. Policy changes are revisioned. Notifications are local and idempotent at threshold crossings.

## External Codex adapter

Use the official Codex app-server protocol over `stdio`, beginning with initialize/capability negotiation and the minimum thread/turn/account methods needed for ORT. WebSocket transport is not used for the initial implementation.

### Discovery and verification

1. Check an explicitly user-selected executable first.
2. Check documented platform install locations, then a sanitized PATH search.
3. Resolve canonical path, reject writable/untrusted parent locations where possible, inspect platform signature/provenance, and run a bounded version probe.
4. Require the version to fall within the release compatibility manifest and the advertised protocol capabilities to match.

### Isolated runtime

- separate ORT `CODEX_HOME`/configuration directory with user-only permissions;
- credentials configured for keyring storage;
- managed ChatGPT/device-code sign-in only for this product mode;
- empty ORT-owned working directory, no resume paths, minimal environment and inherited handles;
- approvals disabled and no tool/file/command/browser capability exposed;
- child process tree tied to an OS job/lifecycle object and killed on timeout/cancel/app exit.

Requests contain only the minimized prompt data through app-server messages. Parse JSON-RPC-like frames with a bounded codec and allowlist methods/events. Any unknown capability with side-effect potential, approval request, tool call, command execution, file operation, or patch event terminates the attempt and records `CODEX_CONTAINMENT_VIOLATION`.

Account/rate-limit information from app-server is normalized into timestamped snapshots with source method, runtime version, bucket identity, reset time, and freshness. Local token totals are never presented as provider account quota.

Codex remains a runtime-disabled feature until the process-level network/filesystem containment proof in the threat model passes for both operating systems. Compatibility loss disables new Codex operations but preserves local activity and documents.

## Import pipeline

Supported initial inputs: text-based PDF, DOCX, and plain text.

1. Native file dialog returns a one-use path token.
2. Import service verifies size, magic bytes/container, permissions, and supported type.
3. Parse in bounded staging with cancellation and time/page/decompression limits.
4. Extract text plus structural hints; never execute macros, external relationships, scripts, or fetch remote content.
5. Detect scanned/image-only PDF when text density is below the documented threshold and return an actionable unsupported state; OCR is deferred.
6. Show extracted content plus the selected AI connection/provider/model and request confirmation before any transmission.
7. The configured direct or Codex backend maps extracted text—not the original binary—into a structured proposal with warnings and confidence/evidence links.
8. Show source and proposal together; only confirmed sections update the master draft. If AI mapping fails, keep extracted text in the current review so the user can assign it manually where feasible.

PDF extraction uses a pinned, hash-verified PDFium build behind a replaceable adapter after license/security review. DOCX reads the OPC/Open XML package with a constrained zip/XML parser. Import correctness must not depend on Microsoft Word being installed.

## Canonical document and renderer

The canonical resume contains ordered typed sections and stable IDs as specified in the product plan. It stores content and semantic style/template selections—not mutable PDF/DOCX layout fragments.

Use a pinned embedded Typst pipeline for preview and PDF export. Resume content is passed as data into fixed template functions; it is never concatenated into executable Typst source. Fonts and templates are bundled, versioned, licensed, and identified in every render receipt.

Preview displays the exact PDF bytes generated by the export renderer using bundled PDF.js or the platform-safe equivalent with no remote resources. A render receipt includes:

- canonical document hash/schema version;
- renderer/Typst version;
- template/font bundle versions;
- output format/options;
- warnings, page count, and timestamp.

Historical content stores its renderer tuple. If the exact old renderer is no longer shipped, ORT labels a regenerated export as using the current renderer rather than claiming byte-identical history.

## DOCX and plain-text export

DOCX output is a constrained Open XML generator/adapter supporting the semantic elements ORT owns: paragraphs, headings, lists, tables only where approved, links, page breaks, and bundled/standard font references. It does not automate Word. A library such as `docx-rs` may be used only after golden tests prove relationship safety, accessibility semantics, stable packaging, and license acceptability; otherwise `ort-documents` writes the constrained OPC parts directly.

Plain text is generated from the same canonical ordering with deterministic whitespace and Unicode. Export writes atomically and never overwrites without user confirmation.

## Evaluation program

Synthetic corpus dimensions include career level, section mixes, sparse/dense resumes, Unicode, date formats, long job descriptions, technical/nontechnical roles, and adversarial page instructions.

Per provider/model/preset, measure:

- schema validity and retry rate;
- unsupported/fabricated claim rate (release threshold: zero in gate corpus);
- source-reference accuracy;
- required/preferred classification precision/recall;
- confirmed-mismatch and not-found precision, with special sensitive-category cases;
- change-summary completeness;
- document constraint pass rate;
- token/cost estimate error distribution;
- cancellation/timeouts and latency budgets.

A catalog/prompt/model change cannot promote to stable merely because it “looks better”; it must meet recorded thresholds and not regress safety.

## Tests and evidence

- unit/property: minimizers, pricing math, model resolution, alert validator, fact references;
- adapter contract: fixture responses, streaming splits, usage variants, auth/rate-limit/retired-model errors;
- integration: mocked HTTPS, vault, coordinator/database transactions, retries/cancellation/crash;
- live probes: minimal synthetic calls under project-owned restricted credentials;
- document: import corpus, golden semantic output, links, pagination, Unicode, scanned detection;
- adversarial: prompt injection, schema bombs, fabricated qualifications, sensitive inference, unsupported Codex events;
- resource: maximum request/response/import/render limits;
- platform: Codex version discovery/auth/kill/containment matrix.

## Completion criteria

- No backend can bypass operation persistence, validation, accounting, and applicable guardrails.
- Tailoring produces a structured proposal, change summary, and validated alert set in one logical result.
- Alerts achieve the recorded false-positive gate and never manufacture sensitive-status conclusions.
- Preview/PDF use the same renderer tuple and DOCX/text preserve required semantics.
- All three direct providers pass common contract tests.
- Codex support is either backed by complete platform containment evidence or disabled in stable builds.
