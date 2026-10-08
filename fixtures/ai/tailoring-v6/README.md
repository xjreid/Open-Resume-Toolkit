# Resume tailoring v6 evaluation corpus

All identities and facts are synthetic. Each JSON contains a master source,
job description and source-cited candidate. Rust contract and pipeline tests
consume these fixtures without contacting providers.

Cases cover student projects, experienced reporting work with exact metrics,
sparse career-change evidence, an oversized master, misleading job instructions,
an unsupported requested metric, and ambiguous skill text. The candidate is a
contract fixture, not a claim that a model passed editorial evaluation. In
particular, the oversized candidate intentionally fails the one-page target.

## Opt-in, budgeted live comparison

Do not run paid evaluation as part of ordinary tests or builds. To explicitly
compare model quality:

1. Choose the same configured provider/model, document style and synthetic cases
   for both the pre-change app revision and this revision, using disposable local
   profiles. Set an explicit general spending cap in both profiles before calls.
2. Import each case's source, review and publish it, then paste its job description
   into the overlay. Run the old version once and this version once per selected
   case. The new version uses 2–4 calls per case; the provider's current catalog
   and app reservations determine the monetary bound. Stop when the chosen cap
   rejects a reservation. Do not increase it automatically.
3. Record version, model, style, case, actual calls/cost, rendered page count,
   outputs and reviewer notes locally. Do not use personal applicant data.
4. Review both outputs against the same rubric below. Score editorial dimensions
   0 (poor), 1 (mixed), or 2 (strong); preserve notes rather than collapsing factual
   failures into an average score.

| Dimension | Acceptance |
| --- | --- |
| Factual support | Every claim is explicit in this master; zero invented tools, scope, metrics or qualifications. Any unsupported claim is a hard failure. |
| Protected facts | Names, titles, dates, exact metrics and links remain accurate. |
| Role relevance | Strong direct evidence leads; job requirements are never treated as applicant evidence. |
| Distinct value | Weak/redundant material is omitted even when space remains. |
| Concision | Specific technical evidence survives tightening; bullets avoid filler and stacked clauses. |
| Structure | Ordering is intentional, retained identities stay in their sections, and ambiguous skills stay intact. |
| Page fit | Exactly one page with the original template settings. |

Also exercise Refine with AI by asking to restore an omitted entry/list item,
then making a focused description correction. Check preservation of unrelated
reviewed edits, visible change summaries, and minimal additional page-fit cuts.

No live model-quality result is implied by the offline contract tests. Record
completed comparisons separately with their model, date, budget and limitations.
