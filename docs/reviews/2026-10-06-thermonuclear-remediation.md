# Thermo-nuclear review remediation — 2026-10-06

All nine findings in the [original review](2026-10-05-thermonuclear-code-quality.md)
have been addressed. The fixes preserve complete resume content, retire profile
state together, and move application policy and serialized contracts into their
canonical owners. Regression tests exercise the defective cases identified in
the review, and the normal parallel Rust gate now passes.

## Finding closure

| Finding | Resulting implementation | Verification |
| --- | --- | --- |
| F1 — Lossy resume fields and mixed bodies | `ort-domain` owns field roles and the paragraph label. Editors update individual fields by identity; the canvas, reading view, AI context, plain text, PDF and DOCX preserve every ordinary/extra/paragraph field and retained bullet. Body conversion includes all existing body content. | Editor preservation tests, DOCX regression tests, and independent PDF.js/XML extraction confirm seven distinct markers across all four export styles and plain text. The new artifact gate is included in CI. |
| F2 — Profile lifetime and artifact cache collisions | One native retirement boundary clears previews, application exports and drag files, disconnects browser state, and notifies all webviews. `ProfileBoundary` remounts all profile owners. Application caches include the profile UUID; writes carry and verify the expected profile UUID. Retired save queues discard pending edits and stale acknowledgements. | A real encrypted-store/render regression replaces a profile with another at the same revision and rejects old cached bytes. Separate regressions cover lower fresh revisions, rejected retired-profile writes, queue disposal and frontend remount/listener cleanup. |
| F3 — Publication history rejected by backup | Backup writer format 1.6 removes arbitrary publication/AI activity count ceilings while retaining the authenticated aggregate payload budget. Collection inventories use 32-bit counts. Readers retain legacy format support, and frontend semantic validation uses the generated current format constant. No history is silently pruned. | An encrypted profile with 101 distinct publications backs up and restores all 101. Rust-produced format-6 inventories beyond the previous collection bounds pass TypeScript validation; current and legacy deterministic vectors pass. |
| F4 — Duplicated AI execution and discarded billing | `ai_execution.rs` owns reservation, dispatch, streaming, cancellation, parsing and settlement with explicit operation policy. Content acceptance and trustworthy billing evidence are separate. Known usage settles even when content is rejected; ambiguous evidence retains unresolved accounting. The unused in-memory ledger is deleted. | Production SQL settlement tests cover rejected content with known usage/cost and mismatched-model evidence with unknown cost. Existing provider parser, cancellation, retry, cap and durable-ledger tests pass. |
| F5 — Application policy in the native adapter | Canonical workspace/material models and setting policy live in `ort-domain`; typed encrypted workflow services, reviewed-edit checks, prompt policy and small artifact-source projection live in `ort-application`. Native commands retain authorization, AI invocation and platform handles. Tracker renders a real material source without fabricating a workspace. | Encrypted reload/revision, reviewed provenance, stage-one/capture, generated-material and export/preview regressions pass. Native tests are grouped by workflow, capture and artifacts rather than collected in another giant file. |
| F6 — Asserted IPC types and duplicate contracts | Rust serialization models produce schemas and TypeScript types. Command signatures produce paired command/argument/value contracts. A shared client receives unknown native responses and decodes them before state updates. AI, application and tracker components use these contracts; existing semantic validators consume generated type aliases and Rust-produced fixtures. | 39 contract tests and desktop client rejection tests cover actual native fixtures, malformed values/envelopes, request defaults, UUIDs and prototype-shaped property names. All 67 generated files remain byte-identical after regeneration. |
| F7 — Root-owned editor leaves and duplicate save queues | Shared canvas/reading/field/contact leaves no longer import `App`. Resume orchestration uses `use-resume-session` and the existing reducer. Both overlay save stages use one narrow revision-aware, coalescing coordinator. Unused duplicate editor implementations are deleted. | Desktop tests cover import/canvas reuse, published output, edit history, load/save/publish behavior, coalescing, explicit retry and retired queues. No shared consumer imports the root application. |
| F8 — Tracker scans and oversized list responses | Profile-scoped indexed point reads serve update/detail/preview. Bounded SQL summary pages project metadata and retained-content flags. Metadata updates preserve retained artifacts in native storage. The UI loads retained material only for detail/preview and uses summary pages for the table. | Storage regressions cover large retained content, small summary output, pagination/filter bounds, profile isolation and invalid IDs. Native metadata preservation and frontend autosave/detail tests pass. |
| F9 — Historical CSS override accumulation | One stylesheet entry imports tokens/controls and semantic shell, resume, application, export, import, tracker, AI and settings owners. Responsive/state rules stay with their family. Superseded declarations are removed, including cross-owner rules identified during browser comparison. | Twelve final browser comparisons match the original cascade across editor/contact, tracker, backup settings, populated AI keys/monitoring, professional/modern variants, and 360/800/1280-pixel layouts. The production build passes. |

## Structural results

| Hand-maintained owner | Before | After |
| --- | ---: | ---: |
| `App.tsx` | 3,270 lines | 834 lines |
| `application_materials.rs` | 3,177 lines | 870 lines |
| `ai_request.rs` | 1,633 lines | 767 lines |
| Workspace CSS | 8,200 lines in two accumulated sheets | 7,762 lines in 23 owner/import sheets; largest owner 913 lines |

The reductions include deletion of the unused ledger and duplicate editors, and
canonical ownership of workflow models, save coordination, field roles and
artifact source projection. Test extraction supports navigation; it is not the
basis for claiming the architecture fixes.

No new hand-maintained source file crosses 1,000 lines. The generated 1,079-line
`wire.ts` catalog is the explicit exception: it is deterministic schema output,
not a manually maintained component or policy owner. Splitting that generated
catalog would add imports without simplifying the source models. Existing large
owners such as storage and the overlay still contain further decomposition
opportunities; their sizes do not invalidate the specific ownership fixes above.

## Verification

| Check | Result |
| --- | --- |
| Rust workspace, locked/offline, all targets, normal parallel execution | 411 passed; 0 failed; 11 existing opt-in tests ignored |
| Native desktop suite after final test-module decomposition | 87 passed |
| Parallel platform stability check | 50 complete runs passed, including all five pipe scenarios and export crash tests |
| Clippy, workspace/all targets/all features, `-D warnings` | Passed |
| Rust formatting | Passed |
| Desktop TypeScript / Vitest | Passed; 44 files, 247 tests |
| Contracts TypeScript / Vitest | Passed; 9 files, 39 tests |
| Extension TypeScript/build and Chrome tests | Passed; 22 tests; Chrome, development bridge and Edge builds checked |
| Tooling Node tests | 32 passed |
| Desktop production build | Passed; Vite retains its warning about a JavaScript chunk above 500 kB |
| Prettier | Passed |
| Contract generation | Passed; 67 generated files unchanged after regeneration |
| Web security and secrets checks | Passed |
| Dependency license inventory | Passed; 793 Rust and 167 JavaScript packages |
| Independent plain DOCX/PDF fixtures | Eight DOCX/text and eight PDF/text fixtures passed golden, semantic, structure and geometry checks |
| Independent document-style corpus | 48 PDF/DOCX/text pairs passed local regression checks across technical/professional/modern and schema v1/v2 |
| Lossless-field export corpus | Seven retained markers survived PDF, DOCX and plain text for all four styles |
| Browser style comparisons | 492 computed properties per element; 638–666 DOM elements per compared state; all twelve final comparisons had zero differences |
| Patch whitespace checks | Passed |

The intermittent macOS pipe test was investigated under the parallel gate. Of
40 characterization runs with the concurrent export crash test, two failed; all
40 runs excluding that process-launch test passed. Isolating only the reader-drop
case exposed the same timing problem in an EOF case. Concurrent process creation
can temporarily inherit pipe ends before exec applies close-on-exec, so immediate
EOF/EPIPE assertions were testing unrelated process timing as well as the driver.
Each pipe scenario now executes its original assertions in its own child process;
the parent suite still runs in parallel with the export crash test. Production
descriptor code is unchanged. Fifty subsequent parallel platform runs and the
full workspace gate passed. This evidence supports test-process interference;
it is not a claim of a proven production descriptor leak.

Raw local evidence and the synthetic browser fixture are preserved in
`target/remediation-2026-10-06/` (ignored build output). The temporary browser tab,
viewport override, Vite server and source-tree fixture files were cleaned up.
The permanent regression tests and CI artifact gate remain in the repository.

## Compatibility and validation limits

Backup format 1.6 preserves complete histories within the existing 64 MiB
authenticated payload budget. Oversized archives fail explicitly. Earlier readers
reject the new writer version; current readers continue accepting supported
legacy archives. These constraints are documented in `docs/backup/backup-v1.md`.

The local pnpm wrapper attempted a dependency-state repair requiring registry
access and an interactive modules-directory replacement. Installed
TypeScript/Vitest/Vite/Node binaries were used directly, without reinstalling
dependencies. The listed checks therefore do not represent a clean execution of
the top-level `pnpm check` wrapper.

Live provider network/billing behavior, real OS-vault opt-in tests, native
signing/install, parser guest/native qualification and other operating systems
were not exercised. Browser style checks used synthetic native responses in the
real React application; they do not qualify a live native webview. Document style
native qualification remains separate from the local regression corpus. The
eleven existing opt-in tests remain ignored in the normal gate.

Pre-existing plan, design evidence, capability and milestone edits were preserved.
The remediation changes are left in the working tree for review; no commit or
push was requested.
