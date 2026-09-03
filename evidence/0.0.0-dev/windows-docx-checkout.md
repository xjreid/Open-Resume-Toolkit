# Windows DOCX determinism and CI runtime repair

Date: 2026-09-03 UTC. Base commit: `e349856`.
Platform used for this repair: macOS arm64. Status: locally verified repair;
the new four-job CI result, especially Windows, is still required.

Follow-up (before the PDF checkpoint): the user confirmed all four CI jobs
passed for `748d13b`. No run URL was independently retrieved. This closes the
reported CI repair gate, not the separate native UI/vault/filesystem gates.

## Failure and cause

The user reported three of four CI jobs passing and supplied the Windows log
for [run 33700811611, job 100479492326](https://github.com/xjreid/Open-Resume-Toolkit/actions/runs/33700811611/job/100479492326).
The log was inspected locally; the linked job could not be retrieved independently.
Windows frontend/Rust checking and the executed Rust tests passed, including
encrypted startup and DOCX storage. At 00:58:22 UTC the independent Python
verifier failed `review required: deterministic DOCX bytes changed`.
OpenSSL debug-symbol warnings and the Node-action deprecation warning were not
the failing assertion. The SQLCipher logging-recursion repair remains intact.

The four bundled XML assets in `crates/ort-documents/src/docx` were LF in Git
but had no checkout attributes. `include_str!` packages those file bytes
verbatim. A Windows-style `core.autocrlf=true` checkout changed them to CRLF,
altering part lengths, CRCs, ZIP offsets and whole-package SHA-256 values despite
equivalent XML semantics. `.editorconfig` does not control Git checkout behavior.
Git documents the interaction between `text`, `eol` and `core.autocrlf` in its
[attributes reference](https://git-scm.com/docs/gitattributes).

## Implemented repair

- `.gitattributes` sets `text eol=lf` only for the bundled DOCX XML directory.
  It also covers future XML assets there, without normalizing unrelated files.
  The exporter, layout version and all five reviewed golden hashes are unchanged.
- Four Node regression tests use isolated temporary Git repositories and real
  checkout conversion with `core.autocrlf=true`, `false` and `input`. They compare
  all existing assets and a synthetic future asset byte-for-byte, retain unrelated
  text/binary controls, and reproduce CRLF drift when the policy is removed.
  No real repository index, global configuration, credential or network is used.
- The checkout tests run before expensive build steps in all four CI jobs and
  within the existing `pnpm test` / `just check` path. Rust and Python also reject
  CR/CRLF in generated XML with a targeted diagnostic. Python gains a seventh
  altered-package rejection control and names the fixture/expected/actual hash
  when any other golden mismatch occurs. No test or golden check is skipped.
- Both `actions/setup-node` uses now pin v7.0.0 at
  `820762786026740c76f36085b0efc47a31fe5020`. The official
  [release](https://github.com/actions/setup-node/releases/tag/v7.0.0),
  [commit](https://github.com/actions/setup-node/commit/820762786026740c76f36085b0efc47a31fe5020)
  and [pinned action metadata](https://github.com/actions/setup-node/blob/820762786026740c76f36085b0efc47a31fe5020/action.yml)
  were checked; the action runs on Node 24. Its documented minimum runner is
  2.327.1, below the supplied runner's 2.337.0. Automatic package caching is
  explicitly disabled to preserve the existing workflow's behavior. `.nvmrc`,
  pnpm/Rust pins, permissions and credential persistence policy are unchanged.

## Verification actually run

1. A separate local clone of the base commit was checked out with CRLF conversion.
   `git ls-files --eol` confirmed all four assets as `w/crlf`. Its fixture generator
   reproduced the original Python golden assertion. This is a macOS simulation
   of Git's Windows checkout behavior, not a Windows-native execution claim.
2. With the scoped attribute rule applied, sources were materialized into a
   **fresh** directory and built offline using a separate Cargo target directory.
   The original, unmodified verifier passed all five original goldens. Each fixed
   package was exactly 28 bytes smaller; only the four embedded assets differed,
   and removing CRLF from the old XML exactly reproduced the new XML. Sizes:
   standard 7053, sparse 5169, Unicode 7100, hostile 7122, dense 21645 bytes.
3. The new verifier rejects the deliberately CRLF-built corpus with the explicit
   checkout diagnostic. The repaired corpus passes the strengthened verifier,
   including all seven negative controls.
4. The normal workspace fixture generator and strengthened independent verifier
   pass. The affected local Rust build artifacts were rebuilt after the deliberate
   CRLF reproduction to avoid reusing its artifacts; no source/data was removed.
5. Full local `just check` passes: formatting, TypeScript, Node/Vitest tests,
   frontend/extension builds, web/secret policy, Rust formatting, workspace Clippy
   with warnings denied, and workspace/all-target tests. The existing opt-in
   OS-vault test remains ignored as designed; no native credential test was run.
6. `git diff --check` passes. No contracts, application dependencies, XML assets, golden hashes,
   SQLCipher flags, encryption/memory protection or import-enablement gates changed.

The final synthetic corpus is ignored under `target/docx-ci-repair-final`.
No desktop, Word, LibreOffice, Keychain or Credential Manager was launched.
Visual rerendering was not repeated: repaired bytes equal the already-reviewed
goldens. This does not expand the prior document-reader/accessibility evidence.

## Remaining gates and handoff

Commit/push is left to the user. Confirm all four new CI jobs before further
feature implementation. The action runtime update has been verified against
upstream metadata but cannot be executed as a hosted action locally.
Existing Windows checkouts can retain old CRLF working files until rematerialized;
use a fresh checkout, preserving any local edits, rather than a global Git change.

M2 remains incomplete. Import remains disabled (`IMPORT_ENABLED=false`, worker
exit 78); no parser/containment, Windows UI/vault, native Word, PDF rendering,
export recovery/replacement or release gate is waived by this repair.

Suggested commit: `fix(ci): preserve DOCX golden bytes on Windows and update Node action`
