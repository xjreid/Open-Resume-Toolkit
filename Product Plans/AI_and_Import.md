# AI, credentials, and resume import

## AI operating model

ORT operates no shared AI account and pays for no user inference. AI is optional and uses one of these modes:

1. A remote provider and API key explicitly configured by the user.
2. A later local-model adapter installed and selected by the user.
3. No AI, preserving manual editing, tracking, rendering, import review where possible, and export.

Remote provider calls go directly from the desktop application to the provider over authenticated HTTPS. ORT operates no AI gateway, relay, logging proxy, or fallback credential service.

Provider adapters and exact supported models remain implementation decisions. Initial presets should include at least one cost-conscious configuration and one high-quality configuration when supported by the user's chosen provider. Every preset must use a publicly available API model and pass cost, structure, privacy, and quality testing before release. No provider or model is a centrally funded entitlement or permanent hard-coded dependency.

## Credentials

- The desktop app receives provider credentials directly from the user.
- Credentials are stored in Windows Credential Manager, macOS Keychain, or an equivalently protected OS facility.
- Credentials never appear in the local content database, logs, backups by default, browser-extension storage, native-messaging payloads, exports, crash reports, or diagnostic bundles.
- The user can test, replace, and remove a credential.
- A failed credential never causes silent fallback to another provider or key.
- Provider-specific usage and billing links should be available so the user understands that charges come from that provider.

## Provider transparency

Before every new class of transmission—and again whenever the selected provider changes—ORT explains:

- Which provider and model will receive the request.
- Which resume, job, question, or instruction content is included.
- That the provider's terms, retention, privacy, rate limits, and charges apply.
- That ORT cannot recover provider accounts, refund provider charges, or guarantee provider availability.

The application should estimate input size and surface provider-reported token usage/cost metadata when reliably available. Estimates are informational and must not be represented as the provider's final bill.

## Supported AI operations

- Convert extracted resume text into a structured import proposal.
- Tailor the published master to a reviewed job description.
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
- Prohibited attestations
- Requested word/character limits where applicable
- Structural integrity required by the renderer

Tailored resumes include approximately three change-summary bullets. ORT verifies these using a deterministic structural comparison rather than trusting only the model's self-description.

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
4. ORT shows the extracted content and identifies the selected AI provider before transmission.
5. With confirmation, the provider maps the extracted content into ORT's structured schema.
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
- Provider errors, rate limits, authentication failures, safety refusals, timeouts, and malformed output receive distinct actionable messages.
- Closing the app may cancel the remote request or allow a bounded local operation record to recover its result, depending on provider support. Exact behavior must be documented per adapter.
- ORT never promises that cancellation prevents provider billing after a request has begun.

## Local models

Local-model support is deferred. A future adapter must disclose download size, hardware requirements, performance, model license, provenance, update behavior, and quality limitations. Local inference should use the same structured schemas and validation gates as remote providers.
