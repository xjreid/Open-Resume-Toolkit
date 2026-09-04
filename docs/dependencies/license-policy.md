# Dependency license policy

Status: enforced locally and in CI for M0. Reviewed: 2026-09-04.

`config/dependency-license-policy.json` is the review boundary for dependency
licenses. The allowlist contains SPDX identifiers compatible with the project's
GPL-3.0-only distribution. It deliberately excludes AGPL, SSPL, noncommercial,
source-available, unknown, and missing-license terms. Adding an identifier is a
licensing decision, not a routine dependency update.

`node tools/check-licenses.mjs` checks the complete Cargo metadata graph and the
pnpm lockfile without installing another audit tool. Rust packages must declare
an SPDX expression. JavaScript packages are matched to the exact locked
name/version and their installed package manifest. Platform-specific optional
packages that cannot be installed on the current OS are accepted only through a
family-and-version-exact policy whose declared license must also match an
installed representative from that family. The macOS Intel/Apple Silicon and
Windows CI jobs repeat the JavaScript check against their target installations.

SPDX `OR` expressions pass when at least one branch is allowed. Every term in a
selected `AND` branch and every `WITH` exception must be allowed. Legacy slash
expressions used by crates are treated as `OR`. Missing or unparseable metadata,
unknown platform families, changed family versions, duplicate lock keys, and
unused package exceptions fail closed.

The checker writes a deterministic machine-readable inventory to
`target/licenses/dependency-inventory.json`. This generated build evidence is
not committed. A narrowly reviewed package exception, if ever necessary, must
identify one ecosystem/name/version/license tuple, include a concrete reason,
and carry a future review-expiry date; the checker rejects duplicate, expired,
or unused exceptions. The current policy has none.

Binary assets remain separate review items. PDFium and bundled font provenance,
licenses, notices, and digests are recorded under `docs/dependencies/pdf/` and
`docs/dependencies/document-import.md`; final package/SBOM verification remains a
release gate.
