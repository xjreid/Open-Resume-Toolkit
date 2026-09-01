# AI, credentials, and resume import

## AI operating model

ORT operates no shared AI account and pays for no user inference. AI is optional and has exactly one active connection mode per local profile:

1. **Direct API** — a user-supplied API key for OpenAI, Anthropic, or Google Gemini.
2. **Codex subscription** — a user-authorized ChatGPT/Codex session reached through the local Codex app-server interface.
3. **No AI** — manual editing, tracking, rendering, import review where possible, and export remain available.

Direct provider calls go from the desktop application to the selected provider over authenticated HTTPS. Codex subscription calls go through a locally launched, version-compatible Codex app-server process to OpenAI. ORT operates no AI gateway, relay, logging proxy, shared account, or fallback credential service.

Direct API and Codex subscription modes are mutually exclusive. A user may retain vault entries for more than one direct provider, but only one provider credential and one model preset are active. Switching modes or providers is explicit, applies only to future operations, and never causes a silent retry or fallback through the previously active connection.

Local-model support remains a later adapter. No provider, subscription, or model is a centrally funded entitlement or permanent hard-coded dependency.

## Initial providers and model adaptability

The initial direct-API adapters are limited to OpenAI, Anthropic, and Google Gemini. Each adapter exposes only a curated intersection of models that are available to the user's credential and that ORT has tested for structured output, factual constraints, latency, token reporting, and cost behavior. A provider's model-list endpoint may confirm availability, but it does not automatically make an untested or incompatible model selectable.

The research baseline verified on August 31, 2026 is:

| Provider | Economy | Balanced default | Quality |
| --- | --- | --- | --- |
| OpenAI API | `gpt-5.6-luna` | `gpt-5.6-terra` | `gpt-5.6-sol` |
| Anthropic API | `claude-haiku-4-5-20251001` | `claude-sonnet-5` | `claude-opus-5` |
| Gemini API | `gemini-3.5-flash-lite` | `gemini-3.6-flash` | `gemini-3.7-flash` |

These are implementation candidates, not permanent product promises. Before every release, maintainers must verify current model identifiers, lifecycle status, account availability, features, and prices against official provider documentation and re-run ORT's evaluation set. A removed, deprecated, or failing model is disabled for new selections without rewriting historical activity.

For implementation comparison only, the same August 31, 2026 research snapshot records standard direct-API USD list prices per one million text tokens as follows. It is not a runtime price source and must not be copied into code without effective-date/version metadata:

| Provider/model | Input | Cached input | Output | Important condition |
| --- | ---: | ---: | ---: | --- |
| OpenAI GPT-5.6 Luna | $0.20 | $0.02 | $1.20 | Additional long-context/cache-write rules may apply. |
| OpenAI GPT-5.6 Terra | $2.00 | $0.20 | $12.00 | Additional long-context/cache-write rules may apply. |
| OpenAI GPT-5.6 Sol | $4.00 | $0.40 | $20.00 | Additional long-context/cache-write rules may apply. |
| Anthropic Claude Haiku 4.5 | $1.00 | Provider-specific | $5.00 | Cache writes/reads have separate rates. |
| Anthropic Claude Sonnet 5 | $2.00 | Provider-specific | $10.00 | Cache writes/reads have separate rates. |
| Anthropic Claude Opus 5 | $5.00 | Provider-specific | $25.00 | Cache writes/reads and optional service modes have separate rates. |
| Gemini 3.5 Flash-Lite | $0.30 | $0.03 | $2.50 | Standard paid tier; output includes thinking tokens. |
| Gemini 3.6 Flash | $0.75 | $0.075 | $3.75 | Promotional standard rate through December 31, 2026; scheduled to change afterward. |
| Gemini 3.7 Flash | $0.75 | $0.075 | $3.75 | Promotional standard rate through December 31, 2026; scheduled to change afterward. |

Official maintenance sources are the [OpenAI model catalog](https://developers.openai.com/api/docs/models), [Anthropic model overview](https://platform.claude.com/docs/en/models/overview), [Anthropic pricing documentation](https://platform.claude.com/docs/en/about-claude/pricing), [Gemini model catalog](https://ai.google.dev/gemini-api/docs/models), and [Gemini API pricing](https://ai.google.dev/gemini-api/docs/pricing). Exact cache, long-context, batch/flex/priority, tool, regional, free-tier, and promotional rules must be represented as conditional catalog components rather than flattened into one misleading rate.

The model selector shows the preset purpose, provider model name, tested ORT operations, context/output constraints, reasoning setting when applicable, and the current catalog's input, cached-input/cache-write, output, reasoning, and fixed/tool prices that can affect ORT. Prices include currency, unit, effective-from/effective-to or verified-through dates, official source, and any applicable size or service-tier condition. Unsupported, expired, or unverified price dimensions show **unavailable**, never zero. The UI may show an operation-specific cost projection, clearly labeled as an estimate.

## Direct-API credentials

- The desktop app receives direct-provider API keys from the user.
- Credentials are stored in Windows Credential Manager, macOS Keychain, or an equivalently protected OS facility.
- Credentials never appear in the local content database, logs, backups by default, browser-extension storage, native-messaging payloads, exports, crash reports, or diagnostic bundles.
- The user can test, replace, and remove a credential.
- A failed credential never causes silent fallback to another provider or key.
- Provider-specific usage and billing links should be available so the user understands that charges come from that provider.
- Every saved direct credential receives a random local credential identifier. Activity and guardrail records use that identifier and never a key value, key prefix, or reversible fingerprint.
- Replacing a key creates a new credential identity. ORT asks whether to copy the prior guardrail settings, but never carries an all-time spend counter to a new identity silently.

## Codex subscription connection

Codex subscription mode is a separate choice from the direct API-key menu. The initial ORT package does not bundle Codex. When the user chooses this mode, ORT detects a separately installed compatible Codex runtime, validates its executable identity, version, app-server protocol, required methods, and model capabilities, and gives platform-appropriate installation or update guidance when it is absent or incompatible. ORT then launches `codex app-server` as an app-managed child process over local `stdio`; the user is not required to start or keep a server running independently. Initial production support does not use app-server WebSocket transport.

ORT starts the runtime with an ORT-specific configuration/authentication root, keyring namespace, and process environment, requests managed ChatGPT authentication, opens the returned browser sign-in page, and lets Codex own token persistence and refresh. Device-code login may be offered as a fallback. ORT must not scrape browser cookies, request a ChatGPT password, copy, inherit, or mutate the user's general Codex CLI/desktop configuration and authentication files, or route an OpenAI API key through this mode. An existing general Codex installation may provide the executable, but ORT still creates and owns its separate configuration and authentication boundary.

After sign-in, ORT reads the minimum auth/plan state, current quota windows, token-activity summaries when available, and the account's current model list. An email address returned during connection may be masked for immediate confirmation but is not required or persisted. The Codex model picker displays only the intersection of picker-visible models returned by `model/list` and ORT's tested compatibility catalog. The initial compatibility candidates are the GPT-5.6 Luna, Terra, and Sol family; the service-recommended tested model is selected by default, and the user may choose another tested model and supported reasoning effort.

Each ORT Codex operation uses an ephemeral thread and an ORT-owned empty scratch directory. ORT does not configure dynamic tools, MCP servers, apps, plugins, skills, collaboration, or web access. It starts the turn with `approvalPolicy: never`, a restricted read-only/external sandbox, no access to user files, and process-level network egress limited to the provider endpoints required by app-server. ORT sends only the reviewed operation content, accepts only the final structured response, validates it through the same gates as direct providers, and tears down the thread and scratch data after settlement. Any command, file-change, tool, web, connector, or permission event interrupts and fails the operation even if a future app-server version would otherwise allow it. ORT stops the app-server child process when the connection is closed and detects and recovers an orphaned or interrupted child on the next launch. Shipping requires adversarial proof that this effective containment works on every supported platform; the plan does not assume app-server is a tool-free inference API.

The implementation plan must define supported minimum and maximum runtime/protocol versions, schema generation and negotiation, executable discovery and verification, installation guidance, child-process supervision, timeouts, health checks, incompatible-version behavior, and safe disablement. ORT never installs, upgrades, or modifies the external runtime silently. A future optional companion installer or bundled runtime requires a new product decision plus license, package-size, signing, update, SBOM, and user-consent review. The integration contract must be maintained against the official [Codex app-server documentation](https://developers.openai.com/codex/app-server), [Codex authentication documentation](https://developers.openai.com/codex/auth), and [Codex model guidance](https://developers.openai.com/codex/models).

## Provider transparency

Before every new class of transmission—and again whenever the selected provider changes—ORT explains:

- Which provider and model will receive the request.
- Which resume, job, question, or instruction content is included.
- That the provider's terms, retention, privacy, rate limits, direct charges or subscription quota, and account eligibility apply.
- That ORT cannot recover provider accounts, refund provider charges, or guarantee provider availability.

The application estimates input size before dispatch and surfaces provider-reported usage after a response when reliably available. Pricing calculations are informational and must not be represented as the provider's final bill. In Codex subscription mode, ORT shows subscription quota and token activity rather than inventing an API-equivalent dollar charge.

## Local usage and cost accounting

Every remote provider call made by ORT passes through one standardized local accounting boundary, regardless of the provider adapter or product workflow. Before network dispatch, ORT durably creates an attempt record so that a crash, cancellation, timeout, or ambiguous response cannot make a potentially billable or quota-consuming call disappear from AI Activity.

For each logical operation and provider-call attempt, ORT records only the non-content metadata defined in the local data model, including:

- Operation and attempt identifiers and operation type
- Connection mode, provider, requested and effective model, and tested preset version
- Start/end time, duration, terminal status, and retry relationship
- Provider-reported input, output, cached, reasoning, or other separately billed usage categories when available
- Locally estimated input size when provider-reported usage is unavailable
- Cost estimate, currency, applicable pricing components, pricing-catalog version/effective date, and whether the estimate is complete, partial, or unavailable
- Coarse local/provider error category without response bodies or sensitive request details

Provider-reported usage values remain distinguishable from local estimates. A versioned local pricing catalog uses documented public provider prices that are verified during preset maintenance and bundled with application releases. Between application releases, ORT may check a signature-verified, content-only catalog published through the canonical GitHub release infrastructure; the request contains no user content, credential, or stable profile identifier, and the user may trigger it manually. A catalog update may change pricing/lifecycle metadata but cannot install code or automatically enable an untested model. A historical record preserves the estimate and catalog version used at the time; a later price update does not silently rewrite earlier history. Providers may apply account tiers, cached-input rules, batch discounts, credits, taxes, minimums, tool/image charges, or later adjustments that ORT cannot know, so all calculated currency values are labeled **estimated**.

AI Activity provides two levels of direct-API aggregation:

- **Per model** — logical operations, attempts, input, cached/cache-write, output, reasoning, total and other provider-reported token categories, plus contemporaneous estimated cost.
- **Per provider** — the same totals across that provider's models, with historical model identifiers retained.

Aggregates never silently add unlike currencies or incompatible pricing bases. If any attempt in a group is missing reportable usage or price data, the group is marked **partial** and shows the excluded/unknown count. Users can switch between logical-operation and provider-attempt views so retries are not mistaken for unique work.

AI Activity describes only calls routed through the local ORT installation. ORT does not claim to inspect all use of an API key, reconcile a complete provider account, or replace the provider's usage and billing tools. Removing a provider credential does not erase existing content-free activity history; those records have separate export, retention, and deletion controls.

Changing a provider/model from AI Activity uses the same validated presets, disclosure, connection testing, and no-silent-fallback rules as Settings. A change applies to future attempts only and cannot alter an active or historical operation.

If a provider reports that it rerouted or substituted the requested model, ORT records both identifiers and does not promote the result unless the effective model is in the compatible tested catalog and the applicable pricing/guardrail decision remains safe. Otherwise the operation fails visibly; rerouting is never treated as permission to bypass model selection or a cap.

## Direct-API spending guardrails

A user may leave spending limits disabled or configure independent estimated-cost caps for the active direct credential for the current calendar week, calendar month, calendar year, and/or all time. Multiple caps may coexist; reaching any enabled cap blocks future AI dispatches for that credential until its period resets or the user explicitly changes the policy. A newly enabled cap begins with an explicit zero baseline at activation and does not claim to include earlier provider or ORT use; the activation time is always displayed. Week boundaries, time zone, currency, reset date, included credential identity, and counted attempt statuses are shown before saving.

Before dispatch, the accounting boundary atomically reserves a conservative maximum for the attempt using counted spend, unresolved reservations, the provider's preflight input count or a safe local estimate, the configured maximum output, and all applicable catalog price dimensions. Dispatch occurs only if the reservation fits every enabled cap. On completion, the reservation settles to the best supported provider-reported estimate. Ambiguous or missing usage remains conservatively reserved and visibly unresolved; it is never assumed to cost zero. When an applicable price is missing, expired, unverified, or cannot be converted under the configured policy, a hard currency cap fails closed and explains how to refresh the signed catalog, update the application, change the model, lower the output bound, disable the cap, or verify the provider bill.

Guardrail counters are durable accounting state separate from the user-cleared activity table. Clearing or age-expiring AI Activity does not reset a cap or its period total. Disabling a cap, changing its amount, or deliberately resetting an all-time baseline requires a separate confirmation that shows the effect. ORT notifications default to local warnings at 50% and 80% and a blocking notice at 100%; warning thresholds may be adjusted without weakening the hard cap.

These limits govern only requests routed through this ORT installation and credential identity. They cannot see other applications using the same key, delayed provider adjustments, taxes, credits, negotiated rates, exchange rates, or a compromised key used elsewhere. The provider's own billing controls remain the authoritative protection and should be linked prominently.

## Codex usage visibility and guardrails

Codex subscription mode stores no estimated dollar spend. AI Activity may show:

- ORT operation/thread token totals from `thread/tokenUsage/updated` when the compatible app-server reports them.
- Account-level lifetime and daily token activity returned by `account/usage/read`, clearly separated from ORT-only operations because it may include other Codex clients.
- Every provider-reported quota bucket from `account/rateLimits/read`, including its name or identifier, used percentage, exact window duration, and reset time.

The interface names a window **weekly** or **monthly** only when the provider-reported duration actually matches that description; otherwise it shows the exact duration, such as 5 hours or 7 days. A before/after quota-percentage delta may be shown as an **observed account change during this operation**, never as exact per-call consumption, because provider rounding, delayed updates, resets, and concurrent Codex activity can affect it. Missing token or quota fields remain **unavailable**.

Users may set an optional maximum provider-reported `usedPercent` for each currently returned Codex quota bucket, or one threshold applied to every bucket. ORT refreshes quotas before dispatch and blocks a new Codex operation when any applicable threshold is already reached. Because provider quota updates can be delayed or rounded, the operation that crosses a threshold may finish; the cap is a best-effort stop for subsequent ORT work, not a guaranteed reservation or provider-side limit. If quota state cannot be refreshed, a configured Codex cap fails closed. A reset, renamed, added, or removed provider bucket is reconciled by stable bucket identifier and requires review when the old policy no longer maps safely.

Codex guardrails cover the connected ChatGPT account's reported quota, including activity outside ORT. They do not change the user's ChatGPT plan, provider limits, billing, or other Codex clients.

## Supported AI operations

- Convert extracted resume text into a structured import proposal.
- Tailor the published master to a reviewed job description and return bounded Required Qualification Alert candidates within the same logical operation and provider request.
- Refine a current tailored resume using a specific user instruction.
- Generate or revise a cover letter.
- Draft permitted answers to deliberately captured application questions.
- Perform a self-review for unsupported claims, omissions, malformed structure, and requested length/page intent.

Every operation is initiated from the desktop application after review. The browser extension cannot call an AI provider directly.

## Request safety and minimization

- Send only content necessary for the chosen operation.
- Treat imported documents, captured job text, questions, URLs, filenames, and user instructions as untrusted quoted data.
- Delimit untrusted material and instruct the model not to follow embedded instructions.
- Require a versioned structured-output schema when the provider supports it; otherwise validate and normalize the result before use.
- Enforce size, timeout, concurrency, and output bounds locally.
- Never execute code, tools, URLs, or commands suggested by captured content or model output.
- Do not transmit unrelated tracker history or retained documents.

## Factual boundaries and result validation

AI output may reorganize, select, condense, and rewrite confirmed facts. It may not invent or infer employers, dates, degrees, credentials, projects, skills, metrics, achievements, authorization, identity, or intentions.

Before display, ORT validates:

- Schema and allowed fields
- Size and collection bounds
- Link formats
- Relationship to the published master
- Required-qualification classification, job-text evidence, published-resume evidence, and allowed alert category
- Prohibited attestations
- Requested word/character limits where applicable
- Structural integrity required by the renderer

Tailored resumes include approximately three change-summary bullets. ORT verifies these using a deterministic structural comparison rather than trusting only the model's self-description.

Required Qualification Alert candidates use a separate versioned portion of the tailoring response schema. Before display, ORT verifies that each candidate points to an explicit mandatory requirement in the reviewed job text, maps to a supported resume-content category, and is either an explicit factual conflict or information not found in the published master. Preferred, ambiguous, non-resume, personal, protected, and legal-attestation requirements are dropped. A confirmed mismatch must cite resolvable conflicting resume evidence; absence alone can produce only **Not found in your published resume**. The model does not decide eligibility, calculate a fit score, or make application recommendations.

The qualification comparison is part of the existing tailoring request because the provider already receives the reviewed job description and published master. ORT does not issue a second provider request or resend those inputs solely to generate alerts. The compact alert schema and collection bounds count within the existing tailoring output limit and direct-API guardrail reservation.

If a result cannot be validated, ORT preserves the previous local state, explains the failure, and allows retry or manual editing. A partially streamed or malformed response is never silently promoted to a selected final artifact.

## Resume import

### Initial supported input

- Text-bearing PDF and DOCX documents.
- Build-from-scratch remains available without AI.
- Scanned/image-only resumes and OCR are deferred.

### Import flow

1. The user selects a local document.
2. ORT validates type and protective size/page limits.
3. Local libraries extract text, links, headings, bullets, dates, and available layout hints without uploading the original binary.
4. ORT shows the extracted content and identifies the selected AI connection mode, provider, and model before transmission.
5. With confirmation, the selected direct provider or Codex connection maps the extracted content into ORT's structured schema.
6. ORT displays the source alongside the proposal and highlights uncertain or unusual mappings.
7. The user can edit, move, relabel, accept, or reject every proposed item.
8. Accepted information enters the master-resume draft only after confirmation; publishing remains a separate action.
9. Original files and temporary extracted content are removed from ORT working storage when no longer needed.

The original binary should not be sent when local extraction provides adequate content. If a future provider feature requires uploading the original file, ORT must disclose that separately and obtain explicit confirmation.

### Import safeguards

- Import never silently replaces the master draft.
- Potential duplicates are presented for merge, keep-both, or discard decisions.
- Low-confidence content remains visibly unconfirmed.
- When AI structuring fails, extracted text remains available during the current import review so the user can manually assign it where feasible.

## Reliability and cancellation

- AI operations are asynchronous from the interface perspective: the UI remains responsive, shows progress, and supports safe cancellation where the provider permits it.
- Each local request has an identifier so a retry or reopening the overlay does not duplicate accepted results accidentally.
- Each network attempt is written to the local AI activity ledger before dispatch and finalized recoverably after success, failure, cancellation, or timeout.
- Provider errors, rate limits, authentication failures, safety refusals, timeouts, and malformed output receive distinct actionable messages.
- Closing the app may cancel the remote request or allow a bounded local operation record to recover its result, depending on provider support. Exact behavior must be documented per adapter.
- ORT never promises that cancellation prevents provider billing or subscription-quota consumption after a request has begun.

## Local models

Local-model support is deferred. A future adapter must disclose download size, hardware requirements, performance, model license, provenance, update behavior, and quality limitations. Local inference should use the same structured schemas and validation gates as remote providers.
