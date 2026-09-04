# M0 dependency-license and contract-drift gates

Date: 2026-09-04. Source baseline: `5f4b33e`; the repository commit containing
this checkpoint is authoritative. Local verification platform: macOS arm64.

## Implemented gate

- A committed SPDX allowlist covers compatible permissive, notice-bearing, and
  project GPL-3.0-only terms. No package exceptions are approved.
- The license checker validates the complete locked Cargo metadata graph and
  every pnpm lock package. It compares installed JavaScript manifests directly
  and handles other-target optional binaries only through exact family/version
  declarations backed by an installed representative.
- Missing, malformed, unknown, mismatched, duplicate, and stale-exception cases
  fail closed. Unit tests exercise SPDX OR/AND/WITH and legacy slash behavior,
  lock parsing, exact platform-family matching, and build/CI wiring.
- Each JavaScript workspace package now declares `GPL-3.0-only` explicitly.
- `just check` now runs the pinned Cargo contract generator directly and rejects
  a generated-file diff in addition to running the license policy. CI runs the full license graph on the
  Ubuntu quality job and repeats the target JavaScript check on macOS arm64,
  macOS Intel, and Windows.
- A successful check writes the deterministic inventory to
  `target/licenses/dependency-inventory.json`; it is generated evidence rather
  than source.

## Local result

The focused policy run passed 727 Rust packages and 167 JavaScript/workspace
packages with zero exceptions. The five policy tests passed. The complete
canonical `just check` passed locally, including formatting, lint, 24 repository
policy tests, 2 extension tests, 20 contract tests, 62 desktop tests, all Rust
format/strict-Clippy/tests, license inventory, security checks, builds, and a
clean contract regeneration. Hosted CI for this checkpoint remains to be
recorded.

## Remaining M0 evidence

This closes the identified M0 tooling implementation gap, not every native exit
criterion. Clean-checkout bootstrap, installed main/overlay health commands,
development/stable profile isolation, and installed production-bundle remote
asset/capability inspection still need current macOS and Windows evidence. Final
package notices and SBOM generation remain a protected-release gate.
