# Quality, accessibility, and verification

## Quality principles

- Critical user actions fail safely and preserve the last valid local state.
- The project makes no durability promise without tested backup and migration behavior.
- The same structured input produces consistent preview and export within a declared renderer/template version.
- Provider outages or API changes degrade AI features without blocking manual editing, rendering, tracking, backup, or export.
- Desktop, extensions, native host, updater, and local schema are versioned and tested as one compatibility system.

## Requirement traceability

Future implementation planning assigns stable identifiers using these categories:

- `PRD` — product-wide principles, scope, terminology, and settings.
- `RES` — master draft, publish, schema, import review, editing, and rendering.
- `APP` — application workspace, tailoring, Finish Application, tracker, and retained materials.
- `AI` — direct-provider and Codex adapters, credentials/authentication, model catalogs, prompts, validation, generation, activity/usage accounting, pricing estimates, spend/quota guardrails, and evaluation.
- `DAT` — local storage, encryption, migrations, retention, backup, export, and deletion.
- `EXT` — browser capture, native messaging, local IPC, installation, and repair.
- `SEC` — privacy, security, import isolation, diagnostics, and supply chain.
- `DST` — packaging, signing, Stores, releases, updates, and rollback.
- `OSS` — license, dependencies, contributions, governance, and release provenance.
- `QLT` — compatibility, performance, accessibility, document quality, and release evidence.

Each traceability record contains the requirement, authoritative file/section, applicable platforms and data categories, owning component, dependencies and central configuration values, verification type, objective pass criterion, evidence location, owner, status, and applicable release gate.

## Accessibility

The desktop app, overlay, and extensions target WCAG 2.2 AA principles where applicable and equivalent native-platform accessibility expectations.

Required coverage includes:

- Complete keyboard operation and logical focus order
- Screen-reader labels, roles, status announcements, and error recovery
- High contrast, scalable text, zoom, and reduced-motion behavior
- No color-only meaning
- Accessible dialogs for destructive, provider-transmission, update, and Finish Application decisions
- Accessible AI Monitoring period controls, token/cost graphs with equivalent textual summaries, aggregate breakdowns, mode/model controls, spending/quota-cap forms, progress warnings, blocked states, exports, and clearing/reset confirmations without color-only cost, quota, or error meaning
- Readable exported documents with selectable text, meaningful ordering, links, and appropriate tagging where supported
- Overlay behavior that does not trap focus or obstruct essential browser/OS controls
- Required Qualification Alerts that are keyboard and screen-reader accessible, do not rely on color alone, expose the requirement and evidence relationship clearly, can be dismissed/ignored/reopened, and never seize focus or block the workflow

## Testing policy — M2.5 and M3 onward, revised 2026-09-09

The user replaced the previous exhaustive acceptance approach with focused,
lightweight verification. This policy supersedes broader test-matrix, soak,
fuzz-campaign, specialized-harness and evidence-package requirements elsewhere
in the plans for remaining milestones. M0–M2 stay closed under their recorded
acceptance scope. Actual product protections above and in the security/data plans
remain requirements; testing depth and completion procedure are reduced.

### Default completion check

1. Build/typecheck and run the existing fast checks relevant to changed behavior.
2. Exercise one representative user journey for the new feature. Aim for a single
   5–15 minute manual session after setup; use automation when it is simpler.
3. Check the main failure/cancel path and a small set of critical protections
   affected by the change (for example credential redaction, rejected unauthenticated
   messages, spending caps or preservation of saved data).
4. Record the build/commit, checks and outcomes, plus known limitations. A short
   note or ordinary test output is sufficient.

Do not require real-time expiry waits, overnight runs, repeated full-account
sessions, disk-filling, forced shutdown/crash campaigns, custom destructive
harnesses, long fuzzing, statistical AI evaluations, broad performance benchmarks,
or every combination of provider/model/browser/OS/package. Use controlled clocks,
mocks and small deterministic fixtures for time, cost, failure and recovery paths.
These specialized activities are removed from default milestone/release gates;
use one only when a specific observed defect makes it worthwhile, explaining why.

Retain working automated tests and existing quick CI. Do not rerun a passing suite
without changed code, a failure or another concrete reason. This document does
not itself remove or reconfigure CI jobs. Fix reproducible critical failures;
record minor limitations or defer the affected feature rather than expanding the
whole acceptance matrix. Skipped checks are unrun, never implicit passes.

### Platform and account scope

Use the active macOS Apple Silicon development environment for M3–M8 until another
platform is explicitly activated. Use the developer account by default. A separate
account or clean installation is needed only when that boundary is itself changing
or when checking the actual distributed package. Browser work gets one primary
Chrome journey and an Edge smoke check; no multi-profile/version matrix is required.
Test one supported runtime/provider configuration live where available, with small
adapter fixtures for the other implemented providers. Record missing credentials
or unavailable environments as limits instead of making setup a separate test project.

### Accessibility and output

Preserve the accessibility behavior listed above. For changed UI, check keyboard
access, visible focus, labels and understandable errors; use a short screen-reader
spot check for a new major interaction. Inspect one representative export or preview
when its layout changes. Existing automated accessibility/render checks are useful;
full assistive-technology, template/content and platform combinations are not gates.

### AI and critical boundaries

Use a small synthetic fixture set covering valid structured output, one unsupported
claim, one malformed/prohibited response and relevant alert behavior. Verify costs
with a few known arithmetic examples and representative cap/duplicate-reservation
rejection. Do not make paid live calls across every preset a completion requirement.
A changed adapter or prompt gets affected checks, not the full AI system retest.

Authentication, containment, no-tool runtime restrictions, provider-transmission
consent, encrypted storage and atomic writes remain implemented protections. Review
the relevant enforcement code/configuration and run bounded negative checks when
changing those boundaries. No known critical exposure should ship merely because
the happy path passed; disable or defer the affected feature if needed.

### Distribution check

For the channel actually being shipped, check one install/launch/update-or-reinstall/
uninstall cycle and one supported-version data migration/backup reopen. Check the
artifact identity, applicable updater signature rejection, shipped notices and
truthful preview/support limitations. Inspect changed logging/diagnostics for secrets.
A concise review of known critical issues replaces a blanket specialized audit gate.
Additional platforms/channels need their own small relevant check when introduced,
not a pre-emptive all-platform campaign.

See the [delivery roadmap](../Implementation%20Plans/System%20Documentation/Delivery_Roadmap.md)
for each milestone's minimum checks. The detailed feature plans define behavior;
this policy defines the proportionate verification needed to close the work.
