# M3 completion audit

Audit date: September 16, 2026. This audit distinguishes implementation,
automated evidence, native observation, and the roadmap's no-credential exception.

| Requirement | Current evidence | Assessment |
| --- | --- | --- |
| No AI / Direct API state | `ai_settings.rs` connection commands and AI workspace tests cover save, pause, resume, replacement cleanup, and removal state | Implemented and automated |
| Vault-backed credentials | Provider secrets are opaque/non-serializable, stored under random credential identities, and excluded from settings/backups; opt-in synthetic macOS provider-vault round trip passed with OS access; signed app is storage-ready | Implemented and verified |
| OpenAI, Anthropic, and Gemini adapters | Request builders, pinned provider URLs, bounded stream parsers, model validation, and provider fixtures pass in `ort-ai` and desktop tests | Implemented and automated |
| Signed model/preset/pricing catalog | Exact-byte Ed25519 verification, expiry/rollback/compatibility/disable and price-shape checks, official-source review of catalog `2026-09-15.1`, backend selection enforcement, and user-visible signed-entry details | Implemented and automated |
| Accounting lifecycle | Encrypted operation/attempt reservation precedes dispatch; dispatching/streaming/terminal transitions, cancellation, one classified retry, startup recovery, and unknown outcomes are covered by Rust tests | Implemented and automated |
| Streaming and cancellation | Bounded SSE reads deliver content only to the initiating main window; component tests cover progress, cancellation, failure, and missing usage | Implemented and automated; no live provider stream |
| Usage and cost normalization | Provider fixtures cover input/output/cache/reasoning categories; integer-micro rounding, missing prices, OpenAI reasoning breakdown, and cost-above-reservation behavior are tested | Implemented and automated |
| Historical pricing provenance | Schema 4 attempts snapshot catalog ID/effective date, preset version, exact bounded pricing components, currency, reservation, and settled estimate; format 1.4 preserves those fields across restore | Implemented and automated |
| Transactional caps | Week/month/year/all-time policies, IANA-zone rollover, atomic reservations, concurrency rejection, unresolved exposure, separate disable/reset, new all-time baseline, and 50/80/100% notices are tested | Implemented and automated |
| AI Monitoring | Selected-period encrypted aggregates, daily/monthly buckets, totals, graphs, breakdowns, partial/unknown labeling, native no-clobber JSON export, retention, and separate clearing/reset behavior are covered | Implemented and automated |
| Backup, restore, and deletion | Portable format 1.4 carries bounded content-free AI activity and pricing provenance but not credentials/cap authority; transactional restore and all-data credential deletion are tested | Implemented and automated |
| Provider transparency | Confirmation names provider/model, fixed transmitted content, preflight input estimate, conservative reservation, provider terms/privacy/retention/rates/charges/eligibility, and ORT account/refund/availability limitations; returned usage is displayed | Implemented and component-tested |
| Original desktop UI and functions remain | Resume/import/settings routes remain, AI is additive, production frontend builds, 119 frontend tests pass, Rust DOCX/PDF/import/storage regressions pass, and the signed pinned-helper candidate preserves editor/view/export/import/settings controls | Automated and native evidence |
| Live synthetic request | Roadmap allows this to be documented unavailable when no provider credential is configured | Not run: no user credential configured |
| Storage-ready native M3 walkthrough | September 16 signed current-source pinned-helper candidate reports encrypted storage ready; AI/catalog/period/export-cancel and original route/import-cancel checks are recorded in `native-walkthrough.md` | Verified within development scope |

## Verified commands

- `pnpm check` — complete formatting/lint/test/build/security/license gate
- `cargo test --workspace --all-targets --locked --offline`
- `cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings`
- desktop `vitest run` — 119 tests
- desktop `tsc --noEmit` and direct `vite build`
- contracts `vitest run` — 29 tests, plus `tsc --noEmit`
- extension `tsc --noEmit`
- repository tool tests — 30 tests
- secret and web-security checks
- dependency-license inventory — 778 Rust and 167 JavaScript packages
- opt-in synthetic database-key and provider-credential macOS Keychain tests
- final bundled `parser_helper_smoke` — PDF/DOCX, identity denial, cancellation/reaping
- locked contract regeneration — exact generated-file hashes unchanged

M3 exit evidence is satisfied for macOS Apple Silicon development. A live
provider call was not run under the roadmap's explicit no-credential exception;
it must not be represented as run. Notarization, production catalog key custody,
Windows/Intel native qualification, and later AI product workflows remain outside M3.
