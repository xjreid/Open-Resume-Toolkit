# Public credential-exposure audit — October 27, 2026

## Result and scope

No usable developer/provider token, private signing key, password, or other
credential was identified in the inspected public repository and built clients.
This is an evidence-based result for the revision below, not a guarantee against
unknown formats, future changes, or exploitation.

Public repository: `xjreid/Open-Resume-Toolkit`.
Public/local `main`: `55c4c38aad1890ec5b4eb756b20ac3cfe4a43f6e`.
The remote refs were queried and a separate fresh mirror was cloned from GitHub.
It contained one branch, no tags/PR refs, and **90 reachable commits**. The current
checkout had **1,429 tracked files** before these audit changes.

Inspected:

- All 90 commits using checksum-verified Gitleaks 8.30.1 default rules, followed
  by the reviewed repository configuration. Four initial findings were inspected
  individually; none was a usable secret.
- All **3,205 unique historical file blobs** (81,923,622 bytes materialized),
  scanning supported file contents, archives, and decoded content independently
  of Git patches. The final pass removes upstream global filename exclusions. The scanner examined about 78.74 MB of scannable content; binary
  image/font bytes are not a text-secret assurance.
- Tracked and unignored working source, public workflows, manifests, packaging,
  Rust provider/vault/request code, frontend key-entry/IPC/roster contracts,
  debug redaction, backup exclusions, and runtime endpoint restrictions.
- Fresh desktop and Chrome/Edge production frontend builds, with secret scans
  attached to each build. Renderer environment/dotenv canary regression.
- Public GitHub releases/issues/branches/Actions metadata and authenticated
  Secret Protection/settings/secret-scanning/code-scanning pages.
- Local OCR of **244 historical large images/screenshots**, followed by secret
  scanning of their extracted text; no detections. Small icons were not OCRed.
- The four historical resume PDFs: extracted text uses Alex Morgan and the
  `example.org` fixture address. Public support-contact addresses and Git author
  identity are public metadata, not authentication credentials.

## Reviewed scanner candidates

| Candidate | Assessment |
| --- | --- |
| `apps/extension/manifest/chrome-dev-key.json` | Base64 DER RSA SubjectPublicKeyInfo, intentionally public extension identity |
| `apps/extension/manifest/chrome-store-key.json` | Base64 DER RSA SubjectPublicKeyInfo, intentionally public extension identity |
| Historical `application.stage1.v1` literal in `crates/ort-backup/src/lib.rs` | Database setting name |
| `XChaCha20-Poly1305` in technical-reference documentation | Algorithm name |

No credentials were tested against a live provider. No OS vault secrets were
retrieved. The catalog contains its public verification key/signature; test-only
signing seeds are separate from the built-in catalog's public trust key.

## Client credential boundary

Provider API calls run in Rust using OS-vault credentials. The key roster sent
to the renderer contains IDs and metadata, not provider-key bytes. The local
key-entry form necessarily holds a user-entered key briefly before sending it
through IPC to the vault. This is not a shared developer secret distributed in
the app. Provider endpoints are exact HTTPS allowlists, redirects are disabled,
and credential-bearing HTTP header values are marked sensitive.

The renderer's production CSP restricts connections to Tauri IPC; the extension
has no provider credential contract. Existing tests cover vault/transport debug
redaction and exclusion of provider credentials from backups. The audit also
changes `ApiKey` cleanup to compiler-resistant `zeroize()` rather than `fill(0)`.
This does not establish resistance to a compromised operating system, injected
renderer code, runtime memory inspection, or future application vulnerabilities.

## GitHub state observed

- Secret Protection: **enabled**.
- Repository push protection: **enabled**.
- GitHub secret alerts: **0 open, 0 closed**, page states “No secrets found.”
- Private vulnerability reporting: **enabled**.
- `main` branch protection: **absent at audit start**.
- Releases: **0**. Issues/PRs: **0**. GitHub Pages: **disabled**.
- Actions metadata exposed one unexpired **OSV Scanner SARIF file** artifact.
  Anonymous artifact download required authentication (HTTP 401); Actions job
  log download returned HTTP 403. This audit does not claim those bytes scanned.
- Latest ordinary CI for the audited public head: **success**.
- Latest scheduled dependency scan (October 26): **failure**. The authenticated
  code-scanning page lists **15 open dependency alerts** (2 high, 8 medium,
  1 low, 4 warning), covering Vite/Vitest/launch-editor/undici and Rust dependency
  advisories including TLS trust roots, timing behavior, and Tokio I/O safety.
  These are separate from credential disclosure. They require dependency triage
  and remediation before claiming broader application security or release
  readiness; this audit does not dismiss or suppress them.

## Changes and verification

The old check recognized only three private-key headers and four filenames and
excluded `dist`. It did not detect ordinary provider/API tokens or inspect Git
history. Ignore rules did not cover common credential files. No local credential
hooks were configured.

The [secret-protection guide](secret-protection.md) documents the replacement:
verified pinned scanner, exact false-positive exceptions, credential-file guards,
staged/history/source/artifact scans, local hooks, independent CI, build/package
gates, disabled renderer environment injection, and incident response.

Local hooks were enabled in this checkout with `core.hooksPath=.githooks`.
New clones need `pnpm security:setup` or `just bootstrap`. Checks fail closed if
the scanner is missing or a complete history/build input cannot be inspected.

Synthetic regression tests cover leaked tokens, redacted failures, inline bypass
comments, exact public-key exceptions, ZIP members, symlinks, staged/working-copy
differences, deleted historical secrets, shallow clones, missing builds/scanners,
and renderer environment isolation. Source/history/build scans and focused
regressions were run locally. Hosted checks for these working-tree changes are
not implied by successful CI for the older public head.

## Remaining enforcement and assurance limits

The new workflow/build/configuration changes must be reviewed and published to
become the public repository's workflow. Require the new secret scan and CI
quality check under branch protection, including administrator restrictions.
Push protection is already active and should remain enabled.

No audit can promise that secrets will never become public. Local hooks can be
bypassed, CI runs after a push, provider patterns are incomplete, contributors
can upload sensitive screenshots or arbitrary low-entropy values, and privileged
maintainers can change enforcement. Public forks/caches, deleted or unreachable
GitHub objects, unavailable Actions logs/artifact bytes, and future releases are
outside a claim of full inspection. Never embed a shared secret in any client or
binary; require reviewed release inputs and credential rotation if exposure is
suspected.
