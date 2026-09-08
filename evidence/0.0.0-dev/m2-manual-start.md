# M2 manual-start checkpoint

Baseline: M1-qualified commit `65518eb9d434a5ff6158810000b8798261d9227c`.
Status: initial Step 5 subsection in progress; not complete or natively qualified.

An untouched empty workspace now presents Build from scratch and an explicitly
unavailable Import an existing resume choice. Optional starting profiles cover
Student/Recent Graduate, Experienced Professional, Technical, Academic/Research,
Sales/Marketing, and Custom, plus a neutral No preference default. Profiles add
only empty suggested section headings to the one current document. They do not
create another resume, add example facts, or restrict later editing. Building
focuses Full name and enters the existing edit/autosave path; selecting a profile
alone does not save. Save draft is disabled until the user starts. Backup recovery
remains available before creating a first draft.

The editor's add-section selector offers the product plan's suggested sections.
The starting screen uses the existing light application palette and stacks at
narrow widths. Resume export styles were unchanged by this manual-start slice;
subsequent contract/renderer work is recorded in `m2-style-foundation.md`.

Tests cover profile validity, independent IDs, canonical section order, absent
example facts, the live choice/build/focus transition, inaccessible import,
existing backup-before-first-draft regressions, and semantic accessibility of
the start and loaded editor. A live regression caught a duplicate React key
between the new start panel and existing PDF panel; distinct keys fix the stale
start panel after the build action.

Validation: the full `CI=true just check` passed, including 74 desktop tests,
TypeScript, workspace Rust tests/Clippy, contracts, formatting, security and
license checks (`target/m2-start-canonical.log`). No dependency or schema changes
were introduced by this frontend checkpoint.

Structured dates and stable ordered links still require High reasoning for
shared schema/contract and compatibility work. The subsequent High export-style
foundation is locally implemented; see `m2-style-foundation.md`. Do not change generated files directly or enable the import path.
Document-centered editing, full entry details, the three qualified export styles,
historical regeneration, and native UI qualification remain outstanding.

The installed application remains the M1-qualified artifact. Current frontend
changes are uncommitted and are not represented as installed/native evidence.
