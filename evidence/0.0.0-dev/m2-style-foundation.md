# M2 style foundation checkpoint

Baseline: M1-qualified `65518eb9d434a5ff6158810000b8798261d9227c`.
Status: Step 5 in progress; local implementation only, not installed or natively
qualified. M1 remains complete for the recorded macOS-arm64 development scope.

PDF and DOCX now accept an optional reviewed style choice: plain, technical,
professional or modern. The three new templates establish typography and spacing
for the product's initial style categories. The application selector is now
integrated. Final product layout review and native reader qualification remain
outstanding. This checkpoint
does not satisfy the three qualified selectable styles gate.

## Compatibility and boundaries

- No style in a request continues to select the historical plain template. Plain
  PDF/DOCX bytes match all eight existing golden fixtures. Plain text is unchanged
  and does not accept a style field at its IPC boundary.
- Styled requests carry only source, expected saved revision and an enum value.
  Paths, template source, font selection and document content remain rejected.
  The request change is additive within command contract 2; document schema 1,
  stored source hashes, backup formats and the compatibility manifest stay intact.
- Styled DOCX receipts name the exact versioned template. Old plain receipts keep
  their original seven-field shape; cancellation remains a single status field.
  Format version 1 continues to identify the constrained OPC package format, with
  absent template identity retaining its original plain meaning.
- PDF receipts include the actual template ID and source hash. Local and portable
  history can select the original installed style. Replay exposes output only
  after the full receipt matches; an unknown or changed tuple is refused.
  Regeneration under a new tuple, with truthful labeling, is still separate work.
- Client checks bind the returned style to the request as well as source/revision.
  Export feedback uses the returned DOCX template instead of claiming plain output.
- All PDF styles share pinned fonts, the memory-only world, denied external files,
  no system font fallback, glyph/geometry checks, five-page and four-MiB bounds.
  DOCX keeps the same six constrained parts, semantic content order, escaping,
  safe hyperlinks and two-MiB bound. No dependency or capability was added.

## Local evidence

- `CI=true pnpm check`: formatting, TypeScript, frontend tests/build, static
  security and licenses passed (`target/m2-style-web-check.log`). Desktop tests:
  76 passed. The added contract receipt test also passed (21 contract tests;
  `target/m2-style-contract-tests.log`).
- Workspace Rust tests and all-target/all-feature Clippy passed, including style
  determinism, malformed selection, source identity, legacy receipt shape,
  overflow/glyph/link refusal and exact replay/tampered-tuple tests. Logs:
  `target/m2-style-workspace-tests.log`, `target/m2-style-clippy.log`.
- The generated contracts were regenerated and compared with the working-tree
  generation snapshot; byte-identical. `just verify-contracts` compares against
  the Git baseline, so its expected diff is not a clean-tree pass until the
  intentional generated changes are committed. No real commit was created.
- Independent PDF.js and Python ZIP/XML audits passed for eight legacy pairs and
  24 new styled PDF/DOCX pairs (`target/m2-style-independent.log`). Checks include
  exact content/order, safe links, tagged PDF structure and geometry, no active
  content, DOCX CRC/parts/semantic headings and lists, and exact bundled style XML.
  Existing plain golden hashes were enforced unchanged. The styled audit mode is
  explicitly not a new golden baseline or native qualification.
- First-page standard samples for all three PDF styles were rendered with Poppler
  and inspected: readable hierarchy and no clipping/overlap. This is sample visual
  evidence, not complete dense-pagination or native DOCX layout signoff. Samples:
  `target/m2-style-review/`; generated corpora: `target/m2-style-fixtures/`.

The installed app is still the M1-qualified artifact. No test-account switch,
Keychain access, installation, commit or push was performed for this checkpoint.

## Medium selector follow-up

The document-style selector defaults to Technical / Engineering and offers all
three product categories. It applies to new PDF previews and DOCX exports only;
plain text remains style-free. Selection is session-local and the UI explains
that reopening defaults to Technical / Engineering. It does not edit, autosave or
publish the resume. Persisting a preference would require separate storage design.

PDF previews and both history lists label the actual receipt style. Changing the
selector preserves an existing preview, explains the difference, and instructs
the user to generate a new preview to apply the new choice. Existing PDF export
continues to use the shown preview's exact bytes. No receipt checks, replay
eligibility, native commands or saved-revision guards changed in this UI slice.

Validation: 77 desktop tests and TypeScript passed
(`target/m2-style-selector-tests.log`). The added live test checks the default,
all selector options, unchanged contact content, absence of save/publication or
automatic re-render, selected-style command payloads, DOCX feedback, retained
preview style labels and axe accessibility. Full frontend checks passed in
`target/m2-style-selector-web-check.log`. Native interaction and final layout
qualification remain pending. No application was installed or test profile used.

The subsequent date/link schema foundation is recorded in
`m2-schema-v2-foundation.md`; structured editor activation is still pending.
Next high tasks include historical regeneration, sandboxed import and lifecycle
completion.
