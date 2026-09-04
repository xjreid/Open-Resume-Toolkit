# M2 lockfile vulnerability-scan repair

Date: 2026-09-04. Base commit: `28401b0`; implementation changes are
uncommitted. Status: workflow repair implemented and locally inspected; first
GitHub Actions result pending.

## Failure diagnosis

The attached `Contracts, tests, and security checks` log ran on GitHub runner
2.337.0. Contract generation/drift, Prettier, TypeScript, JavaScript tests,
frontend builds, source security checks, strict Rust Clippy, all Rust tests, and
the independent DOCX/text verifier passed. The only failure was the final
production dependency audit.

Each of three `pnpm audit --audit-level high --prod` attempts reached npm's
public bulk-advisory POST, received no audit report, and was terminated by the
existing 75-second bound. The final process status was timeout code 124. The log
contains no advisory or vulnerable-package finding. A separate 15-second empty
diagnostic POST from the local verification host also received zero bytes and
timed out. This establishes endpoint unavailability for these runs; it does not
establish that the lockfiles are vulnerability-free.

## Repair

- The quality job no longer performs a fourth request sequence against the same
  unavailable npm advisory endpoint.
- A separate `Dependency vulnerability scan` workflow calls Google's official
  OSV reusable workflows, pinned to commit
  `880d9b542cc66d36d91d51b3fbcc038f5f28cfc5` (`v2.5.1`).
- Scan arguments name `pnpm-lock.yaml` and `Cargo.lock` explicitly, expanding
  coverage from production JavaScript packages to both locked ecosystems.
- Pull requests, pushes to `main`, manual runs, and a weekly schedule all use
  the full scan. Existing and newly introduced findings therefore block every
  trigger, and newly published advisories are detected even without a code
  change.
- The called workflow receives only `actions: read`, `contents: read`, and
  `security-events: write`, the permissions required to scan and publish its
  result. It receives no release or repository-write authority.
- The full scan's default `fail-on-vuln` behavior remains enabled. A finding or
  scanner failure is blocking; the change does not convert an unknown result
  into a passing security result.

## Local verification and limitations

- The edited workflow remains valid YAML under the repository's formatting
  check.
- Local lint, unit/component tests, production builds, source security checks,
  and Rust checks do not execute a GitHub reusable workflow. Therefore only a
  GitHub Actions run can complete this evidence.
- The npm endpoint remained unavailable during implementation, so no claim is
  made from the failed npm requests about current vulnerability status.
- The user handles commit and push.
