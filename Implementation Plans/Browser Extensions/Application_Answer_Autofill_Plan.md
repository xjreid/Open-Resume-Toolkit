# Application-answer autofill implementation plan

## Status and authority

- Status: approved design, not implemented; documentation-only addition on October 19, 2026.
- Milestone: M5 Chrome/Edge development functionality on macOS Apple Silicon.
- Owners: browser/IPC, desktop, domain/contracts, and storage maintainers.
- Product authority: [Application-answer autofill](../../Product%20Plans/Application_Answer_Autofill.md).
- Related authorities: [extension communication](../../Product%20Plans/Desktop_Extension_Communication.md), [data model](../../Product%20Plans/Local_Data_and_Document_Model.md), [configuration](../../Product%20Plans/Configuration_Limits_and_Defaults.md), and [quality policy](../../Product%20Plans/Quality_Accessibility_and_Verification.md).

No code, generated contract, migration, manifest, or released privacy claim is
changed by this planning task. Schema 9, backup 1.9, capture protocol v1, and the
current capture-only UI remain implemented facts until future implementation.
M7 retains signing/notarization, production host/vault identity, and Store gates.

## Component boundaries and data flow

Reuse the shared TypeScript extension and existing authenticated native client.
Add separate feature modules for the foreground monitor, field adapters, popup,
and service-worker coordination; do not enlarge rectangle-capture logic into a
general form engine. Existing capture remains independently authorized.

The future flow is:

1. The desktop loads the persisted mode and exclusions, verifies an available
   profile and compatible authenticated browser connection, then authorizes the
   foreground page with a short-lived, revocable lease.
2. The service worker validates browser/window/tab/document identity and site
   access, injects the packaged isolated-world monitor, and handles lifecycle and
   native traffic. It does not inspect the DOM itself.
3. The monitor discovers eligible controls and bounded associated question/choice
   descriptors. Recognized application forms proceed; ambiguous pages request
   explicit page approval through the popup.
4. Batched desktop lookup normalizes/classifies descriptors and returns per-field
   outcomes: no match, manual/unsupported, approval required, or authorized answer.
5. Immediately before a write, the monitor rechecks lease/config/answer revisions,
   field identity, choices, visibility, empty state, and intervening user edits.
   It writes through a field adapter and verifies retention.
6. Direct user edits commit after blur or native selection. The desktop saves an
   eligible new ordinary answer, creates a revisioned ordinary correction, or
   holds a sensitive/unclassified candidate in memory pending explicit approval.
7. Popup decisions return through the same authenticated service. Library writes
   invalidate dependent results; stop/navigation/disconnect revoke monitors and
   pending actions. None of this creates an overlay workspace or tracker entry.

The source URL used for local document checks is not retained as browsing
history. Keep only the minimum current-page identity in memory. Persist exact
origins only when the user creates a website exclusion.

## Desktop library and domain contracts

Introduce a dedicated answer-library application service using `ort-domain`,
`ort-application`, and `ort-storage` ownership conventions. Tauri main-window
commands and authenticated browser handlers call the same policy/use cases.
Do not place the collection in generic settings, temporary `approved_answers`,
or tracker snapshots.

Define typed concepts before generating TypeScript/schema contracts:

| Concept | Required semantics |
| --- | --- |
| Saved answer | Opaque ID; original question/qualifiers; normalized match identity/version; text or single-choice kind; compatible choices; literal answer; sensitivity/context/reuse restrictions; timestamps; optimistic revision |
| Alias | Explicit normalized equivalent wording/qualifiers targeting one entry with compatible control/choices; uniqueness/conflict checks |
| Ordinary correction | Opaque ID; entry ID/base revision; proposed answer and bounded question descriptor; timestamps; pending/approve/reject outcome |
| Preferences | Off/Capture only/Autofill + capture, consent version, configuration revision, exact-origin exclusions |
| Page authorization | In-memory connection/browser/tab/document/navigation/field binding, configuration and answer revisions, expiry, and per-question approval scope |
| Fill receipt | In-memory current-page field binding, expected written-value fingerprint, prior empty/placeholder state, and Undo suppression; never a history ledger |

Use indexed normalized lookup and paginated list metadata/previews. Full values
load on selection. Do not return the entire library to the monitor. Entry edits,
deletion, alias changes, mode changes, and exclusion changes invalidate dependent
results. Delete cascades to aliases/corrections. A stale editor or correction
cannot silently replace a newer revision. Identical capture submissions are
idempotent and do not flood the review queue.

One desktop-owned normalizer/classifier serves captured and manually created
entries. Match text plus meaningful qualifiers and compatible control/choice
configuration. Preserve negation, numbers, and meaningful punctuation. Separate
question classification from permission to capture and permission to reuse.
Recognized stable ordinary facts may auto-reuse; ordinary context-dependent
answers require page approval; ambiguous reuse starts review-only; sensitive or
unclassified questions need explicit save approval and page-specific reuse.
Hard exclusions are never overridable by aliases or page approval.

Add **Application answers** after Tracker and before Settings in the existing
shell. Use shared mode/status/search/list/form/dialog patterns and Open Folio
tokens. Include saved entries, answer editing, manual entry creation, aliases,
reuse policy, exclusions, and Needs review. Preserve dirty-editor/close guards
and optimistic concurrency. The overlay's AI Answers tab stays separate.

## Native protocol and authorization

Add an explicitly negotiated protocol v2 autofill capability. Preserve protocol
v1 capture behavior and tests; an old desktop/host/extension reports autofill
unavailable without enabling a broader fallback. M5 may use the existing
explicit development transport; its current-user trust limitation remains true.
The production transport remains disabled until M7 identity/vault gates pass.

Specify generated strict request/response families for:

- Capability/configuration state, foreground monitor authorization/renewal, and revocation.
- Bounded batched question lookup with independently bound per-field outcomes.
- Completed-answer capture and correction proposal, with idempotency/base revision.
- Explicit save/reuse approval, Use once, alias linking, fill result, and Undo state.
- Popup-authorized saved-question search/detail and desktop library CRUD/review.

Do not add fields to a strict v1 envelope and assume compatibility. A negotiated
v2 peer still accepts the documented v1 capture contract. Roll out desktop/host
support before the extension; retain compatible capture during Store skew.

Content scripts may submit bounded current-field descriptors and receive only
authorized field answers. Only an extension-owned popup interaction or authorized
main window may search/select library records. Validate sender origin, tab,
main frame, document, navigation generation, lease, schema, size, freshness,
replay/idempotency, and revisions at the relevant boundaries. No webpage
`postMessage` bridge, arbitrary record enumeration, file path, provider request,
command execution, or secret-reading capability is introduced.

Reuse the existing native-frame, queue, text, and backup safety ceilings and
centralize schema-specific monitor limits in the configuration register. Bound
both encoded lookup requests and responses; split batches at field boundaries
or return a manual/oversized result, never truncate a descriptor/answer/choice
set. Heartbeats contain no page/question/answer content. Observation never
launches the desktop repeatedly or queues content across worker restarts.

Switching Off or excluding a site revokes active leases and queued work. Show
stopping until monitors acknowledge or leases expire. A mode retained across
restarts is intent, not a reusable browser authorization. Service-worker restart
reconstructs the current policy from the desktop and revalidates the page before
doing work; it does not restore an offline answer cache.

## Monitor, field adapters, and popup

Use packaged isolated-world code under existing host-access controls; granting
site access alone never starts observation. No startup reader or new blanket
permission is implied. A toolbar action/popup is the new approved UI surface.
Refer to the official [content-script model](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts)
and [MV3 lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle).

- Do one bounded discovery pass, then debounce `MutationObserver` work over
  changed candidate subtrees. Yield bounded work; avoid whole-page polling,
  full-text scraping, and repeated lookups for an unchanged descriptor/revision.
- Read associated labels, ARIA label references, nearby fieldset legends, bounded
  instructions, and native choices. Missing/ambiguous labels or application-form
  recognition produce manual/page-approval states rather than guessed matches.
- Restrict v1 to supported native controls in the main document, including
  accessible open shadow trees under the same bounds. Never inspect hidden
  values or attempt custom widgets, embedded frames, or closed roots.
- Correlate trusted user editing/selection activity with field completion;
  ignore programmatic events and prefilled values. Never treat focus alone,
  ORT's own writes, or arbitrary page events as a new user answer.
- Use the native value setter and input/change events, verify the resulting
  value and unchanged field identity, and stop on reversion/rejection. Do not
  move focus, scroll, simulate typing, dispatch trusted events, use a main-world
  bypass, or try arbitrary click/keyboard choreography. Synthetic events are
  [not trusted user events](https://developer.mozilla.org/en-US/docs/Web/API/Event/isTrusted).
- A dropdown needs one unique compatible option. A filled/edited/focused field,
  changed choices, late response, conflicting candidate, or answer exceeding a
  field constraint stays manual. Do not truncate or repeatedly fight the site.
- Keep Undo receipts only for the current page. Revalidate before reverting;
  preserve intervening user/site changes and suppress refill after Undo.

The popup presents status, current-page decisions, Use once versus explicit
alias linking, Undo, page pause, exact-origin exclusion, global stop, and a link
to Application answers. The desktop owns mode/library state. Sensitive answer
previews stay in extension-owned UI until approved; they are never injected as
suggestions into page DOM. Capture only cannot write fields. Explicit Undo may
reverse a still-valid fill while paused/Off without enabling a monitor.

Foreground changes, navigation (including same-document URL transitions),
disconnect, permission loss, expiry, or relevant form changes cancel pending
operations. Unchanged rerenders may retain the same per-question page approval;
new questions/revisions cannot inherit it. Cancellation removes observers and
temporary values without changing already entered page content.

## Storage, backup, export, and deletion

Plan the next ordered migration as **schema 10** on the current schema-9 baseline.
Create dedicated saved-answer, alias, and ordinary-correction records with
indexes, validation, revisions, and deletion relationships. Keep preferences
separate from answer records. Sensitive/unclassified candidates are never
serialized, journaled as pending content, or logged before explicit save approval.

Plan **backup 1.10** and its content-schema mapping, a typed
`PortableAnswerLibraryV1`, inventory counts, and bounded collection/restore
validation. Do not put new canonical rows into backup 1.9. New readers default
the library to empty for historical formats; old readers reject 1.10 safely.
Preserve legacy readers/vectors and the current authenticated container design.

Include saved entries, aliases, and user-created origin exclusions in encrypted
backup and deliberate full-data export. Exclude active mode/consent grants,
browser identities, current-page approvals, Undo receipts, pending corrections,
and temporary values. Replacement restore creates the new library with mode Off;
no merge, cross-application answer promotion, or operational permission transfer.
Rebuild derived lookup indexes from canonical data.

Delete all local data removes the library, correction queue, aliases, mode, and
exclusions under existing profile-retirement rules. Ordinary workspace finish,
tracker deletion, or AI activity clearing does not. Entry/profile deletion also
revokes outstanding browser results. External backups/exports and data already
read by a website remain outside ORT deletion. No extension-side durable answer
storage, content logging, full-URL history, or diagnostics containing answers.

## Implementation sequence and completion criteria

Each step is future implementation, not work authorized by this document edit:

1. **Domain/storage/contracts:** classifier and typed records; schema/backup
   transitions; indexed lookup, revisions, portable/deletion rules; generated
   validators and strict contracts.
2. **Desktop library:** route, three modes and consent, status, CRUD, aliases,
   exclusions, Needs review, accessible edit/review and close behavior.
3. **Authorization/protocol:** v2 negotiation and narrow browser use cases,
   foreground leases, revocation, version skew, existing v1 capture preservation.
4. **Capture-only monitor:** application-page gate, supported controls, efficient
   observation, genuine edit completion, capture policy, correction flow.
5. **Autofill/popup:** empty-field adapters, verification, page approvals,
   Use once/linking, Undo, stop controls, manual fallback, popup packaging.
6. **Chrome/Edge qualification:** representative journey, focused negative/data
   cases, accessibility, and build/limitations record under the existing policy.

Minimum future checks cover normalization/negation/qualifiers, incompatible or
duplicate choices, alias conflicts, each capture/reuse category, direct edits
versus ORT/site events, field constraints, framework-controlled inputs,
rerenders, rejected writes, occupied values, Undo/user-edit races, mode changes,
foreground switching, navigation, worker restart, disconnection, invalid origin/
lease/replay/version, stale edits/corrections, migration, legacy/new backup,
restore-Off, deletion, and redaction. Use synthetic forms and controllable clocks;
do not add broad site matrices, elapsed-time campaigns, or paid AI testing.

Record one Chrome learn → review → fill → correct → Undo journey, an Edge smoke
check, and keyboard/screen-reader checks for the new route/popup. Assert no
provider calls, automatic submission, unrelated/background collection, or
extension persistence. Keep M5 open until working development integration passes;
signing, production-host identity, Store-installed checks, updated privacy/
permission disclosures, and release qualification remain M7 gates.
