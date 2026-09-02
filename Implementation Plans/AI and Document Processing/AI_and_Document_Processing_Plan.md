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
- field-level change records used locally to derive no more than three concise user-visible change points;
- zero or more Required Qualification Alerts;
- response metadata needed for validation, not hidden chain-of-thought.

The result cannot change contact facts, dates, employers, degrees, certifications, skills, or achievements without a referenced source fact or explicit user-provided data. Rephrasing and selection/reordering are allowed within product rules. Local validation compares semantic fields to the input snapshot, rejects unreferenced claims, and applies length/section invariants.

A resume regeneration request is a separate refinement operation whose schema requires a nonempty `correctionInstruction`; the prompt includes that instruction, the current reviewed workspace resume, published-source revision, and reviewed job description. Cover-letter generation is not part of initial tailoring and starts only from its tab button. Question answering accepts one reviewed captured question and returns bounded editable plain text. Resetting a question is local and causes no provider call.

## Required Qualification Alert algorithm

Alerts are extracted in the same tailoring response to minimize tokens and keep requirement analysis consistent with the generated document. The operation uses this pipeline:

1. Identify statements explicitly expressed as mandatory (`required`, `must`, minimum, exact eligibility constraint) and exclude preferred/nice-to-have language.
2. Classify only the versioned allowlist: `degree_level`, `field_of_study`, `graduation_date`, `certification_or_professional_license`, `named_skill_or_technology`, `language_proficiency`, `experience_duration`, and `portfolio_or_work_sample`, subject to the category-specific restrictions in the product plan.
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

Category validators are local and explicit. Degree/date/certification/language comparisons use typed canonical values. Experience duration merges overlapping month intervals before comparison, counts only entries whose stored content/evidence resolves to the named domain, and rejects the candidate if relevance or dates are ambiguous. `portfolio_or_work_sample` supports `not_found` only. A category without a deterministic validator is unsupported even if present in the provider schema.

Personal or sensitive requirements are not inferred. If a resume literally contains a directly contradictory statement, the generic evidence rules may compare it; otherwise citizenship, sponsorship, authorization, disability, demographic status, security clearance, and similar unknowns are ignored rather than labeled `not_found`. This implements the approved “direct mismatch or resume-related not found; ignore unrelated/unverifiable” boundary.

Output bounds allow at most 10 validated alerts, a 500-character requirement excerpt, and a 500-character explanation per alert. Overflow produces a visible completeness note rather than unbounded output.

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

1. Check documented canonical platform install locations and package receipts. A sanitized PATH result or explicit user-selected path is only a locator candidate.
2. Resolve the canonical file and parent chain; reject symlink/reparse substitution, a writable download/temp/project/app-data location, unexpected ownership, or parent permissions that permit replacement by a less-trusted principal.
3. Match the strongest available official identity recorded in the signed compatibility manifest: expected platform signer/notarization requirement, package identity/receipt, and/or digest from an authenticated official release. A compatible filename, `--version` string, or protocol handshake is never sufficient.
4. Run any necessary bounded version/capability probe inside the external sandbox with no resume files, user roots, inherited secrets, or unrestricted network.
5. Require runtime and protocol versions to fall within the release compatibility manifest and advertised capabilities to match. A manual path cannot override identity, version, or containment failure.

### Isolated runtime

- separate ORT `CODEX_HOME`/configuration directory with user-only permissions;
- credentials configured for keyring storage;
- managed ChatGPT/device-code sign-in only for this product mode;
- empty ORT-owned working directory, no resume paths, minimal environment and inherited handles;
- approvals disabled and no tool/file/command/browser capability exposed;
- experimental app-server capabilities disabled; no dynamic tools, MCP, apps, skills, collaboration, filesystem, command, shell-command, process, or permission-profile APIs configured;
- child process tree tied to an OS job/lifecycle object and killed on timeout/cancel/app exit.

Requests contain only the minimized prompt data through app-server messages. Parse JSON-RPC-like frames with a bounded codec and an exact per-version allowlist for initialization, model/account reads, ephemeral thread/turn start, final structured output, cancellation, and shutdown. Reject method aliasing, duplicate IDs, unexpected direction, oversized/deep params, post-final messages, and unsupported additive events. Any unallowlisted request/notification or side-effect item—including `thread/shellCommand`, `command/*`, `process/*`, `fs/*`, tool, approval, permission, elicitation, MCP, app, skill, collaboration, web, file-change, or patch activity—kills the process tree and records `CODEX_CONTAINMENT_VIOLATION`; ORT never answers it.

Account/rate-limit information from app-server is normalized into timestamped snapshots with source method, runtime version, bucket identity, reset time, and freshness. Local token totals are never presented as provider account quota.

Codex remains a runtime-disabled feature until the process-level network/filesystem containment proof in the threat model passes for both operating systems. Compatibility loss disables new Codex operations but preserves local activity and documents.

## Import pipeline

Supported initial file inputs: text-based PDF and DOCX. Plain text remains an export format and may be pasted into manual fields, but a standalone `.txt` importer is not part of the initial file-import promise.

1. Native file dialog returns a one-use path token.
2. Import service verifies size, magic bytes/container, permissions, and supported type.
3. Copy/open the input into private staging and parse it in a new disposable OS-sandboxed worker with cancellation plus memory/CPU/wall-time/page/decompression/object/handle limits. The worker has no network, subprocess, database, vault, native-IPC, or general filesystem access.
4. Extract text plus structural hints; never execute macros, external relationships, scripts, or fetch remote content.
5. Detect scanned/image-only PDF when text density is below the documented threshold and return an actionable unsupported state; OCR is deferred.
6. Show extracted content plus the selected AI connection/provider/model and request confirmation before any transmission.
7. The versioned deterministic local mapper first proposes existing field/entry types from recognized structure. Unknown headings become proposed custom sections and unclassified text becomes a simple text/list proposal; nothing is silently discarded.
8. In No AI mode, show source and local proposal together for complete review with no network request. With a configured direct or Codex backend, offer a separate confirmed AI-mapping step that sends extracted text—not the original binary—and retains warnings and confidence/evidence links.
9. Only confirmed sections update the master draft. If AI mapping fails, preserve the complete local proposal and extracted text.

PDF extraction uses a pinned, hash-verified PDFium build behind a replaceable worker adapter after license/security review. DOCX reads the OPC/Open XML package with a constrained zip/XML parser in the same worker boundary. A parser crash or exploit attempt can fail the import but must not gain the desktop process's user-data or secret authority. Import correctness must not depend on Microsoft Word being installed.

### No-AI import core checkpoint (2026-09-02)

The import core is implemented in `ort-documents::import` and
`ort-application::import_review`, but is not connected to a desktop command or
file picker. The disposable worker remains inert and exits with code 78 even
when given an input argument. Passing these core tests is not sandbox evidence.

Extraction wire v1 contains only `version`, `format` (`pdf`/`docx`), `pageCount`,
and ordered `blocks` containing `page`, `kind` (`heading`/`paragraph`/`list_item`),
and `text`. The native parent supplies the expected format from its independently
validated input. Unknown/duplicate fields, malformed UTF-8/JSON, wrong versions,
format mismatch, invalid/out-of-order pages, and unsupported control characters
are rejected. Bounds are 512 KiB encoded result, 1,000 blocks, 30,000 characters
per block, 50,000 total characters, and 10 pages. Character limits count Unicode
scalar values; byte limits are separate. All-whitespace/empty extraction produces
an actionable no-readable-text result. This does not yet implement PDF text
density, partial-scan detection, or trustworthy DOCX pagination: parser adapters
must prove those independently, along with magic/container/resource checks.

Mapper v1 uses exact versioned heading aliases in English, Spanish, French,
German, and Chinese as review hints, not a claim of full language coverage.
Recognized headings retain their original spelling and receive a section-kind
hint. Explicit contact labels before the first section propose name/email/phone/
location values. Unlabeled names, employers, dates, skills, and URLs are not
guessed. Unknown heading hints become custom sections; other content remains
literal paragraph/list proposals. Richer entry/date/link normalization remains
unfinished. A list hint may remove one displayed bullet marker from the proposed
value, but the original block—including whitespace, marker, and line endings—
always remains intact. Oversized-for-draft blocks are retained and flagged for
editing/splitting, never silently truncated.

Each immutable source block has exactly one proposal identified by a locally
assigned source index. Review starts with no decisions and prepares nothing
until every block, including blank blocks, has an explicit disposition. Users
may edit/reclassify values, create custom sections, move text, merge into an
existing section by its stable ID, keep both sections, keep/replace conflicting
contact values, or reject content. Existing section names/IDs/content are not
implicitly renamed or replaced. Rejecting a heading with accepted children
requires explicitly moving or rejecting those children. Possible section
duplicates are hints only; semantic entry-level duplicate detection is pending.

`ImportReview::prepare` compares the complete base draft and revision with the
current saved value, builds a separate candidate, and validates all canonical
document limits before returning `SaveResumePayload`. It never writes a draft.
The caller must save with that expected revision in the existing storage CAS
transaction; a concurrent edit or replay cannot be overwritten by that save.
The caller retains review/source after any preparation or storage failure and
retires the review only after confirmed save success or explicit cancellation.
Publication remains separate. Pending source and decisions are memory-only;
debug output/errors do not contain extracted text or edited values. Retained
decision strings have a separate aggregate 100,000-character ceiling.

Next integration gates, in order:

The subsequent transport-policy checkpoint implements 8 KiB chunks, a 512 KiB
stdout cap, 16 KiB discarded-stderr cap, both-EOF/successful-OS-exit gating,
sticky failures, cancellation and a 60-second monotonic deadline. It is tested
with synthetic events only; actual native I/O, enforced resource ceilings,
termination and cleanup remain absent. `finish` is not a sandbox proof or an
import-enablement flag. See
`../System Documentation/Document_Worker_Containment.md` for platform research
and the required native probe matrix. A subsequent separate macOS XPC/App Sandbox
probe passed local descriptor, seeded filesystem and loopback checks, but allowed
child creation. Cooperative disconnect does not prove forced cleanup. No parser,
production sandbox adapter or UI was enabled; the gates below remain in force:

1. Prove supported native worker containment and supervision on macOS/Windows
   without credentials or real documents; do not replace it with a protocol-only
   check or an uncontained parser subprocess.
2. Add the native input capability, private staging, pinned parser adapters,
   and bounded pipe reader. It must stop/kill the worker on size overflow before
   collecting an unbounded result; `decode` is a second boundary, not the pipe
   reader. Data validation does not prove a worker's page/format claims.
3. Bind extraction to an app-owned review session/window and saved revision;
   add generated IPC records and the source/proposal review UI. The renderer
   must never supply raw worker responses or a file path as extraction authority.
4. Add finer-grained entry/date/link mapping, editable split/merge review,
   cancellation/expiry and source cleanup, real format fixtures, accessibility,
   native fault injection, and the complete offline import journey.

## Canonical document and renderer

The canonical resume contains ordered typed sections and stable IDs as specified in the product plan. It stores content and semantic style/template selections—not mutable PDF/DOCX layout fragments.

Use a pinned embedded Typst pipeline for preview and PDF export. Resume content is passed as data into fixed template functions; it is never concatenated into executable Typst source. Fonts and templates are bundled, versioned, licensed, and identified in every render receipt.

Application/website design tokens are unavailable to document templates. The default `technical_jakes_v1` adapter recreates the approved Jake's Resume reference structure—single column, conventional section hierarchy, restrained typography, ATS-readable text—only after upstream source/license attribution is recorded. If exact source reuse is incompatible, implement the visual structure independently and do not ship upstream source, branding, or unlicensed assets.

Preview displays the exact PDF bytes generated by the export renderer using bundled PDF.js or the platform-safe equivalent with no remote resources. A render receipt includes:

- canonical document hash/schema version;
- renderer/Typst version;
- template/font bundle versions;
- output format/options;
- warnings, page count, and timestamp.

Historical content stores its renderer tuple. If the exact old renderer is no longer shipped, ORT labels a regenerated export as using the current renderer rather than claiming byte-identical history.

Preview editing always mutates canonical structured content and re-renders; it never edits PDF bytes. Download writes the validated rendered bytes through an atomic native save flow. Overlay drag-out materializes those same bytes in a private session directory and provides a platform file-list payload; it is a convenience beside Download, not the only export path.

## DOCX and plain-text export

### Implemented M2 plain-text checkpoint (2026-09-02)

`export_resume_text` is a main-window-only command whose payload contains only
`source` (`saved_draft` or `published_snapshot`) and `expectedRevision`. Native
code loads that exact saved revision, validates it, and renders literal UTF-8
text. A newer revision requires explicit reload/reselection; renderer text is
never accepted as export input. The latest published snapshot and saved draft
are separate sources. This additive command remains on development contract v2.

Format v1 preserves canonical section/entry/field/bullet order and Unicode,
normalizes line endings to LF with one terminal newline, emits readable bullet
and link text, and omits empty fields, internal title/IDs, and app branding.
Other control characters cause an explicit error. Empty output is refused and
output is capped at 256 KiB. Formatting does not interpret HTML, Markdown, shell
text, or document directives, and does not invoke a parser or external renderer.

The Rust-owned Save dialog runs off the UI thread and yields a single-use
destination capability consumed inside that operation. No path/token is sent
through renderer IPC. One export may run at a time. The frontend has no dialog,
filesystem, shell, or process plugin permission. A held directory capability
anchors staging and publication after selection; a synced sibling payload is
published with a no-clobber hard link. Existing files, directories, and symlinks
are refused even after a target race. Filesystems without this primitive fail
closed. This checkpoint supports only new `.txt` files: confirmed replacement,
alternate-filesystem adapters, and historical render records remain future work.

Users must be warned that exported files and temporary staging are unencrypted
and may be visible to destination-folder users/sync services. Unix staging is
mode 0700 and output mode 0600; Windows currently inherits selected-folder ACLs
and requires native verification before release. Normal completion removes only
the exact operation-owned staging entries. Failed/crashed writes can leave a
hidden `.ort-export-*` sibling; cleanup recovery is not implemented and must not
later scan/delete arbitrary matching user directories. A committed-file receipt
separately reports cleanup failure and unconfirmed directory durability. No
automatic retry occurs after an uncertain IPC result. Export does not mutate
resume revisions or clear save errors; guarded quit waits for an active export.

### Remaining output implementation

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
- document: deterministic No-AI mapping, unknown-section preservation, import corpus, golden semantic output, links, pagination, Unicode, scanned detection;
- adversarial: prompt injection, schema bombs, fabricated qualifications, sensitive inference, unsupported Codex events;
- resource: maximum request/response/import/render limits;
- platform: parser-worker sandbox/kill/access-denial matrix and Codex identity/version discovery/auth/kill/containment matrix.

## Completion criteria

- No backend can bypass operation persistence, validation, accounting, and applicable guardrails.
- Tailoring produces a structured proposal, change summary, and validated alert set in one logical result.
- Alerts achieve the recorded false-positive gate and never manufacture sensitive-status conclusions.
- Preview/PDF use the same renderer tuple and DOCX/text preserve required semantics.
- No-AI PDF/DOCX import produces a complete reviewable local proposal, preserves unfamiliar extracted content, and never runs the parser in the desktop process or performs network I/O.
- All three direct providers pass common contract tests.
- Codex support is either backed by complete platform containment evidence or disabled in stable builds.
