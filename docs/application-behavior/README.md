# Application behavior reference

Last reviewed: 2026-09-20

This folder documents how the implemented desktop application behaves from a
user and troubleshooting perspective. It records workflows, state changes,
failure behavior, and the boundaries between data that can look similar in the
UI but has different persistence rules.

These documents are a descriptive map of the current implementation. Approved
product behavior still belongs in `Product Plans/`, architectural decisions in
`docs/adr/`, and implementation design in `Implementation Plans/`. If this
reference disagrees with tested code, treat the discrepancy as documentation
drift and resolve it before relying on either description.

## Reading order

| Document | Contents |
| --- | --- |
| [AI keys and spending](ai-keys-and-spending.md) | Adding, naming, organizing, activating, pausing, testing, removing, models, per-key limits, and the general limit. |
| [AI data and monitoring](ai-data-and-monitoring.md) | Data selection, graph behavior, export, activity clearing, retention, removed-key data deletion, and accounting boundaries. |
| [Resume workspace](resume-workspace.md) | Resume creation, Edit and View modes, sections and entries, autosave, styles, publication, and PDF/DOCX export. |
| [AI resume tailoring](resume-tailoring.md) | Editorial freedom, the seven template regions, factual grounding, refinement, and response validation. |
| [Shared application behavior](shared-application-behavior.md) | Main navigation, encrypted storage, import, backup/recovery, destructive operations, and common failure rules. |
| [Maintenance and troubleshooting map](maintenance-and-troubleshooting.md) | How to update this reference and where to find the corresponding UI, native commands, storage logic, and tests. |

## Core distinctions

Several product concepts must remain separate during development:

- An **active key** is the one key selected for ordinary AI requests. A saved
  key can exist without being active.
- **Recorded activity** powers the Data graphs. **Lifetime spend** and
  **spending-cap progress** are durable accounting values and are not reduced
  when graph activity is cleared.
- Removing an API key removes the credential from My Keys, but keeps its
  historical activity selectable in Data. Deleting removed-key data is a
  separate, irreversible action.
- A **saved resume** is the editable draft revision. A **published resume** is
  an immutable snapshot of a successfully saved revision.
- Resume style controls presentation and export rendering; it does not replace
  the underlying resume content model.

## Documentation rule for future changes

Before changing user-visible desktop behavior, review the applicable document
in this folder. Update it in the same change whenever any of the following
changes:

- what a control does or when it is enabled;
- which state or data survives another action;
- confirmation, failure, retry, or recovery behavior;
- terminology visible to the user;
- selection, sorting, aggregation, filtering, or export scope;
- the relationship between a saved draft, published snapshot, external file,
  API key, activity record, lifetime total, or spending cap.

Keep this folder organized by workflow rather than by source file. Add a new
document for a substantial application area and link it from this index.
