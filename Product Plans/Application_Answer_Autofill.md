# Application-answer autofill and reusable answer library

## Authority, status, and scope

**Status: approved M5 scope; planned, not implemented.** The user confirmed this
design on October 19, 2026. This document owns the product behavior for reusable
application answers. The [technical plan](../Implementation%20Plans/Browser%20Extensions/Application_Answer_Autofill_Plan.md)
owns its component boundaries and implementation sequence. The
[delivery roadmap](../Implementation%20Plans/System%20Documentation/Delivery_Roadmap.md)
owns milestone acceptance. This addition does not close M5 or change M0–M4
acceptance, optional M6, or M7 production signing/Store requirements.

Initial scope is Chrome and Edge on macOS Apple Silicon. This is a separately
opted-in exception to the prohibition on automatic form reading and modification.
It never authorizes unrelated/background collection, automatic submission, AI
calls, or operation without the connected desktop and available local profile.
Existing deliberate rectangle capture and the overlay's AI Answers tab remain
separate workflows.

## Application answers in the desktop

Add **Application answers** to the main navigation after Tracker and before
Settings, using the existing Open Folio design and shared accessible controls.
It is a reusable profile library, not another current-job workspace.

The top of the section provides one three-option mode control:

| Mode | Automatic observation/learning | Automatic field writes |
| --- | --- | --- |
| Off | None | None |
| Capture only | Eligible completed user edits | None |
| Autofill + capture | Eligible completed user edits | Authorized matching empty fields |

Fresh profiles start Off. A selected mode survives ordinary desktop/browser
restarts, but every new connection requires fresh desktop authorization. Quit,
disconnection, unavailable profile, or incompatible components pause automation.
Backup restore always starts Off. Merely installing an extension, granting site
access, connecting a browser, or opening an application does not enable a mode.

First-enable consent explains that ORT learns answers into an encrypted local
library and that filling discloses the answer to the active website, which may
read it immediately before submission. It explains foreground scope, exclusions,
approval rules, and how to stop. It does not imply AI use or an ORT cloud copy.

Below the mode control show connection/activity status, including active,
paused, unavailable, unsupported page/control, permission needed, and stopping.
Provide a searchable, paginated list of saved questions/answers with edit-answer,
delete, reuse-policy, and alias controls, plus a **Needs review** queue for
ordinary corrections. Load list metadata/previews separately from full values.
Users may explicitly add an entry. Deletion removes its aliases and corrections
and invalidates outstanding uses; it does not rewrite an already completed form.

## Eligible pages and fields

Automation follows only the active tab in the focused browser window. The
desktop arbitrates Chrome/Edge authorization so background browsers do not
process pages. Work resumes through fresh validation rather than stale requests.

Recognizable job-application forms qualify automatically while a mode is enabled.
An ambiguous page needs **Use on this application page** approval. Do not learn
from unrelated surveys, account settings, login, or checkout forms. A user's
website exclusion overrides recognition and page approval. Missing browser site
access and restricted browser pages remain unavailable; there is no workaround.

V1 supports visible, enabled native text inputs, textareas, and single-choice
HTML select controls in the main document. Native controls in accessible open
shadow roots may be processed with the same label and validation rules. Closed
shadow roots, embedded frames, custom dropdowns/comboboxes, rich-text editors,
radio buttons, checkboxes, hidden/disabled/read-only fields, and unsupported
controls remain manual. Recognition is not a promise of universal ATS support.

Always exclude passwords, login/verification codes, payment or bank details,
government identifiers, file uploads, and typed signatures. The extension never
clicks Submit, Next, agreement, or certification controls. Page approval and alias
creation cannot override these exclusions.

## Learning, matching, and approvals

The library contains one saved answer per normalized question and compatible
control/choice configuration. Include bounded question qualifiers in the identity
so similarly labeled fields do not silently share different meanings. Text and
single-choice answers are typed separately; different choice sets do not silently
replace each other.

Matching and classification are deterministic, local, and desktop-owned.
Normalize harmless formatting differences while preserving negation, numbers,
meaningful punctuation, and qualifiers. Do not infer paraphrases, translate,
invert Yes/No meanings, infer personal facts, or call an AI provider. Local rules
are conservative; an unclassified question requires approval, not a guess that
it is ordinary. The user may deliberately link equivalent wording as an alias.

| Question or event | Capture behavior | Reuse behavior |
| --- | --- | --- |
| Recognized ordinary stable fact | Save after field completion | Fill a matching empty field automatically in Autofill + capture |
| Ordinary but employer/role/location/date/application-dependent | Save after field completion | Approval for this question on the current page |
| Ordinary but reusable meaning is ambiguous | Save as review-only | Explicit approval before becoming reusable |
| Sensitive or unclassified | Hold in memory pending explicit save approval | Approval for this question on the current page |
| Different answer to a known question | Propose a correction; apply the same sensitivity gate | Preserve the saved answer until replacement approval |
| Website-prefilled or ORT-filled | Do not learn as a new answer | Leave occupied page values untouched |

Sensitive examples include disability, ethnicity, veteran status, work
authorization, sponsorship, salary, and legal declarations. These are answers
supplied by the user, not generated/inferred attestations. Sensitive and
context-dependent reuse always requires current-page approval; aliasing or
marking an ordinary answer reusable cannot bypass that requirement.

Capture text only after a direct user edit followed by leaving the field; capture
a native dropdown after a user selection. Skip empty, invalid, unfinished, and
excluded values. Do not capture every keystroke, treat a focus event as an edit,
or learn a website/ORT-generated event as user input. Completing a field does
not imply that an application was submitted or its answer accepted by an employer.

An ordinary correction is encrypted and durable in Needs review across restarts.
Approval replaces the saved answer only against its expected revision; a stale
or conflicting proposal needs review again. Rejection preserves the old answer.
Unapproved sensitive/unclassified values are memory-only and expire on page or
session end. After explicit save approval, a new entry may be persisted with its
restrictions intact. A sensitive/unclassified replacement also requires explicit
replacement approval against the saved revision before committing; it does not
enter an unapproved durable correction queue. Pending ordinary corrections are
not portable until approved into saved answers.

Approval is bound to the question, answer revision, choices, and current page.
Unchanged same-page rerenders do not repeatedly ask. Navigation, relevant identity
changes, disablement, or disconnection invalidate it. It does not authorize every
sensitive question on a site or carry to another page.

## Filling and browser popup

Fill only an empty or verified placeholder field. Never overwrite a meaningful
page value or an intervening user edit. Match a dropdown to one unique compatible
option; ambiguity stays manual. Preserve answer content and field constraints;
do not truncate, guess another choice, move focus, scroll, simulate typing, or
repeat failed writes. Verify that the site retains the value; unsupported or
reverted writes are reported as manual work.

The compact extension toolbar popup shows current-page status and pending
decisions. It provides:

- Saved-question search, **Use once**, and explicit **Link as equivalent**.
- Sensitive/context approval with the destination website and proposed answer
  visible inside the extension popup, outside the webpage DOM.
- **Undo** for an ORT-filled field only if the user/site has not changed it since
  filling. Restore its previous empty/placeholder state and suppress another
  automatic fill of that field for the current page. Keep the saved answer.
- **Pause this page**, **Never on this website**, and **Stop everywhere**.
- **Open Application answers** in the desktop for full library management.

Pause lasts for the current page. Website exclusions use exact HTTP/HTTPS origins
and persist in the encrypted desktop profile; the interface shows their scope
and permits removal. Stop everywhere sets the desktop mode Off. The popup does
not silently re-enable automation. In Capture only, fill actions explain that
Autofill + capture must be enabled in the desktop. Off performs no automatic
observation or capture. Explicit Undo can reverse a recorded current-page fill
without re-enabling automation, subject to the unchanged-field check.

Searching, using once, and creating an alias are separate actions. Use once does
not create an alias. Aliases must preserve question meaning and compatible
control/choices, and cannot downgrade sensitivity or context restrictions.
Conflicting mappings require review; no last-write-wins alias substitution.

## Ownership, retention, and portability

SQLCipher-encrypted desktop storage is the source of truth. The extension has no
durable answer database or offline filling mode. A content script receives only
answers authorized for its bound fields; explicit popup/desktop interactions
alone may search the library. Never expose the library through webpage messages.

The library is independent of the master resume, overlay workspace, and tracker.
Do not automatically promote resume facts, AI drafts, or tracker answer sets.
Finish Application, workspace reset, tracker deletion, and AI-history clearing
do not delete the reusable library. Delete all local data does.

Encrypted backup and deliberate full-data export include saved answers, aliases,
and user-created website exclusions. They exclude active modes, browser sessions,
page grants, pending corrections, and temporary captures. Legacy backups restore
an empty library; replacement restore forces Off and does not merge libraries.
Ordinary logs and diagnostics contain no questions, answers, full source URLs,
or browsing history. External backups/exports and values already disclosed to a
website remain outside local deletion.

## M5 acceptance and later work

Use the existing focused quality policy. Record the tested build, one Chrome
learn/review/fill/correct/Undo journey, an Edge smoke check, relevant deterministic
failure/security/data tests, and keyboard/screen-reader spot checks. The technical
plan specifies the cases. Do not require an exhaustive site/model/platform matrix.

Custom widgets/embedded forms, broader controls, semantic matching, answer
variants, automatic imports from other records, and automatic submission are
outside v1. Future additions need their own product decisions; submission remains
an initial non-goal. Production Store disclosures must describe the implemented
capability before release, not borrow the current capture-only privacy claims.
