# Open Resume Toolkit — Product Context

<!-- impeccable:product-schema 1 -->

This is a concise context record for Impeccable. The subject-specific documents in
[Product Plans](<Product Plans/Product_Scope_and_Principles.md>) remain authoritative,
following the precedence in [Plan_Index.md](Plan_Index.md). This record summarizes
approved behavior; it does not establish new scope or certify release readiness.

## Platform

web

This value describes the React/TypeScript interface rendered in Tauri desktop
webviews, not a browser-hosted resume service. The desktop runtime is Tauri 2 with
Rust. macOS Apple Silicon is the active development and preview target; Windows
and Intel Mac are later expansions. Chrome and Edge extensions are narrow capture
companions. English is the initial interface and document language.

## Users

Individual job seekers maintaining their resume and preparing materials for a
specific application. They work with job descriptions and application questions
in a browser alongside a desktop editor and separate overlay. No narrower career,
demographic, or research-backed audience segment has been established here.

## Product Purpose

Maintain one structured master resume and prepare reviewed, job-specific resumes,
optional cover letters, and application answers. Users can edit, import, preview,
export, and track applications locally. Success means users understand what is
saved, what changed, what will be sent to a provider, and which materials they
choose to retain. ORT makes no interview, employment, or universal ATS guarantees.

## Positioning

ORT is free, open source, and local-first, with useful manual workflows independent
of AI. It operates no ORT account service, hosted document store, subscription,
feature tiers, advertising, or centrally funded inference. Optional AI uses a
deliberately selected personal provider API key or eligible ChatGPT/Codex
subscription connection; it never silently switches provider or model.

## Operating Context

- The main desktop window owns master-resume editing and publication, the tracker,
  aggregate AI monitoring, settings, backup, and recovery.
- There is one autosaved master draft and at most one deliberately published
  snapshot. Tailoring uses the published snapshot, never unpublished draft edits.
- The separate desktop overlay owns the one active application workspace: capture
  and review job text, tailor, review changes and qualification alerts, then work
  across Resume, Cover letter, and Answers tabs.
- Extensions capture selected content after an explicit authorized action. They
  hold no AI credentials and provide no parallel tailoring workflow.
- Finish Application optionally retains selected structured materials in the
  local tracker, removes unselected temporary material, and resets the workspace.

## Capabilities and Constraints

These are approved product requirements; consult implementation and milestone
evidence before representing any particular integration as available.

- Structured, versioned JSON is the editable source of truth. PDF, DOCX, and text
  are locally rendered exports; styles change presentation without losing content.
- Imports require review before changing the master. AI output is a proposal and
  must not invent qualifications, dates, employers, achievements, or personal facts.
- Manual editing, rendering, export, and tracking remain useful without AI.
  Provider requests require connectivity and explicit transmission awareness.
- Required Qualification Alerts distinguish confirmed conflicts from facts not
  found in the published master. They remain dismissible and non-blocking, and
  never become an eligibility decision or fit score.
- Aggregate local usage and optional spend/quota guardrails must explain estimated,
  partial, unavailable, and account-wide measurements honestly.
- User content and retained records remain local; backup and migration are explicit
  user-controlled operations. Cloud synchronization and recovery are not implied.
- No automatic submission, automatic form completion, continuous browser
  monitoring, employer tooling, universal ATS scoring, or mobile service is promised.
- Unresolved and deferred product work stays in
  [the central register](<Product Plans/Release_Scope_and_Open_Decisions.md>).

## Brand Commitments

The public name is Open Resume Toolkit, abbreviated ORT. Copy is direct, specific,
functional, and documentation-oriented; it explains actions, state, limits,
privacy, installation, and source without inflated AI or employment claims.

Future interface work preserves the approved Precision Workbench direction,
Quiet Navy identity, Offset Open Frame assets, and light-only application UI.
The existing [aesthetic direction](Aesthetic/README.md) and
[visual rules](Aesthetic/Precision_Workbench_Visual_Direction.md) supply the visual
authority. Resume and cover-letter documents remain independent professional
documents and do not inherit ORT branding, logos, colors, or promotional language.

## Evidence on Hand

- [Product scope and principles](<Product Plans/Product_Scope_and_Principles.md>)
  and [core workflows](<Product Plans/Core_Workflows.md>) establish product truth.
- [Application behavior reference](docs/application-behavior/README.md), the
  desktop code in `apps/desktop`, and milestone records in `evidence/0.0.0-dev`
  document implementation behavior and its verification limits.
- [Logo assets](Aesthetic/Logo/README.md),
  [document examples](Aesthetic/Resume-Designs/README.md), and the original visual
  reference in `Aesthetic/Reference` provide existing design evidence. Reference
  layouts are illustrative and do not override approved workflows.
- No customer testimonials, measured employment outcomes, universal ATS evidence,
  or user-research findings were established by this initialization. Do not
  fabricate them or treat design references as proof of usability.

## Product Principles

1. Preserve local ownership, recoverable data, and explicit user control.
2. Keep master editing in the main window and job-specific work in the overlay.
3. Treat AI output as reviewable proposals grounded in the published master.
4. Preserve manual usefulness and portable structured documents.
5. Describe capabilities, costs, limitations, and support status truthfully.

## Accessibility & Inclusion

Target WCAG 2.2 AA principles where applicable and equivalent native-platform
expectations, as specified in
[the quality and accessibility plan](<Product Plans/Quality_Accessibility_and_Verification.md>).
Preserve keyboard operation, logical focus, screen-reader labels and status
announcements, visible focus, scalable text and zoom, reduced motion, accessible
dialogs, and meaning independent of color. Graphs need equivalent textual
summaries. The overlay must not trap focus or obstruct essential browser and OS
controls. Exported documents need readable selectable text and meaningful order.
