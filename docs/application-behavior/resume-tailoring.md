# AI resume tailoring and refinement

Tailor and Refine with AI produce selective, source-grounded resumes targeting
exactly one rendered PDF page. Both use the pinned published master as factual
source and the reviewed job description for priorities. Refinement additionally
uses the current reviewed resume and the user's correction instruction.

## Editorial behavior

The model identifies four to six important role qualifications, or fewer when
the job description supplies fewer. It selects the strongest direct evidence,
writes concise and specific descriptions, removes weak or redundant material,
and orders sections and entries by relevance. Education has no automatic
priority. Reverse chronology within employment remains preferred unless there
is a strong relevance reason to depart.

Existing sections and entries may be reordered or omitted. Their retained
headings and identities cannot be created, renamed, split, merged, or moved
between sections. Names, titles, dates, metrics, contact information, links,
and all subsection header metadata remain protected. The app copies protected
fields locally rather than asking the model to reproduce them. Description rewrites
must preserve meaning and exact metrics. One sentence and two rendered lines
per bullet are editorial guidance; the app measures whole-document page count,
not individual line geometry.

## Protected headers and editable bodies

All retained subsection header fields are copied unchanged: title, role,
details, dates, location, extra, and any additional named header fields. The AI
cannot select, reorder, rewrite, or remove individual header fields or items
within them. Sections and subsections can still be omitted as a whole.

Skills and coursework in body content use the regular source-backed body
contract. Their relevance can guide selection, reordering, and rewriting while
preserving facts. Skills or coursework placed in header fields remain protected.

The former **AI list selection** controls and provider selection contract were
removed because header classification did not fit this distinction. Existing
`isSkill` and `listKind` metadata remains readable for saved-document compatibility,
but grants no AI editing permission and is not included in provider context.
Manual editing still permits users to change header values directly.

## Bounded quality pipeline

Each operation uses at most four paid provider calls:

1. Draft a complete candidate.
2. Always review its source support and editorial quality, returning a complete
   improved candidate. A malformed first response is repaired in this slot.
3. Correct unresolved review issues, invalid output, or measured PDF overflow.
4. If another correction is needed, make a final revision that resolves as many
   issues as possible. Use this locally valid final candidate even if the model
   still reports review issues or the resume exceeds one rendered PDF page.

Calls two and three may finish early when the candidate has no unresolved review
issues and renders to one page. Call four explicitly returns the best complete
revision rather than rejecting it on editorial quality or page fit. Unreadable
responses, invalid document structure, and invalid source references still
cannot be saved; these are local contract failures, not model review judgments.

Every valid candidate is rendered with the frozen selected style. Corrections
receive validation failures, content-length diagnostics and actual page count,
or the renderer's layout-limit failure. Fonts, margins, spacing, layout, and glyphs never shrink to obtain a
pass. A local contract failure after four calls leaves the saved workspace intact. Cancellation,
provider failures, unknown provider usage, and spending-cap rejection stop the
operation without dispatching another correction.

The source, job, baseline, style, provider/model, credentials, profile, and
revisions are fixed for the operation. One overlay lease covers every call and
the final save. Each call reserves its own maximum cost, including the structured
output schema, and records a separately settled attempt linked to one logical
operation. Successful resume-quality stages may continue that operation; no
automatic transport retry is added. A profile or revision conflict prevents
stale results from replacing saved work.

Manual editing and export retain their existing behavior, including multipage
exports. One page remains the AI target; the final revision is accepted even
when page fit fails. A renderer failure does not discard the final valid resume,
but export may require a manual content adjustment before files can be prepared.

## Evidence and refinement

Schema v7 requires each bullet/paragraph to cite published bullet or field IDs.
References must be nonempty, unique, known, and owned by the entry's published
anchor. Every entry uses exactly one published anchor; published entries may
anchor only themselves. Unknown or duplicate
identities, cross-entry references, protected output fields, obsolete list-selection
requests, empty sections, malformed documents, and document-limit violations are
rejected locally. Valid paragraph items are packed into consecutive native
paragraph fields without exceeding the per-field limit. This preserves multiple
long source paragraphs; oversized individual items still fail validation.

References provide traceability; they do not prove that every rewritten claim
is true. The mandatory model review checks source support, role relevance,
distinct value, and concision. `reviewIssues` cannot override local validation.
The user still reviews the final resume. New applicant facts belong in the
master and must be published before tailoring uses them.

Refinement can restore published sections, entries, and body content that
previous tailoring omitted. Current reviewed headers win for retained identities;
absent identities are copied from the published source. Unrelated reviewed body
edits and ordering should remain where possible, with minimal additional cuts
when necessary for one page.

## Review and provider boundary

The overlay shows the phase, call number out of four, and latest measured page
count during generation. The existing tailoring
notes explain job-specific decisions. **What changed** is computed locally from
the before/after documents and summarizes actual removals, restoration,
reordering and description edits. The export renderer supplies
the displayed page count; unsaved edits invalidate the visible page-fit claim.

All providers share the same v7 contract: `tailoringPlan`, `rolePriorities`,
`reviewIssues`, `roleInfo`, `templateSections`, and `alerts`. OpenAI uses the
strict Responses output schema; Gemini uses its existing structured-output
adapter with unsupported string-length keywords omitted and checked locally.
Older response schemas cannot bypass the overlay permissions. Existing saved
resumes and workspaces remain readable.

The provider response contract is v7 because header-list selection was removed.
It is separate from saved-resume schema versions; no saved document migration
or republishing is required to protect existing headers.

Qualification alerts still check exact mandatory job excerpts against the full
published source rather than the selective draft. Cover letters and answers
retain their existing single-call prose contract. Provider completion/model
checks, pinned-host handling, and accounting remain in the existing request
boundary. No prompts or responses are retained in the usage ledger.

## Failure diagnostics

The popup retains native error details instead of reducing every failure to a
code. It shows the selected model, provider, failed call and phase, elapsed time,
HTTP status or Gemini finish reason when available, operation/attempt IDs, and
specific local format, source-reference, or PDF-fit failures. Correction passes
receive those specific validation failures within the existing four-call limit.

Data → Monitoring → **View recent failures** shows the latest ten failed or
uncertain attempts for the selected period and keys, newest first. Expand an
error row for timing, call number,
requested/serving model, observed token counts, usage completeness, and the
locally authored diagnostic. HTTP 503, rate limits, transport timeouts, output
token limits, policy blocks, incomplete streams, and material validation are
distinct. Older records retain their broad category; missing historical details
are not guessed. Model-authored review issues are represented only by a count.

Diagnostics are bounded, encrypted with the activity ledger, included in activity
JSON and portable backups, and removed with their associated activity. They
exclude credentials, prompts, raw provider messages, generated response bodies,
and resume/job text. Rejected content still retains trustworthy billing evidence.
Recording a material failure does not change settled costs or spending caps.

The synthetic evaluation corpus and an opt-in live comparison procedure are in
`fixtures/ai/tailoring-v7/README.md`.
