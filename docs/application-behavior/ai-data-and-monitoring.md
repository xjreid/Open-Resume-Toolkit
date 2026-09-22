# AI data and monitoring behavior

## Data scope and key selection

The Data tab displays retained local AI operation and provider-attempt
activity. The **Choose view** control selects either:

- **All keys**, which aggregates every retained activity record; or
- one saved or removed key, identified by name, provider, preset/model, logo,
  and creation date.

Successfully removed keys remain selectable while their activity is retained.
Keys whose removal failed are still My Keys entries and are not treated as
successfully removed. After a removed key's data is permanently deleted, its
tombstone is forgotten and it no longer appears in the selector.

All keys is computed from the same retained operations as the per-key views. It
is not a separate total. Therefore any activity deletion that changes a key's
graph also changes All keys for the same period.

## Graph behavior

The graph supports two Y-axis metrics:

- **Price**: recorded estimated cost, separated by currency when multiple
  currencies exist;
- **Tokens**: total estimated tokens, composed of input, output, cached input,
  cache-write, and reasoning categories.

The X-axis timeframe choices are Week, Month, Year, and All time:

- Week shows the last seven calendar days, including today and quiet days.
- Month shows the last 30 calendar days, including today and quiet days.
- Year shows the last 12 calendar months, including the current month and
  months with no activity.
- All time starts at the earliest retained month. When it exceeds 24 monthly
  points, the display aggregates into yearly points.

Point spacing follows calendar time rather than compressing quiet periods.
Hovering, clicking, or keyboard-focusing a point opens a popup anchored over
that point. It shows the exact selected estimate, attempts, partial-data state,
and the token-category breakdown when Tokens is selected.

Missing or uncertain provider usage is not treated as zero. Partial totals,
unknown attempts, and unresolved reserved exposure are surfaced separately.
Provider billing remains the source of truth; the graph is a local estimate.

## Settings actions

Settings is a separate panel below the graph. Its actions do not inherit the
currently displayed graph key or timeframe; each workflow asks for its own
scope.

### Export activity

Export is a two-step workflow:

1. choose one or more keys, or choose All keys;
2. select one or more months that currently contain activity.

Each key choice toggles independently and can be deselected. Months are the
union of active months across the selected keys, with attempt counts combined.
If All keys is selected, it overrides any individual selections and the scope
is every key. The resulting aggregate JSON contains only the effective scope
and selected months. It is saved through a native Save dialog and is
unencrypted after export. Export does not mutate activity, totals, caps, or key
state.

### Clear activity

Clear activity uses the same multi-select key workflow and active-month union.
Selections can be toggled off; All keys overrides individual selections. It
permanently removes matching completed activity from the graph for every key
in the effective scope.

- Clearing one or more keys reduces those key views and the All keys
  aggregate.
- Clearing All keys reduces every matching key in the selected months.
- Matching active requests block the clear operation rather than being partly
  removed.
- Lifetime spend, general spend, and spending-cap progress are preserved.

### Activity retention

Retention choices are 30 days, 90 days, one year, or Retain until cleared.
Applying a finite policy requires confirmation because older activity is
permanently deleted immediately. The saved policy is also applied when
retention settings load in a future application session.

Retention affects every key and therefore updates All keys. It does not reset
lifetime spend or cap progress. Retain until cleared does not delete activity.

### Delete removed key data

This action is available only when at least one successfully removed key still
has a retained key tombstone. Its two-step workflow is:

1. select one or more removed keys;
2. confirm permanent deletion after reading the irreversible-action warning.

The UI lists only removed keys, and the native command independently rejects
any credential that is still present in My Keys or has incomplete removal
cleanup.

On success:

- all retained activity for each selected removed key is deleted;
- the activity disappears from the All keys aggregate;
- the removed-key tombstone is forgotten;
- the key can no longer be selected in Data;
- lifetime and general spending totals on My Keys remain unchanged.

The action cannot be undone. If the user was viewing a deleted key, the Data
view returns to All keys.

## Data-effects matrix

| Action | My Keys card | Provider secret | Per-key Data | All keys Data | Lifetime/general totals | Cap progress |
| --- | --- | --- | --- | --- | --- | --- |
| Pause key | Remains, greyed | Kept | Kept | Kept | Kept | Kept |
| Successful key removal | Disappears | Removed | Kept and selectable as removed | Kept | Kept | Per-key policy removed; general accounting kept |
| Failed key removal | Remains with failure state | Cleanup outcome incomplete | Kept | Kept | Kept | Key unusable until retry |
| Clear selected months | Unchanged | Unchanged | Matching completed activity removed | Recomputed without it | Kept | Kept |
| Apply finite retention | Unchanged | Unchanged | Older completed activity removed | Recomputed without it | Kept | Kept |
| Delete removed key data | Already absent | Already removed | All retained activity and selector entry removed | Recomputed without it | Kept | Unchanged |
| Restart cap | Unchanged | Unchanged | Kept | Kept | Lifetime kept | Usage baseline reset |
| Remove limit | Unchanged | Unchanged | Kept | Kept | Kept | Limit becomes Unlimited |
| Export JSON | Unchanged | Unchanged | Unchanged | Unchanged | Unchanged | Unchanged |
