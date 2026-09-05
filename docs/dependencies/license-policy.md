# Dependency license policy

Status: enforced locally and in CI for M0. Reviewed: 2026-09-05.

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

Single-OS optional packages without an installed sibling use a separate exact
`javascriptPlatformPackages` record. Currently this covers only MIT-licensed
`fsevents@2.3.3`: its locked integrity and `os: [darwin]` restriction must match
the reviewed record. On macOS the installed name, version, license, and OS
metadata must also match. Absence is accepted only on Linux and Windows. This
fixes both attached 2026-09-04 CI failures, which stopped at the license gate
because macOS-only fsevents is intentionally not installed on those runners.
It is not a license exception or a package-family wildcard. Version, integrity,
OS, metadata, duplicate-policy, and unsupported-platform changes fail closed.

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
