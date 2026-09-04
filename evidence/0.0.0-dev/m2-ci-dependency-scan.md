# M2 lockfile vulnerability-scan repair

Date: 2026-09-04. Base commit: `28401b0`; follow-up base commit: `9a14985`.
Status: workflow and reviewed exception policy implemented and locally
inspected; next GitHub Actions result pending.

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

## First full-scan result and follow-up

The first pushed OSV run successfully scanned all 163 pnpm packages and 719
Cargo packages, then blocked as designed. It reported 21 affected Rust packages
and 22 advisory IDs: 19 informational/unmaintained dependencies, the Linux-only
`glib::VariantStrIter` unsoundness, and two high-severity `quick-xml` denial-of-
service advisories. Three findings advertised a nominal fixed version, but none
has an in-range top-level lockfile update:

- GTK/ATK/GLib and `proc-macro-error` enter through Tauri's Linux WebKit graph.
  Linux is a compile/test host, not a supported or distributed ORT target, and
  ORT does not call the affected GLib iterator.
- Five unmaintained `rust-unic` crates enter through Tauri's `urlpattern`.
- The other informational notices enter through pinned Typst dependencies.
- `quick-xml 0.38.4` enters through `citationberg`/`hayagriva`. The vulnerable
  parser handles bibliography XML. ORT supplies only bounded structured resume
  data to a fixed embedded Typst template, has no bibliography input, and its
  renderer World denies every external file/package/plugin.

`osv-scanner.toml` now lists only those exact IDs, each with a concrete reason
and a 2026-12-04 expiry. The workflow passes that config explicitly while
retaining `fail-on-vuln`. `tools/tests/osv-policy.test.mjs` pins the reviewed ID
set, expiry, reasons, both lockfiles, and absence of broad package overrides or
a disabled failure gate. Newly published advisory IDs still fail immediately;
the current exceptions stop working at their review deadline.

Local verification passed the OSV policy regression test. The GitHub-hosted
scanner is the authority for the next full result; this evidence does not claim
that pending run has passed.
