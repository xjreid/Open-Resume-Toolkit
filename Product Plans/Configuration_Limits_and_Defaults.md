# Configuration limits and quality defaults

## Purpose

This register centralizes protective limits and measurable defaults so they are not scattered or hard-coded inconsistently. Values are provisional release defaults unless explicitly described as fixed product behavior. Implementation testing may tighten a safety ceiling or improve a performance target; changing a user-visible limit requires coordinated documentation.

Every implemented value should have a stable name, unit, scope, default, rationale, version, and change history. The desktop, native host, extensions, renderer, importers, provider adapters, and backups must not disagree about a shared limit.

## Fixed product values

- One master-resume draft per local profile.
- At most one published master resume per local profile.
- At most one current application workspace per local profile.
- At most one selected final tailored resume, one selected cover letter, and one ordered approved question-answer set per tracker entry.
- Exactly one active AI connection mode per profile: no AI, one direct-API provider/credential/model preset, or one Codex subscription connection.
- No plan-based usage or retained-record quotas.
- No automatic expiry of the canonical current application workspace merely because it has been inactive; clearing requires explicit Finish Application, discard, replacement confirmation, or user data deletion.

## Platform compatibility defaults

- macOS: current public major release and two preceding major releases at each stable desktop release, on Apple silicon and Intel only where both the OS and selected framework remain supportable and tested.
- Windows: supported, fully patched Windows 11 on x64. Windows on ARM requires a separately built, signed, and tested package before advertising support.
- Chrome and Edge: current stable major and two preceding stable majors.
- Retest the compatibility matrix before each desktop, extension, or native-protocol release and at least monthly while preparing an active release.

Exact minimum version numbers are frozen in release documentation rather than inferred dynamically by users.

## Native messaging and IPC defaults

- Native-message envelope: 256 KiB maximum UTF-8 JSON in either direction.
- Captured job-description or question text in one message: 128 KiB maximum UTF-8.
- Message freshness: 60 seconds.
- Capability acknowledgement target: five seconds.
- One bounded desktop-launch attempt: up to 15 seconds.
- One idempotent retry after a fresh handshake.
- Protocol compatibility: current version and one immediately preceding version.
- Installation/IPC secret: at least 128 bits of cryptographically secure randomness unless equivalent OS access controls provide stronger assurance.

Schema-specific limits below remain tighter than the overall native-message envelope.

## Resume and structured-content defaults

- Master resume: 30,000 visible characters, 20 sections, 100 entries across sections, 500 bullets, 100 skill/keyword items, 25 links, and five rendered pages.
- Ordinary text field: 2,000 characters unless a narrower field rule applies.
- Individual bullet: 500 characters.
- Tailored resume: one- or two-page target selected by the user; up to five pages only after deliberate target expansion.
- Cover letter: one page by default and no more than two pages.
- Structured resume or cover-letter snapshot: 512 KiB uncompressed JSON and 128 KiB compressed.
- Question-answer set: at most 50 approved pairs, 256 KiB uncompressed JSON, and 64 KiB compressed.
- Combined structured artifacts for one tracker entry: 2 MiB uncompressed and 512 KiB compressed.
- JSON nesting: 20 levels maximum.
- Single JSON string: 30,000 characters maximum.

Limits protect rendering and parsing; they are not monetization quotas. When legitimate content exceeds a default, the interface explains the constrained field and offers export or reduction rather than silently truncating it.

## Import defaults

- Accepted initial types: text-bearing PDF and DOCX after signature/MIME validation.
- File size: 10 MiB maximum.
- Document length: 10 pages maximum.
- Extracted text: 50,000 characters maximum.
- Compressed-to-expanded safety ratio: 100:1 maximum.
- Local extraction target: 60 seconds on supported reference hardware.
- Scanned/image-only content and OCR: unsupported initially and detected before any provider call.

The parser also has explicit memory, CPU/time, recursion, embedded-object, relationship, and archive-expansion limits selected during implementation.

## Export and rendering defaults

- PDF: 10 pages and 20 MiB maximum.
- DOCX: 10 pages and 25 MiB maximum.
- Plain text: 2 MiB maximum.
- Portable full-data or backup archive: 1 GiB before a segmented-export strategy is required.
- Supported-maximum PDF render: p95 within five seconds and at least 99.5% successful after content validation.
- Supported-maximum DOCX render: p95 within 10 seconds and at least 99% successful after content validation.

Renderer failure is measured separately from an intentional validation rejection.

## AI request defaults

- Resume content per request: 30,000 normalized characters.
- Job-description content per request: 30,000 normalized characters.
- User instructions: 4,000 normalized characters.
- One application question: 4,000 normalized characters.
- Total provider input ceiling: 64,000 tokens or a lower provider/model limit.
- Initial tailoring output: 5,000 tokens maximum.
- Full-resume refinement output: 5,000 tokens maximum.
- Patch refinement output: 2,000 tokens maximum.
- Cover-letter output: 1,500 tokens maximum.
- One question-answer output: 800 tokens maximum.
- Resume-import proposal output: 6,000 tokens maximum.
- Tailoring/refinement attempt timeout: 120 seconds.
- Cover-letter/question attempt timeout: 90 seconds.
- Import attempt timeout: 180 seconds.
- Logical AI operation lifetime: 15 minutes.
- One active remote AI operation per local profile by default, preventing accidental overlapping charges and conflicting workspace changes.
- At most one automatic retry for an eligible transient failure; validation, authentication, safety, cancellation, size, and permanent-provider errors are never retried automatically.
- Retry delay begins at two seconds with jitter and caps at 30 seconds where an automatic retry is permitted.

Provider limits may be lower. ORT reports those errors without silently changing credentials, provider, or model. Lower schema-specific output limits should be used whenever adequate.

## Workspace and temporary-data defaults

- Application-workspace structured content: 4 MiB uncompressed maximum, excluding user-controlled exports outside the profile.
- Extension capture is cleared after desktop acknowledgement, user cancellation, or a failed-send recovery window selected during implementation.
- Completed AI request working buffers and temporary response files: remove as soon as safely committed or no later than one hour after operation completion.
- Abandoned import working files that never enter active review: remove within 24 hours.
- Stale ORT-owned preview/export temporary files: sweep at startup and at least daily; files outside ORT-controlled temporary directories are never deleted automatically.
- Content-free local diagnostic logs: disabled by default where practical or retained no longer than 30 days when enabled. Size rotation provides an additional cap.

## AI activity and pricing defaults

- AI activity history: retained until the user clears it by default; optional age-based policies are 30 days, 90 days, or one year.
- Activity timestamps: stored in a stable UTC representation and displayed in the user's current local time zone.
- Cost display: disabled for an attempt when the applicable model, usage category, currency, or price cannot be mapped reliably; ORT shows **unavailable** rather than zero.
- Historical estimates: preserve the currency and pricing-catalog version/effective date used when the attempt finalized; never silently recalculate after a catalog update.
- Usage aggregation: logical-operation totals and provider-call-attempt totals remain separately available so retries do not appear to be new user operations.
- Activity export: CSV and JSON are supported and contain non-content fields only.
- Pricing catalog: versioned with each supported provider/model preset; signed independently for content-only updates through canonical GitHub release infrastructure; and reviewed whenever a preset/provider price changes and before each release that advertises cost estimates. Entries carry official source, verification, and effective/expiry metadata.
- Initial direct providers: OpenAI, Anthropic, and Google Gemini only. Curated Economy, Balanced, and Quality presets are selected from the dated research baseline in `AI_and_Import.md`; Balanced is the initial direct-API default after provider configuration.
- Model discovery: provider availability may hide a preset, but never enables an untested model. A selected model that becomes unavailable blocks with an explicit replacement choice and never silently reroutes.
- Provider/model summaries: show logical operations, attempts, reported input, cached/cache-write, output, reasoning, total and other supported token categories, estimated cost, and missing/unpriced counts. Cross-currency totals are not computed.

## Direct-API guardrail defaults

- Spending limits: disabled until deliberately enabled by the user.
- Supported periods: calendar week, calendar month, calendar year, and all time; any or all may be enabled simultaneously for the active credential identity.
- Initial baseline: each cap starts at zero when it is enabled and displays that activation time. It does not retroactively claim to include earlier ORT calls or outside provider use.
- Calendar boundary: the user's current local time zone at policy creation; weeks begin Monday at 00:00. The selected zone and next reset are stored so travel or a later zone change does not move a period silently.
- Currency: the pricing catalog's billing currency for the active provider/model. Initial catalogs use official USD list prices; automatic foreign-exchange conversion is unsupported.
- Notifications: local warnings at 50% and 80% of each enabled cap and a blocking notice at 100%. Duplicate notifications are suppressed within a period unless a new threshold is crossed or policy changes.
- Enforcement: atomic preflight reservation includes the counted estimate, unresolved reservations, estimated/provider-counted input, maximum configured output, and all applicable price components. Any enabled cap with missing, expired, unverified, or otherwise unevaluable pricing fails closed.
- Unknown outcome: retain the conservative reservation as unresolved until trustworthy usage settles it or the user explicitly reconciles it after checking provider records.
- Activity clearing: never resets guardrail period totals or unresolved reservations. An all-time baseline reset is a distinct confirmed action.

## Codex subscription defaults

- Authentication: managed ChatGPT browser flow through local Codex app-server, with device-code fallback where available; API-key auth is not accepted in Codex subscription mode.
- Model selection: intersect picker-visible `model/list` results with ORT-tested models; prefer the service-recommended tested model. The initial compatibility candidates are GPT-5.6 Luna, Terra, and Sol.
- Reasoning: use the returned model default initially; expose only supported efforts that ORT has evaluated. Ultra/subagent operation is unsupported.
- Execution: one ephemeral thread per ORT operation in an empty ORT scratch directory; `approvalPolicy: never`; restricted read-only/external process sandbox; provider-only network egress; no configured dynamic tools, MCP, apps, plugins, skills, collaboration, or web access; fail on any tool/command/file event; and cleanup immediately after the activity record settles.
- Quota refresh: on connection, when AI Activity opens, immediately before dispatch when a cap is enabled, after an operation, and on provider update notifications. Background polling is bounded and does not create ORT telemetry.
- Usage caps: disabled until enabled. A threshold from 1% through 100% may apply to an individual stable quota-bucket identifier or all currently returned buckets. Failure to refresh an enabled cap blocks dispatch.
- Codex costs: do not show API-equivalent currency estimates. Show provider-reported quota windows and token activity with account-wide versus ORT-only provenance.

## Update defaults

- Background application update check: at most once every 24 hours by default, plus explicit user-triggered checks.
- Background signed pricing/model-catalog check: at most once every 24 hours by default, plus an explicit refresh. A valid cached catalog remains usable until its applicable entry expires.
- Update and download requests contain no resume/application content or AI credentials.

## Selection and change process

1. Implementation plans map every value to the responsible component and tests.
2. Test values against representative short, long, malformed, multilingual, and adversarial inputs.
3. Tighten limits when required for security, platform, library, provider, or usability constraints.
4. Record the effective version and include material user-visible changes in release notes.
5. Never truncate, delete, charge, transmit, or overwrite content merely because a new limit was introduced; provide migration or corrective actions.
