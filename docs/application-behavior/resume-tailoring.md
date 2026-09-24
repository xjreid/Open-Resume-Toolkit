# AI resume tailoring and refinement

Tailoring uses the published master resume and the reviewed job description.
Within one response, it generates three direct tailoring priorities before the
resume content. Each priority connects a concrete job need, supporting
published-master evidence, and a specific edit to make in the completed resume. The app displays those three
priorities under Tailoring notes for review. The prompt then requires the model
to write the complete resume and check that it implements each priority. This is a model self-check,
not an independent verification of editorial quality.

For initial tailoring, the model can make a comprehensive relevance edit: it
can remove redundant or irrelevant content, rename, create, remove, combine,
split, move, and reorder sections and entries, including derived summaries and
skills sections. It can rewrite or replace bullets and use a paragraph where
that reads better. Strong existing wording can remain if it serves a priority;
the model does not need to substitute words merely to look different. All
claims remain grounded in the published master resume.

## Fixed template, editable content

The AI fills the existing entry regions; it cannot supply a new layout:

| AI region | Existing resume representation |
| --- | --- |
| Title | Entry heading |
| Role | Entry subheading |
| Details | Details field beside the title |
| Date | Free-text date range, or retained structured dates when unchanged |
| Location | Entry location |
| Extra | Extra field in the right-side metadata |
| Main info | Bullets or the existing body-paragraph field |

The app assigns IDs to new items and canonicalizes ordering. Contact details
and retained entry links are copied locally from the editing baseline. The
selected document style and PDF renderer remain app-owned. Empty strings omit
optional regions. Existing document length and rendering limits still apply.

## Factual grounding

Every generated entry references at least one entry in the published resume.
Unknown references, duplicate retained identities, unsupported output fields,
and oversized or malformed documents are rejected. Prompts prohibit invented
experience, qualifications, tools, metrics, dates, seniority, or achievements;
they also prohibit moving an accomplishment to an unrelated employer.
Job requirements guide emphasis and are never evidence that the applicant has
those qualifications. Embedded resume or job text is treated as data.

These references and structural checks do **not** prove every generated claim
is true. The user still reviews the draft. New applicant facts should be added
to the master and published before starting an application using them.

## Refinement

The AI receives both the published factual source and the current reviewed
resume. The correction instruction is an authorized editorial request.
Refinement first returns three direct priorities scoped to that instruction.
For a narrow correction, a priority may describe a concrete preservation
decision backed by the published master, rather than expanding the requested
scope. It then returns a complete replacement document that addresses the
request and preserves unrelated reviewed content and order. It can retain
sections and entries introduced by earlier tailoring, and keeps current contact
details and retained links. Unchanged displayed dates retain their structured
precision; edited dates use the existing free-text date region without
duplicating them.

The model can still make unwanted editorial changes, so the result remains
reviewable. The existing PDF preflight and revision checks run before replacing
the saved workspace; malformed or unrenderable output leaves it intact.

## Provider boundary

Tailoring and refinement use response schema v4. Every response has
`tailoringPlan` before `templateSections`: exactly three nonempty, distinct
single-line strings, each no more than 500 characters. The response also
contains `roleInfo` and `alerts`. All providers receive the same
explicit seven-region and alert contract. Alerts require exact job-description
excerpts, apply only to explicit mandatory resume-related requirements, and
check absence against the full published master resume rather than the tailored
draft. OpenAI additionally receives a strict output schema through the
Responses API's `text.format`, following the official
[Structured Outputs documentation](https://developers.openai.com/api/docs/guides/structured-outputs).
That constrains response shape while allowing free text inside the regions.

Requests use the active key and selected model already configured by the user.
The output limit follows the trusted catalog, up to the existing application
maximum. OpenAI input-cost reservations include the output schema. No automatic
paid retry or additional model call is added. Cover letters and question answers
keep their existing prose response contract.
