# Next milestones

## Authority and current boundary

The [delivery roadmap](System%20Documentation/Delivery_Roadmap.md) is the
authoritative source for milestone scope, dependencies, exit evidence, and
deferrals. M0–M4 are complete for the macOS Apple Silicon development scope;
their boundaries are recorded in the
[M2 acceptance closure](../evidence/0.0.0-dev/m2-acceptance-closure.md),
[M2.5 closure](../evidence/0.0.0-dev/m2.5/closure.md), and
[M4 closure](../evidence/0.0.0-dev/m4/closure.md).

Testing follows the [focused quality policy](../Product%20Plans/Quality_Accessibility_and_Verification.md):
small automated checks and one short walkthrough, not a repeated full matrix.

M3 Direct AI foundation and M4 application materials are complete; M5 is in
progress. Do not treat this guide as a replacement for the roadmap or the
applicable product plans.

## M2.5 closure — usable desktop and resume designs

Read the [M2.5 plan](Desktop%20Application/M2_5_Usable_Desktop_and_Resume_Designs.md),
the [approved aesthetic](../Aesthetic/Precision_Workbench_Visual_Direction.md)
and the [resume editor specification](../Product%20Plans/Resume_Editor_and_Schema.md).
The implementation and accepted limitations are recorded in the
[M2.5 closure](../evidence/0.0.0-dev/m2.5/closure.md).

## M3 completed route

Use these in order when reviewing or maintaining M3 work:

1. [M3 in the delivery roadmap](System%20Documentation/Delivery_Roadmap.md#m3--direct-ai-foundation) for the complete deliverables and exit evidence.
2. [AI and import product plan](../Product%20Plans/AI_and_Import.md) for connection modes, credentials, provider behavior, accounting, and guardrails.
3. [Local data and document model](../Product%20Plans/Local_Data_and_Document_Model.md) and [local data retention and recovery](../Product%20Plans/Local_Data_Retention_and_Recovery.md) for durable records, retention, backups, and deletion.
4. [Security and threat model](System%20Documentation/Security_and_Threat_Model.md), [architecture](System%20Documentation/Architecture.md), and [AI/document-processing component plan](AI%20and%20Document%20Processing/AI_and_Document_Processing_Plan.md) for the implementation boundary.
5. [Configuration limits and defaults](../Product%20Plans/Configuration_Limits_and_Defaults.md) and [quality, accessibility, and verification](../Product%20Plans/Quality_Accessibility_and_Verification.md) for limits and evidence expectations.

M3 establishes Direct API connection state, vault-backed credentials, the three
provider adapters, catalog verification, operation accounting, streaming,
spend-cap enforcement, and aggregate AI Monitoring. Use its roadmap exit
evidence to assess M3 completion.

## Ordered follow-on milestones

| Order | Milestone | Sequence/context | Primary starting documents |
|---|---|---|---|
| M4 | Tailoring, alerts, and application materials | Complete; maintenance reference | [M4 roadmap section](System%20Documentation/Delivery_Roadmap.md#m4--tailoring-alerts-and-application-materials), [Core workflows](../Product%20Plans/Core_Workflows.md), [AI and import](../Product%20Plans/AI_and_Import.md) |
| M5 | Workspace, tracker, and browser bridge | In progress; development app/host; no paid ORT signing | [M5 roadmap section](System%20Documentation/Delivery_Roadmap.md#m5--workspace-tracker-and-browser-bridge), [Core workflows](../Product%20Plans/Core_Workflows.md), [desktop-extension communication](../Product%20Plans/Desktop_Extension_Communication.md), [browser extension and IPC plan](Browser%20Extensions/Browser_Extension_and_IPC_Plan.md) |
| M6 | Optional external Codex | Only if its containment gate passes | [M6 roadmap section](System%20Documentation/Delivery_Roadmap.md#m6--optional-external-codex), [AI and import](../Product%20Plans/AI_and_Import.md), [security and threat model](System%20Documentation/Security_and_Threat_Model.md) |
| M7 | Distribution and stable hardening | Required production signing and bridge/Store qualification; M6 may be deferred | [M7 roadmap section](System%20Documentation/Delivery_Roadmap.md#m7--distribution-and-stable-hardening), [distribution and updates](../Product%20Plans/Distribution_and_Updates.md), [quality, accessibility, and verification](../Product%20Plans/Quality_Accessibility_and_Verification.md) |

The roadmap's cross-milestone rules and explicit deferrals apply throughout.
