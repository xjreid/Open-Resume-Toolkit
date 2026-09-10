# Delivery roadmap

## Current position

| Milestone | Status | Outcome |
| --- | --- | --- |
| M0 | Complete | Architecture skeleton and contracts |
| M1 | Complete | Encrypted local core and structured resume |
| M2 | Complete | Offline editing, reviewed import, publication and export |
| M2.5 | Next | Usable desktop layout, approved aesthetic and professional resume designs |
| M3 | Planned | Direct AI foundation |
| M4 | Planned | Tailoring, alerts and application materials |
| M5 | Planned | Workspace, tracker and browser bridge |
| M6 | Optional | External Codex integration |
| M7 | Planned | Distribution and stable hardening |
| M8 | Planned | Static project website |

M0–M2 completion applies to macOS Apple Silicon development. See the
[closure and accepted limitations](../../evidence/0.0.0-dev/m2-acceptance-closure.md).
Windows and Intel Mac remain deferred; existing shared CI is not a native-support
claim. The test-account signing-trust issue remains a packaging/setup follow-up.

Start with the [next milestone guide](../Next_Milestones.md). Detailed technical
behavior belongs in the component and product plans, not this status document.
[Earlier checkpoint history](Delivery_Roadmap_History_2026-09-09.md) is archived.

## Testing approach

The user approved a lighter testing policy for M2.5 and M3–M8. The
[quality and verification plan](../../Product%20Plans/Quality_Accessibility_and_Verification.md)
is authoritative: focused automated checks, one short feature walkthrough, and
representative critical-failure checks. Aim for a 5–15 minute manual walkthrough
per milestone once the build is ready; this is a planning estimate, not a reason
to hide a defect or skip a required protection.

No mandatory soak tests, elapsed-time waits, fuzz campaigns, forced process/disk
failures, exhaustive model/OS combinations, or repeated full acceptance sessions.
Use mocks and controllable clocks for slow/error paths. Expand investigation only
for a concrete failure or changed high-risk behavior. Existing quick CI remains;
this planning revision does not edit CI or remove working tests.

## M2.5 — usable desktop and resume designs

**Deliver:** the approved Precision Workbench / Quiet Navy aesthetic throughout
an organized app shell, navigation and dialogs; a document-centered, user-friendly
resume builder with focused editing; coherent import/publish/preview/export flows;
a clean Jake's Resume-style Technical template and equally professional Business
and Modern variants. This is overall usability and layout work, not just recoloring.

**Minimum checks:** a short create/edit/publish/export/reopen walkthrough, style
switching without data loss, representative PDF/DOCX visual review, keyboard focus
and smaller-window layout. Show the actual app and all three document designs for
user review before proceeding to M3. No extended testing session.

See the [M2.5 implementation plan](../Desktop%20Application/M2_5_Usable_Desktop_and_Resume_Designs.md).

## M3 — direct AI foundation

**Deliver:** No AI / Direct API state; vault-backed credentials; OpenAI, Anthropic
and Gemini adapters; signed versioned model/preset/pricing catalog; operation and
attempt accounting with cancellation/retry/recovery; streaming; token/cost
normalization and transactional spending caps; aggregate AI Monitoring, period
controls, breakdowns, export, clearing and separate cap resets.

**Minimum checks:** adapter fixtures for each provider; one live synthetic request
on the configured provider when credentials are available; cancel/failure UI;
representative cost arithmetic, cap rejection and duplicate/concurrent reservation
checks; credential redaction and monitoring/export consistency. Use controlled
fixtures for missing usage and interrupted attempts. Missing live-provider evidence
is documented, not replaced by a lengthy account-setup exercise.

## M4 — tailoring, alerts, and application materials

**Deliver:** versioned prompts/schemas; evidence-backed factual validation; at most
three change points; same-call Required Qualification Alerts with validated evidence
and dismiss/ignore/reopen; Stage 2 resume, cover-letter and answer flows; editable
preview, explicit regeneration instruction, question reset and PDF download/drag.

**Minimum checks:** one representative generation/edit/export journey and a small
fixture set for unsupported claims, malformed output, prohibited answers and
required-versus-preferred alerts. Confirm alerts remain non-blocking and user edits
persist. No broad adversarial benchmark or per-preset statistical threshold gate.

## M5 — workspace, tracker, and browser bridge

**Deliver:** workspace/tracker transitions; atomic Finish Application; Stage 1
capture/review; retained snapshots, search/filter/reopen; authenticated Chrome/Edge
native messaging, install/repair/status and version handling. Optional overlay
initiation stays separately gated by its documented permission model.

**Minimum checks:** one capture-to-tracker walkthrough in Chrome and a short Edge
smoke check on the active Mac; save/reopen and failed-save preservation; representative
wrong-client, replay/oversized message, desktop-absent and version-mismatch rejection.
Confirm capture does not start AI automatically. No full browser/profile/OS matrix.

## M6 — optional external Codex

**Deliver:** verified official-runtime discovery and supported-version negotiation;
isolated ORT Codex home; managed sign-in/keyring; stdio adapter, cancellation and
lifecycle; account/rate-limit views and quota controls; containment and safe disablement.

**Minimum checks:** one supported runtime's connect/request/cancel/sign-out smoke;
controlled wrong-identity/version and forbidden tool/file/command event rejection;
representative quota/missing-data behavior and child cleanup. Runtime isolation
and strict event handling remain implementation requirements. A smoke pass does
not establish isolation by itself: inspect the enforcement configuration/code.
If safe containment cannot be established, leave Codex mode disabled and defer M6.
No exhaustive runtime/platform matrix or specialized bypass campaign is required.

M6 may be deferred without blocking M7.

## M7 — distribution and stable hardening

**Deliver:** the active macOS preview package and later-signing readiness; signed
updater metadata where updates are enabled; release channels and recovery;
checksums, dependency/license inventory and provenance; extension Store packages
and compatibility sequencing; support/diagnostic runbooks. Windows NSIS, SignPath
and Store work remains deferred until that platform is explicitly activated.

**Minimum checks:** one install/launch/update-or-reinstall/uninstall cycle for the
channel being shipped; one representative supported-version migration/backup reopen;
artifact hash/signature or documented unsigned-preview identity; invalid updater
metadata rejection where applicable; keyboard and brief screen-reader/readability
spot check of the main journey. Review new dependencies/notices and known critical
issues. No multi-VM matrix, full accessibility campaign or performance soak gate.

Publish the exact checked artifacts and truthful limitations. Unsupported update
paths stay disabled. Unsigned preview acceptance is not signed-release acceptance.

## M8 — static project website

**Deliver:** approved product/docs/download/support/legal pages driven by real release
metadata, with no resume upload, account, hosted AI or backend user-data service.

**Minimum checks:** build; primary links/download identity; one desktop and one narrow
viewport; keyboard navigation and readable content. No broad device/browser matrix
or load-testing campaign for the static site.

## Rules for implementation

- Preserve local-first privacy, explicit provider transmission, authenticated IPC,
  immutable published/versioned records and safe failure behavior.
- Migrations remain forward-only; recovery uses a compatible safety copy. Extend
  deletion/backup inventories when a milestone adds persistent data or credentials.
- Keep unfinished or unsafe features absent or disabled. Record actual passes,
  failures and unrun checks; do not convert missing evidence into a pass.
- Use risk-proportionate reasoning and review under the user's current agent
  preferences. A risk marker does not require a separate manual approval session.
- Each work item needs its outcome, relevant plan, main data/security impact and
  a short verification note. Elaborate evidence packages are optional.

## Deferred scope

Windows/Intel native qualification; Linux/mobile; cloud sync/accounts/hosted keys
or resume storage; locally hosted models; Safari/Firefox; automatic job submission;
macOS signing/notarization until the approved trigger; additional themes/dark mode.
The approved light aesthetic and three document styles remain product scope.
