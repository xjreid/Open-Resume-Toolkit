# Secret protection

Credentials must never be committed, embedded in a frontend, copied into a
release, or included in logs, screenshots, issues, PRs, test evidence, or fixtures.
Use synthetic content. Public verification keys, signatures, extension IDs, and
random credential references are public identifiers; private signing keys and
provider credentials are not.

## Local setup and gates

Run `pnpm security:setup` once per clone, or use `just bootstrap`. This downloads
Gitleaks **8.30.1** and its detector configuration from official versioned
sources, verifies checked-in SHA-256 hashes, installs them under ignored
`target/security-tools/`, and enables
`.githooks`. It needs Python 3 and network access during setup. Subsequent scans
run offline. No developer or provider credentials are needed by a build.

- `pnpm check:security`: scans tracked and unignored source, including files
  already tracked despite ignore rules. Also rejects prohibited frontend APIs
  and environment access. Ignored local profiles/dependencies are not read.
- Pre-commit: scans actual staged bytes, not the working copy. Unresolved files,
  symlinks, submodules, sensitive filenames, and scanner failures block it.
- Pre-push and `pnpm check:secrets:history`: scan all local reachable Git refs,
  including deleted credentials. A shallow clone fails instead of passing.
- Desktop and production Chrome/Edge builds: scan the resulting `dist` files.
  Chrome packaging scans the exact input tree again before writing the ZIP.
- `node tools/check-secrets.mjs --artifacts PATH...`: scans explicit build or
  release directories, including ignored directories. Missing directories,
  symlinks, and sensitive filenames fail. Run this on every final release input
  before publication; a successful source scan does not certify a later binary.

The scanner traverses supported archives to depth 3 and encoded content to depth
5. It reports locations and rule IDs, never matching credential values. It
ignores inline `gitleaks:allow` comments and `.gitleaksignore` suppression files.
The configuration retains the pinned upstream detectors while removing their
global filename exclusions for SVG, lockfiles, scanner configuration, documents,
and other files. Installed detector bytes are hash-checked on each scan. An extra
rule rejects low-entropy credential literals in client source. It permits only four reviewed
false positives by exact value, path, and rule. Do not add blanket directory,
fixture, commit, or generic placeholder exclusions to silence findings.

The renderer disables Vite environment injection (`envPrefix: []`), dotenv
loading, and automatic copying of a `public` directory. It does not need
`VITE_*` credentials. A test builds a renderer with synthetic environment and
`.env.production` canaries to ensure they stay out of the output. Keep provider
requests and vault access in Rust. User-entered keys necessarily exist briefly
in the local key-entry form and IPC request; the saved roster and returned
contracts contain references/metadata rather than secret bytes. A client app
cannot hide a shared developer credential from someone who controls the client.

## GitHub enforcement

The independent **Secret scanning / Source and full Git history** workflow scans
source and full fetched history on every push, PR, weekly schedule, and manual
run. CI installs the verified scanner before builds. Actions have read-only
repository access, pinned action revisions, and no persisted checkout credentials.
No release-publishing workflow currently exists.

GitHub repository **Secret Protection** and **Push protection** must stay enabled.
Push protection can reject supported credentials before publication; Actions run
after code has reached GitHub. Local hooks are bypassable and are not a server
security boundary. GitHub push protection also has bypasses and detection limits.

Protect `main`: require pull requests, require the secret-scan and CI quality
checks, apply restrictions to administrators, and disallow force pushes/deletion.
Review changes to workflows, hooks, scanner/configuration/installer files, build
configuration, and release inputs carefully. A passing check can be defeated by
a change that disables its own checks unless repository review rules protect it.
Keep publishing/signing credentials in protected release environments or external
signing services. Never pass them through `VITE_*`, compile-time literals, public
artifacts, or frontend code. Qualify final signed artifacts before introducing
an automated publisher.

## If a secret is detected

Stop the commit, push, or publication. If the credential has reached GitHub or a
public artifact, revoke/rotate it at its provider immediately; deleting a file
or rewriting history alone does not make the old credential safe. Then remove it
from the current tree, inspect reachable history, releases, Actions logs and
artifacts, PRs/issues, screenshots, and forks/caches. Coordinate any history
rewrite and cached-content removal using GitHub's sensitive-data process. Report
privately under [the security policy](../../.github/SECURITY.md). Never copy the
value into a public report or test.

## Limits

Pattern matching does not prove the absence of all secrets, weak passwords,
personal data, image text, encrypted/nested unsupported archives, unpublished
refs, deleted GitHub caches, or credentials outside the inspected repository.
Ordinary CI does not OCR screenshots. Review all screenshots and documents before
publication. Updates to dependencies and periodic security review remain needed;
secret scanning does not establish that dependencies or the application are free
of exploitable vulnerabilities.

See the [October 27, 2026 audit](security-audit-2026-10-27.md) for the exact public
revision, verified settings, findings, and outstanding actions.
