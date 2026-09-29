# Qualification alerts audit — 2026-09-29

Alerts are integrated into the application overlay's Resume tab. The workflow
is provider output → local qualification validation → encrypted application
workspace → `ApplicationOverlay` points. Generation and loading both pass alerts
to the UI. The latest presentation is a static list of concise points, with no
quotes, evidence blocks, hide, dismiss, or reopen controls. Legacy dismissal and
ignore-all fields remain compatible with stored workspaces but no longer hide
anything. Alerts are advisory and do not block exporting.
The panel is now omitted entirely when no validated alerts are present, as
requested in the subsequent display refinement.

The investigation used source inspection and synthetic fixtures. It did not
open the installed application, read user resumes or credentials, or call a
live provider. Historical provider responses are not retained, so the exact
reason each earlier application had no alerts cannot be established here.

## Defects corrected in source

| Defect | Previous behavior | Correction |
| --- | --- | --- |
| Hidden empty panel | Zero accepted alerts removed the entire alert UI, with no visible status. | The panel remains visible with “No required qualification alerts to show.” |
| Required-heading context ignored | `Required qualifications: / Python` was discarded unless the copied bullet itself contained one of five mandatory phrases. | Exact bullets can inherit status from an explicit required heading. Preferred and unrelated headings stop that context. |
| Narrow mandatory wording | `Must be proficient in Python` did not match `must have` or `must possess`. | Additional explicit “must” forms are accepted. |
| Mixed required/preferred excerpts | `Python is required; Go preferred` discarded the Python candidate because the whole excerpt contained “preferred.” | Mandatory status is checked in the clause containing the target; preferences are still rejected. |
| One-letter skill targets rejected | R and C failed the two-byte minimum. | Nonempty alphanumeric targets are accepted with the existing exact-target and presence checks. |
| Any link suppressed portfolio alerts | A LinkedIn or unrelated contact link was treated as sufficient portfolio evidence. | Links must explicitly identify a portfolio/work sample; link labels and URLs are included in the published-text presence check. |
| Refinement erased previous alerts | Every refinement replaced alerts and reset dismissal choices, even though the job and pinned source were unchanged. | Existing alerts, IDs, and dismissal choices remain; new validated alerts are merged within the ten-alert limit. |
| Optional model guidance | The shared example used an empty array and the instructions said alerts “may” be emitted. | Both initial tailoring and refinement explicitly require the qualification check, independent of body edits. This improves prompting but cannot guarantee model recall. |

The strict provider schema also specifies the ten-alert limit. Exact job
excerpts, source absence, category restrictions, and exclusion of personal
attestations remain required. Expanded “must be” wording includes explicit
work-authorization exclusions.

## Remaining limits and potential failure points

- **Model omission:** alerts still originate from the same AI response as the
  tailored resume. An empty array is valid, and no independent second pass or
  deterministic qualification extractor verifies that the model found every
  requirement. The empty UI status does not mean every qualification is met.
- **Exact excerpts and targets:** paraphrased excerpts, changed whitespace,
  expanded skill names absent from the excerpt, unsupported categories, or
  unverified evidence are discarded. Malformed alert objects can reject the
  entire material response. The prompt now asks for precise copied clauses.
- **Intentional scope:** experience-duration alerts are disabled. Personal
  eligibility and attestations, preferences, and inferred qualifications are
  excluded. Broad personal-requirement filters can also suppress legitimate
  domain-specific wording, such as medical technology or race-condition work.
- **Graduation evidence:** confirmed mismatches resolve only named fields
  whose labels explicitly identify graduation and whose values are a single
  year. Structured `ResumeDate` headers and ordinary education date ranges do
  not supply usable evidence IDs through the current alert contract. No date
  mismatch inference was added by this audit.
- **Literal presence:** aliases and equivalent qualifications are not resolved
  semantically by the validator. Any occurrence of a target phrase in the full
  published resume can suppress a not-found alert, even if the surrounding
  context does not establish the requested level or proficiency.
- **Bounded heading parsing:** heading context recognizes explicit required
  headings and standard section boundaries within sixty preceding lines.
  Unusual formatting or unlabeled sections can still be missed.
- **Candidate truncation:** the local validator examines at most twenty
  candidates and saves ten. Its truncation flag is based on candidate count,
  rather than the number that survived validation.

## Verification

Regression coverage checks required headings, common mandatory wording,
one-letter skills, mixed preferences, personal exclusions, exact excerpts,
repeated excerpts with Unicode byte offsets, unrelated and explicitly labeled
portfolio links, and absence against the entire published resume.

A schema-v5 fixture passes through validation, PDF preflight, encrypted
workspace persistence, and UI-shaped serialization with an accepted alert.
Refinement tests preserve earlier alerts and saved dismissal preferences.
React tests cover initial generation, saved-workspace display, empty state,
concise skill/language/mismatch points, and visibility despite legacy hide flags.
The UI can format experience-duration points, but duration detection remains
disabled in the qualification validator.

The workflow audit passed 316 automated tests before the presentation update.
The static presentation update passed 320 tests (44 AI, 76 desktop backend,
200 frontend), plus the production build, Clippy with warnings denied,
formatting, and security checks. It checks target retention and compatibility
with older saved alert data; installation verification is recorded with the build.

Live provider recall and the user's historical applications remain unverified.
