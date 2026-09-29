# AI resume tailoring and refinement

Tailoring uses the full published master resume and the reviewed job description.
Within one response, the model first returns 1–3 concrete, brief tailoring points.
Each point connects a specific job need, supporting published evidence, and an
applicable body edit or ordering decision. The overlay displays them under
Tailoring notes. The model then tailors the main information bodies using the
applicant's actual experience, the job description, and those initial points.
It must check that the completed draft applies the points. This is a model
self-check, not independent verification of editorial quality.

The AI may rewrite, condense, or reorder body bullets and use paragraphs. It may
reorder existing sections and entries within their existing sections, or remove
whole sections or entries when they weaken the job-specific resume. It cannot
create, rename, combine, or split sections or entries. Retained section headings
stay unchanged.

Ordering considers both job relevance and general resume conventions. The
prompt prioritizes Education as the first section; a departure requires a
strong, job-specific reason briefly stated in a tailoring point. Within
sections, reverse chronological order is preferred when applicable unless a
strong relevance reason supports another order. These editorial judgments are
prompt requirements, not mechanically scored by the validator.

## Fixed template, protected headers

Only the main information body at the bottom of an entry is editable by AI:

| AI region | Permission |
| --- | --- |
| Title | Protected entry heading |
| Role | Protected entry subheading |
| Details / skills | Protected fields beside the title |
| Date | Protected free-text date range and structured dates |
| Location | Protected entry location |
| Extra | Protected right-side metadata |
| Main info | Editable bullets or body paragraphs |

Header values are read-only context in the request. They are absent from the
provider's output contract: each returned section contains only its existing ID
and retained entries; each entry contains only its ID, published source IDs,
and `mainInfo`. The app copies headers locally from the editing baseline,
preserving labels, skill flags, field IDs, whitespace, structured dates, and
links. This avoids asking the model to reproduce immutable strings exactly.

Unknown or duplicate identities, extra output fields, and moves of entries into
other sections are rejected. Contact details, document style, and layout remain
app-owned. Ordering is canonicalized. The overlay accepts only the current
template response schema, so older response formats cannot bypass these
permissions. Existing document and rendering limits still apply.

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

The AI receives both the full published factual source and the current reviewed
resume. It first returns 1–3 concrete points scoped to the correction instruction,
then a complete replacement document. The same body-only permissions apply,
even if a correction requests a protected-field change. Protected values come
from the current reviewed resume, preserving user edits to headers, contact
details, and links. Current-only entries may remain when they anchor supporting
published entries. Unrelated reviewed bodies and ordering should be preserved;
whole sections or entries may be removed when the correction calls for it.

The existing PDF preflight and revision checks run before replacing the saved
workspace; malformed, unauthorized, or unrenderable output leaves it intact.
Generated body claims still require user review. New applicant facts should be
added to the master and published before tailoring with them.

## Provider boundary

Tailoring and refinement use response schema v5. Every response has
`tailoringPlan` before `templateSections`: 1–3 nonempty, distinct
single-line strings, each no more than 500 characters. The response also
contains `roleInfo` and `alerts`. All providers receive the same
explicit protected-header, editable-body, and alert contract. Alerts require exact job-description
excerpts, apply only to explicit mandatory resume-related requirements, and
check absence against the full published master resume rather than the tailored
draft. OpenAI additionally receives a strict output schema through the
Responses API's `text.format`, following the official
[Structured Outputs documentation](https://developers.openai.com/api/docs/guides/structured-outputs).
That constrains response shape; local validation enforces protected values.

Requests use the active key and selected model already configured by the user.
The output limit follows the trusted catalog, up to the existing application
maximum. OpenAI and Gemini input-cost reservations include the output schema. No automatic
paid retry or additional model call is added. Cover letters and question answers
keep their existing prose response contract.

Gemini 3.6 Flash is available through the signed Balanced catalog, and Gemini
3.5 Flash Lite through Economy. Tailoring and
refinement send Google's structured-output `responseFormat.text` with the
body-only schema, `mimeType: APPLICATION_JSON`, and `thinkingLevel: LOW`.
These fields are REST enum values; the older `responseMimeType` field accepts
the MIME string `application/json` instead. The request regression tests check
both models against a snapshot of Google's public
[v1beta discovery contract](https://generativelanguage.googleapis.com/$discovery/rest?version=v1beta),
including key tests, tailoring, refinement, and other material requests.
Unsupported string-length schema
keywords are omitted for Gemini; the local validator still enforces those
limits. Other Gemini material requests retain their existing JSON contract.
See Google's [model documentation](https://ai.google.dev/gemini-api/docs/models/gemini-3.6-flash)
and [structured-output guide](https://ai.google.dev/gemini-api/docs/generate-content/structured-output).

Completed responses that fail material validation now receive a format-specific
message. Missing completion and serving-model mismatches have their own
messages, without advising users to shorten a valid job description.
HTTP 400 request rejections and HTTP 404 model-access failures also receive
distinct messages; provider error bodies are not retained.

Gemini responses require an explicit successful candidate completion (`STOP`).
Missing completion, token-limit termination, safety blocking, and other failure
reasons are rejected even when the accumulated text is valid JSON. EOF alone
does not confirm a successful result. The same rule applies to key tests.
