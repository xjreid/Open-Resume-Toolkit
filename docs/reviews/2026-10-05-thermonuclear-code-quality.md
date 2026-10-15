# Thermo-nuclear code quality review — 2026-10-05

**Verdict: changes required.** This codebase does not meet the invoked skill's approval bar. The central problem is split ownership of domain semantics, workflow state, and contracts. It has already produced reproducible data loss, stale export content across profiles, and an unreachable backup boundary.

There are **nine findings: three P1 correctness/lifecycle defects and six P2 structural or accounting issues**. P1 means address before relying on the affected workflow; P2 means a substantive maintainability or correctness issue that should be scheduled. No P0 issue was established.

This is a historical audit of the source revision below. Source links are pinned
to that revision; later fixes are described in the remediation records and
[current implementation map](../development/current-state.md).

## Scope and method

- Reviewed the current working tree on `main`, HEAD `3450c80a7900e3a2759ce6fe074d64125a91c892`. HEAD matched the local `origin/main` reference. This is a repository baseline audit, not attribution of every finding to the latest commit or a claim about a new PR crossing a size threshold.
- Applied the locally invoked Thermo-Nuclear Code Quality Review skill: prioritize deletion of incidental complexity, canonical ownership, atomic lifecycle changes, clean contracts, and substantive decomposition.
- Covered the desktop React/Tauri application, 17 Rust workspace packages, browser extension/native messaging, generated contracts, storage/backup, AI accounting and adapters, import/worker supervision, document rendering/export, and relevant tooling/CI. The review combined a source inventory, boundary/dependency tracing, examination of critical workflows, existing checks, and focused synthetic reproductions. It is not a claim that every source line received equal manual scrutiny.
- The inventory contains 155 implementation source files and 70,961 lines, including inline Rust tests. Dedicated test files and generated files are excluded from that count. Fifteen implementation files exceed 1,000 lines. Counts are an ownership diagnostic, not a substitute for the findings below.
- Existing user edits were preserved. Temporary reproduction registrations were removed after execution. Application source was left unchanged; this report is the persistent review artifact. Local reproduction sources and logs remain in the ignored local evidence directory `target/code-review-2026-10-05` (not published with this report).

## Findings

### F1 — P1: Restore a lossless, canonical model for resume fields

**Primary location:** [App.tsx:2179](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/App.tsx#L2179).

`ResumeEntry.fields` permits multiple named fields, but the canvas projects the collection into one `details` field, one field identified by the display label `Extra`, and a paragraph identified by a magic label. When the user edits a missing Extra slot, `updateVisibleField` reconstructs the entire collection from those slots. Additional ordinary fields disappear from the updated document and can be autosaved away.

The same unsupported assumption appears in the reading/export paths: [PublishedResume slices to one field](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/App.tsx#L3076), [PDF rendering selects the first ordinary field](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-render/src/lib.rs#L384), and [DOCX rendering does the same](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-documents/src/docx.rs#L336). Plain-text export retains the additional field, so the formats disagree about the document's content.

**Evidence:** A focused React reproduction used valid `Language=Rust` and `Certification=Preserve this certification` fields with no Extra field. Editing Extra removed Certification; PublishedResume omitted it even before that edit. A separate Rust reproduction validated a document with two ordinary fields and confirmed that plain text included both while all four DOCX styles omitted the second. These are observed failures, not hypothetical schema drift. PDF omission was established by tracing the renderer; the custom two-field PDF case was not separately parsed.

**Required restructuring:** Preserve the full collection on every edit. Define field roles/body representation in the canonical document model, with an explicit migration for existing labels, and use a lossless presentation projection across viewers and exporters. If a layout supports fewer fields, that limitation must be explicit rather than silently discarding accepted data. Add preservation checks over arbitrary valid fields. The existing independent fixture oracles also reproduce the first-field assumption ([DOCX oracle](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/tools/verify-docx-fixtures.py#L63), [PDF oracle](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/tools/verify-pdf-fixtures.mjs#L120)); passing those checks does not establish losslessness.

### F2 — P1: Make profile replacement invalidate the entire application lifetime

**Primary location:** [data_deletion.rs:42](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/data_deletion.rs#L42).

Deleting local data replaces the encrypted store, clears `PdfState`, and clears drag files, but leaves `ApplicationExportState` alive. That cache identifies artifacts using only `(revision, kind)` ([cache identity](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/application_materials.rs#L142)). A new profile reuses revision numbers. The cache-hit checks verify only that a workspace with the requested revision exists in the new store ([prepare path](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/application_materials.rs#L1473)); they cannot distinguish old-profile bytes from new-profile bytes. A cached higher revision also rejects replacement by a lower fresh revision.

Reset is fragmented in the frontend as well. The main window remounts the tracker with `profileGeneration`, but does not similarly remount AI state ([App.tsx](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/App.tsx#L967)). The overlay loads its workspace once, then polls context/capture status without reloading workspace state ([ApplicationOverlay.tsx](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/ApplicationOverlay.tsx#L389)). Its old saved/draft state can survive deletion.

**Evidence:** A synthetic native test prepared an old profile's PDF, replaced `DesktopState` with a separately initialized profile, saved different content at the same revision, and confirmed that the existing cache accepted the new revision and returned the old PDF bytes. A fresh render differed. This exercises the actual store and renderer/cache; a live native deletion/UI session was not used.

**Required restructuring:** Give profile-scoped state one owner and an explicit profile identity/generation. Cache keys must include that identity and a stable source identity. Profile replacement must retire dependent caches and pending work, then notify every webview to discard profile-scoped state and reload. Adding one cache clear fixes the immediate defect but leaves the same omission pattern available to the next feature.

### F3 — P1: Align publication retention with the backup format's accepted states

**Primary location:** [ort-backup/src/lib.rs:656](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-backup/src/lib.rs#L656).

The backup validator rejects more than 100 published resumes. [Publishing](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-storage/src/lib.rs#L926) adds an immutable snapshot for every distinct published draft without enforcing that bound or pruning history. [Backup collection](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-storage/src/lib.rs#L1887) loads every publication and sends the collection to the rejecting validator. Ordinary successful use can therefore create a valid local profile that cannot be backed up.

**Evidence:** A synthetic encrypted profile successfully published 101 distinct drafts. `load_latest_published` returned revision 101, while `create_portable_backup` returned `StorageError::InvalidData`.

**Required restructuring:** Define one retention/export policy shared by storage and backup. Every state the writer accepts must remain representable in a supported backup. If retention is bounded, manage it deliberately and preserve snapshots referenced by render receipts or other records. Simply taking the newest 100 during export silently loses history and may break references; simply increasing one constant postpones the mismatch.

### F4 — P2: Consolidate AI execution and settle billing independently of output acceptance

**Primary location:** [ai_request.rs:1600](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/ai_request.rs#L1600).

Credential testing and material generation independently implement request reservation, dispatch, streaming, cancellation, parsing, and settlement. They already disagree on accounting: the credential path preserves serving model and usage when output is unusable and prices it when trustworthy ([lines 932–955](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/ai_request.rs#L932)). The material path calls a closure that always supplies `None` for model, usage, and settled cost ([lines 1495–1507](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/ai_request.rs#L1495)), even after `SyntheticStreamState` has parsed that evidence.

A Gemini `MAX_TOKENS` or safety finish can carry model/usage alongside `ProviderFailure`; the existing [adapter test](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-ai/src/tests.rs#L719) establishes that event shape. The material path discards those known tokens and turns the whole preflight reservation into unresolved cap consumption ([durable settlement](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-storage/src/ai_activity.rs#L1105)). This can leave monitoring incomplete and reject later requests sooner than the reported usage warrants. No paid/network provider request was made during the audit; this finding follows the parser output and actual settlement code.

There is a further opportunity to delete complexity: the approximately 390-line in-memory accounting model in [ort-ai/src/lib.rs:707](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-ai/src/lib.rs#L707) has no production consumer. Repository references to `AccountingLedger` are confined to its definition and three unit tests. The desktop uses the durable SQL implementation in `ort-storage::ai_activity`, which has its own tests. Keeping two stateful accounting implementations gives policy two places to drift, and those three in-memory tests do not verify the production implementation.

**Required restructuring:** Use one execution pipeline with explicit operation policy and a typed attempt outcome carrying content status and billing evidence separately. Settle trustworthy usage regardless of whether content is accepted, while preserving unresolved treatment for ambiguous evidence. Remove the unused ledger or reduce it to pure policy genuinely shared by the durable implementation; retain invariant tests against the path the product actually runs.

### F5 — P2: Move application workflows into the application layer

**Primary location:** [application_materials.rs:338](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/application_materials.rs#L338).

This 3,177-line module contains roughly 1,888 lines before its test module: persistent workflow models, prompt policy, capture transitions, validation, AI orchestration, PDF/DOCX preparation, caches, native dialogs, and drag-file lifetime. Its application model lives in the Tauri adapter. The intended [ort-application boundary](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-application/src/lib.rs#L1) currently exports only import use cases.

The ownership leak is concrete: [tracker preview constructs a fake ApplicationWorkspace](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/tracker.rs#L339), including invented published revision, job description, and empty workflow fields, merely to call `document_for`. Storage also branches on feature-specific setting keys and size limits ([storage policy](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-storage/src/lib.rs#L60)). These dependencies make unrelated feature changes require understanding the full overlay workflow.

**Required restructuring:** Put canonical models in the domain/contract boundary and workflow transitions in `ort-application`. Give artifact rendering a small typed source input that tracker and overlay can both provide without fabricating workflow state. Keep Tauri commands responsible for authorization, platform handles, and invoking services. Replace opaque feature blobs with deliberate typed repositories as the workflow moves. Splitting the current file into similarly coupled files would not meet the skill's simplification bar.

### F6 — P2: Make IPC contracts genuinely single-source and validated

**Primary location:** [ApplicationOverlay.tsx:138](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/ApplicationOverlay.tsx#L138).

Application, AI, and tracker UI code declare their own Rust-shaped payloads and call `invoke<Response<T>>`. That generic argument is a TypeScript assertion, not a decoder. Command names and payload/value combinations are accepted independently; malformed or changed native responses reach state as supposedly valid objects. The existing [command-client](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/command-client.ts#L120) shows the safer established pattern: receive `unknown`, validate, and return a typed failure at the boundary.

The generator also has two sources of truth. JSON schemas are derived from Rust, but TypeScript interfaces and runtime guards are copied from handwritten templates ([main.rs:16](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-contract-generator/src/main.rs#L16), [copying output](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-contract-generator/src/main.rs#L65)). [CI's generated diff check](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/.github/workflows/ci.yml#L60) verifies regeneration, not equivalence between the Rust schemas and those templates. AI/application/tracker have no corresponding exported contract modules. This is a demonstrated boundary/design deficiency; the review does not claim an observed incompatible native payload or an external injection exploit.

**Required restructuring:** Derive serialized types and validation from a canonical schema/model, add these command families to the shared contracts package, and use command-specific typed clients with runtime decoding. While handwritten semantic validators remain, test Rust-produced fixtures and rejection cases against the TypeScript boundary. Eliminate per-component response wrappers and duplicate domain interfaces.

### F7 — P2: Reverse the frontend ownership dependency and centralize save coordination

**Primary location:** [ImportedResumeEditor.tsx:4](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/ImportedResumeEditor.tsx#L4).

`App.tsx` is 3,270 lines and owns shell/navigation, storage lifecycle, load/save/publish/export orchestration, multiple editors, the reusable canvas, and the reading view. Reusable consumers import from the root application. There is an actual runtime cycle: `App → DocumentImport → ImportReviewFlow → ImportedResumeEditor → App`. [ApplicationPopup](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/ApplicationPopup.tsx#L6) also depends on the root to get its leaf views. This makes isolated editor reuse pull in native orchestration and unrelated feature modules.

The 1,861-line overlay independently carries saved/draft refs, pending writes, revisions, error status, and in-flight promises. Its [stage-one flush](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/ApplicationOverlay.tsx#L455) and [workspace flush](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/ApplicationOverlay.tsx#L601) implement closely related save coordinators. Every lifetime change must now invalidate several independently maintained queues and flags.

**Required restructuring:** Extract editor/reading leaves below the shell and put resume-session orchestration in a dedicated controller/hook using the existing editor reducer. Share a narrow revision-checked, coalescing save coordinator between overlay stages, with explicit pending/saving/error states and profile lifetime. Keep feature-specific policies outside that coordinator. The unreferenced exported `ResumeFields` in [ApplicationViews.tsx:75](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/ApplicationViews.tsx#L75) is another deletion opportunity once consumers are verified; do not preserve another full editor implementation without a caller.

### F8 — P2: Use indexed tracker access instead of scanning retained content for point operations

**Primary location:** [tracker.rs:233](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/tracker.rs#L233).

Updating a single tracker record calls `tracker_list`, decodes every entry, then searches for one ID. [PDF preview repeats this pattern](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src-tauri/src/tracker.rs#L318). [The storage query](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-storage/src/tracker.rs#L46) selects every `entry_json` without a bound. Those records contain retained resumes, cover letters, and answers and can individually occupy up to 1 MiB. The schema already has an entry primary key.

Point operations therefore cost O(total retained content), including allocation/deserialization, while holding the desktop store boundary. Full list responses also transfer retained content to render a metadata table. The asymptotic behavior is established by the code; no large-profile latency benchmark was performed, so an exact delay is not claimed.

**Required restructuring:** Add a profile-scoped indexed `tracker_get(id)` and a summary/list API with pagination and metadata filtering. Load retained documents only for detail/preview. Prefer metadata-specific native updates that preserve immutable retained content internally rather than making the renderer resend the entire record. This removes the full-list abstraction from operations that need one row.

### F9 — P2: Replace accumulated stylesheet overrides with clear style ownership

**Primary location:** [main.tsx:4](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/main.tsx#L4).

Both desktop entry points load a 6,059-line base stylesheet followed by a 2,141-line theme: 8,200 lines before other feature styles. A comment-stripped, whitespace-normalized flat-rule scan found 127 exact selector-group matches out of 333 theme groups. This measures repeated ownership, not identical declarations or a claim that every overlap is removable.

For a concrete example, [app.css sets title-line layout and field flex proportions](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/app.css#L4476), then [workspace-theme.css replaces those choices](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/apps/desktop/src/shared/workspace-theme.css#L885). The current component layout depends on the accumulation and order of historical rules. The built CSS asset is 139.62 kB uncompressed; the principal concern is maintainability of the cascade, not a measured performance defect.

**Required restructuring:** Move current rules to the owning shell/editor/tracker components, keep one token layer and deliberate document-style variants, and retire superseded base declarations as their owners are migrated. Preserve appearance with existing geometry evidence plus responsive/state visual checks. Adding a third override sheet would increase the problem; moving all 8,200 lines into arbitrary smaller files would merely redistribute it.

## Verification and limits

| Check | Result |
| --- | --- |
| Rust workspace tests, locked/offline, serial | 407 passed; 11 intentionally ignored |
| Rust workspace tests, initial default parallel run | Failed one macOS pipe-lifecycle test; see below |
| Focused pipe test, serial platform suite, later default parallel platform suite | Passed |
| Clippy, workspace/all targets/all features, `-D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed after temporary reproduction registrations were removed |
| Desktop TypeScript + Vitest | Passed; 40 test files, 238 tests |
| Contracts TypeScript + Vitest | Passed; 8 test files, 30 tests |
| Extension TypeScript/build + Chrome tests | Passed; 22 tests |
| Tooling Node tests | Passed; 32 tests |
| Desktop Vite production build | Passed |
| Prettier on reviewed frontend/contracts/capability inputs | Passed |
| Contract generation + generated-file diff | Passed; no generated changes |
| Web security and secrets checks | Passed |
| Dependency license check | Passed; 793 Rust and 167 JavaScript packages |
| Independent plain DOCX/PDF fixtures | Passed, including expected text/order, structure and geometry checks |
| Independent style output checks | 48 PDF/DOCX/text pairs passed local regression checks across technical/professional/modern and schema v1/v2 |
| Focused review reproductions | Five tests confirmed the defective behavior described in F1–F3 |

The initial parallel Rust failure was `worker_output_macos::tests::invalid_input_releases_both_owned_readers_and_drop_preserves_other_handles`, at [worker_output_macos.rs:358](https://github.com/xjreid/Open-Resume-Toolkit/blob/3450c80a7900e3a2759ce6fe074d64125a91c892/crates/ort-platform/src/worker_output_macos.rs#L358): a write expected to fail after dropping the owned readers unexpectedly succeeded. The isolated test, serial workspace suite, and later parallel platform run passed. Treat this as an unresolved intermittent test failure; its root cause was not proven and it is not being mislabeled as a confirmed production descriptor leak. Preserve and investigate the failure under the normal test gate rather than permanently hiding it through serial execution.

The top-level `pnpm lint && pnpm test` wrapper could not run because pnpm attempted a dependency-state repair requiring unavailable registry access and an interactive modules-directory removal. Installed TypeScript/Vitest/Node binaries were used directly instead; no dependency reinstall was performed. The table distinguishes these checks from a clean execution of the entire top-level `pnpm check` command.

Native signing/install, live OS vault integration, live webview behavior, parser guest/native qualification, other operating systems, and real provider network/billing behavior were not exercised. The intentionally ignored tests and native style qualification remain separate from the passing synthetic checks. This review establishes the listed code/reproduction findings, not a production security certification.

## Remediation order

1. Fix F1–F3 with preservation, profile-lifetime, and valid-profile backup regression tests. Keep these changes focused enough to verify their safety before broad movement of code.
2. Establish canonical domain/workflow ownership and one AI execution/accounting pipeline (F4–F6). Preserve existing authorization, guarded settlement, optimistic revisions, and atomic tracker finish behavior during extraction.
3. Break the frontend runtime cycle, reuse save coordination, and replace tracker point scans (F7–F8).
4. Consolidate current styles by owner and delete verified dead code (F9 and the unused ledger/editor identified above). Use measured behavior preservation to delete complexity, not file count as the success metric.

### Largest ownership hotspots

| File | Lines, including inline tests where applicable |
| --- | ---: |
| `app.css` | 6,059 |
| `ort-storage/src/lib.rs` | 5,491 |
| `App.tsx` | 3,270 |
| `application_materials.rs` | 3,177 |
| `ai_activity.rs` | 2,807 |
| `workspace-theme.css` | 2,141 |
| `ApplicationOverlay.tsx` | 1,861 |
| `AiWorkspace.tsx` | 1,665 |
| `ai_request.rs` | 1,633 |

Large inline test blocks account for part of the Rust totals. Moving tests out can improve navigation, but it cannot substitute for fixing the ownership and duplicated-policy issues established above.

## Remediation follow-up

All nine findings were addressed in the working tree on 2026-10-06. See the
[remediation report](2026-10-06-thermonuclear-remediation.md) for the final ownership
changes, regression coverage, passing verification gates and remaining native
qualification limits. This original report is retained as the historical audit.
