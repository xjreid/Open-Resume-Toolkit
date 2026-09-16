# M3 implementation record

M3 began on September 13, 2026 after explicit M2.5 closure.

## Implemented foundation

- No AI / Direct API connection state and a desktop AI & Monitoring workspace,
  including explicit pause/resume and confirmed vault removal without clearing
  content-free activity.
- OS-vault-only direct-provider credentials using opaque random credential IDs;
  secret types are non-serializable, redacted in debug output, and cleared on drop.
- Provider request and bounded SSE normalization for OpenAI Responses, Anthropic
  Messages, and Gemini streamGenerateContent. Hosts are pinned by adapter and no
  silent provider/model fallback exists. Fixtures now cover Gemini text plus
  usage in one event and Anthropic input/output usage split across events.
- Exact-byte Ed25519 verification for a versioned catalog with a trust key
  independent from updater trust, plus chronology, compatibility, expiry,
  rollback, operation eligibility, and emergency-disable checks. The desktop
  connection view now shows the selected signed entry's model, ORT limits,
  price dimensions (unavailable is not zero), source, and effective dates.
- Provider-neutral operation/attempt state, one-active-operation enforcement,
  cancellation, one classified 5xx retry, interrupted-attempt recovery, usage
  normalization, integer-micro cost arithmetic, atomic cap reservation, unknown
  outcome preservation, aggregate monitoring, JSON export, and history clearing
  that does not reduce guardrail counters.
- Forward-only encrypted profile schema v4 with dedicated content-free
  `ai_operations`, `ai_attempts`, and `ai_guardrail_policies` tables, restrictive
  status/provider/category checks, one-active-operation uniqueness, monitoring
  indexes, migration receipts, immutable pricing provenance, and additive
  v1→v2→v3→v4 migration coverage.
- Transactional encrypted attempt reservation, pre-dispatch cancellation,
  dispatched-attempt settlement, and startup recovery; ordinary activity
  clearing preserves cap counters and cap reset is a separate explicit store
  operation. Selected-period monitoring reads the encrypted records and its
  aggregate JSON export matches the displayed result. Mixed currencies are
  separated instead of summed as one amount.
- The desktop synthetic-test path uses the OS vault at dispatch, HTTPS-only
  rustls transport, exact provider URL pinning, no redirects, bounded streaming
  and main-window-only progress. An estimate review/confirmation precedes the
  request; reservation, dispatch, streaming, and settlement are persisted. A
  second attempt is automatically made only for a classified first HTTP 5xx,
  with a `retry_of` relationship and a fresh cap check. Ambiguous transport
  errors remain unresolved and are not automatically resent.
- Before confirmation, the connection test now states the exact fixed content
  class, selected provider/model, conservative input estimate and reservation,
  plus provider terms/privacy/retention/rate/charge/eligibility applicability
  and ORT's account-recovery/refund/availability limitations. Reliable returned
  input/cache/output/reasoning usage is shown after success.
- Four spending-cap periods (week, month, year, all time) have transactional
  reservations, recorded IANA time zones, DST-aware period rollover, explicit
  reset/disable controls, and 50/80/100% local warnings. An all-time reset now
  records a new zero activation baseline and reloads its displayed state rather
  than showing an old activation date. A reported cost above
  the reservation stays unresolved rather than silently overshooting a cap.
  Replacing a key creates a new identity and zero baseline; copying old limits
  is an explicit opt-in, never a counter transfer.
- Desktop Monitoring loads persisted selected-period aggregates, day/month
  time buckets, token/cost bars, provider/model/preset/operation/status
  breakdowns, content-free native-dialog no-clobber JSON export, confirmed
  clearing, and configurable 30/90-day, one-year, or retain-until-cleared
  history. Clearing history does not reset cap counters. The export never
  accepts a renderer-provided path. The original editor, publication, and
  document-export routes are preserved.
- Portable backup format 1.4 carries content-free AI operation/attempt records
  and inventory counts and restores them transactionally into a fresh keyed
  profile. It excludes OS-vault credentials and connection references; cap
  policies/counters are intentionally not transferred, so restore never
  silently inherits spending authority. All-data deletion also removes the
  active provider vault item and is guarded against an active AI request.

Focused Rust tests cover all three provider fixtures, credential/request
redaction, signed-catalog rollback denial, bundled signature verification, cost
rounding and unavailable dimensions, cap concurrency semantics, recovery,
cancellation, aggregate export, and separate history/guardrail behavior.

## Verification and closure evidence

The [requirement-by-requirement completion audit](completion-audit.md) records
which claims have direct evidence and which remain unavailable.
The [catalog review](catalog-review.md) records the official-source model and
price verification for the signed `2026-09-15.1` development baseline.

M3 is **complete for macOS Apple Silicon development**. On
September 15 the offline Rust workspace suite passed (including 10 AI, 36
desktop, and 46 storage tests; five explicit native vault opt-ins ignored), the
desktop frontend suite passed 119/119 including eleven AI workspace tests and an
axe accessibility check, TypeScript and direct Vite production build passed,
and the web-security gate passed. Strict workspace Clippy with warnings denied
also passed after the credential and cost-bound edits. The opt-in synthetic
macOS database-key and provider-credential Keychain round trips passed when
run with OS access; the database-key test failed `Unavailable` under the
workspace sandbox, which cannot access that Keychain context. The repository's
dependency-license inventory passed (778 Rust and 167 JavaScript packages)
after a narrow, expiring exception for `webpki-root-certs@1.0.9` certificate
data. Its CDLA-Permissive-2.0 agreement text must accompany any distributed
data; release notices are still an M7 obligation, not proven by this gate.

No live synthetic provider request was made: this workspace has no configured
user provider credential. The roadmap permits documenting missing live-provider
evidence rather than a lengthy account-setup exercise. The September 15 unsigned
development launch failed closed at `Storage unavailable`. On September 16,
the locally signed current-source candidate with its pinned parser helper was
storage-ready and completed the [native walkthrough](native-walkthrough.md).
The final candidate also contains the corrected M3 backup disclosures; the
desktop suite passed 119/119 again. The installed app was not replaced.
