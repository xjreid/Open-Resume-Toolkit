# Contributing to Open Resume Toolkit

Open Resume Toolkit is in active implementation for macOS Apple Silicon, with
working desktop, direct-AI, tracker, Chrome development-capture, and optional
Codex paths. It is not yet a production-qualified stable release. Start with the
[current implementation map](../docs/development/current-state.md) and
[development guide](../DEVELOPMENT.md).

## Before contributing

1. Read the repository [README](../README.md) and
   [plan index](../Plan_Index.md).
2. Search existing issues and discussions before opening a new topic.
3. For a material change, open or join an issue before doing substantial work.
4. Do not include resumes, job applications, API keys, credentials, private
   correspondence, vulnerability details, or other personal or confidential
   information in public issues, discussions, commits, or pull requests.
5. Follow the [Code of Conduct](CODE_OF_CONDUCT.md).

Security vulnerabilities must follow the private process in
[SECURITY.md](SECURITY.md), not the public issue tracker.

## Contribution scope

Welcome contributions include:

- Corrections that make approved product behavior clearer or more consistent.
- Identification of conflicts, missing failure cases, privacy risks, security
  risks, accessibility gaps, and unsupported claims.
- Test journeys and objective acceptance criteria.
- Research attached to an existing open decision, with sources and assumptions.
- Small repository-documentation and community-process improvements.

Implementation plans must follow the authority and change-control rules in
`Plan_Index.md`. Do not present a framework, provider, platform, or deferred
feature as approved merely by adding it to a supporting document.

## Substantial code contributions

The architecture and dependency policy are implemented. The existing restriction
on substantial external code contributions remains while contributor licensing
and the recorded GPLv3-versus-AGPLv3 decision are unresolved. Do not begin a large
code contribution without an issue explicitly confirming that it is ready to
accept. This restriction does not mean implementation has not begun.

## Pull-request expectations

A contribution should:

- Have one clear purpose and link to the relevant issue or plan requirement.
- Preserve the public/private repository boundary.
- Avoid unrelated formatting or restructuring.
- Update every authoritative and supporting plan affected by a product-rule
  change.
- Include verification appropriate to the change; use the [CI guide](../docs/development/ci.md) and preserve default and feature-enabled checks.
- Update current behavior/setup documentation when controls, state, packaging, or persistence changes; keep historical evidence labeled as historical.
- Identify third-party content and its license or provenance.
- Contain no generated artifacts, personal data, secrets, or unrelated files.

## Contribution license and sign-off

By submitting a contribution, you agree to license your contribution under GNU
GPL version 3 only (`GPL-3.0-only`) and, for original material to which you hold
the necessary rights, to apply the Section 7(b) attribution term in
[`ADDITIONAL_TERMS.md`](../ADDITIONAL_TERMS.md). You retain copyright in your
contribution. The project does not presently require copyright assignment.

Contributions must use the Developer Certificate of Origin sign-off. Add the
following line to each commit, using your real name and an email address you are
authorized to use:

```text
Signed-off-by: Your Name <your-email@example.com>
```

The sign-off certifies the
[Developer Certificate of Origin 1.1](https://developercertificate.org/). You
can add it automatically with `git commit -s`.

The project may revise this inbound process before substantial implementation
contributions open. Already accepted contributions remain governed by the
terms under which they were submitted.
