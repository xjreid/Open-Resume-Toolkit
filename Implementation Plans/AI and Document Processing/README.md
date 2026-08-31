# AI and document-processing implementation plans — reserved

Future plans in this folder should cover:

- Direct-provider adapter interface for OpenAI, Anthropic, and Gemini; dated Economy/Balanced/Quality catalogs; vault identities; model discovery; lifecycle/deprecation handling; and requested-versus-effective model capture
- External Codex-runtime discovery and verification; supported runtime/protocol window; generated schemas and capability negotiation; app-managed `stdio` child-process lifecycle; isolated ORT configuration/authentication/keyring; managed ChatGPT/device-code authentication; tested model intersection; no-tool ephemeral execution profile; event handling, cancellation, orphan recovery, cleanup, and safe disablement
- Prompt, schema, preset, and evaluation versioning
- Request minimization, streaming, timeouts, cancellation, retry, idempotency, and recovery
- Input/output validation, factual comparison, prompt-injection defenses, prohibited-answer checks, and change summaries
- Standardized provider-call accounting, usage-category normalization, versioned conditional pricing catalog, cost estimates, and provider-specific privacy/billing disclosures
- Per-model/provider logical-operation and attempt aggregation, missing/partial usage behavior, cross-currency rules, and interrupted-call reconciliation
- Direct-API cap reservations and settlement; weekly/monthly/yearly/all-time counters; notifications; unknown-outcome reconciliation; and fail-closed enforcement
- Codex thread-token and account-usage provenance, rate-limit bucket normalization, quota-threshold enforcement, delayed updates, and bucket migration
- Local PDF/DOCX text extraction and scanned-document detection
- Structured resume/cover-letter/question-answer schemas
- Preview, PDF, DOCX, and plain-text rendering contracts
- Font/template packaging, layout validation, accessibility, and historical renderer compatibility
- Synthetic evaluation corpus and release gates per provider preset
