# AI keys and spending behavior

## My Keys layout

The AI workspace has two tabs: **My Keys** and **Data**. My Keys contains:

1. an API keys panel with an **Active key** bucket and an **All keys** bucket;
2. a separate **General spending** panel for the shared all-key limit.

Only one key can be active. A saved, unpaused key in All keys is available but
is not used for ordinary AI requests until it is moved into Active key.

## Adding a key

The plus button opens a modal popup. Clicking outside, pressing Escape, or
choosing Cancel closes the popup and discards its unsaved fields.

The workflow is:

1. Optionally enter a name. Before provider selection the placeholder is
   `Enter a name`; after selection it becomes `OpenAI key`, `Anthropic key`, or
   `Gemini key`.
2. Select one provider: OpenAI, Anthropic, or Gemini. Clicking the selected
   provider again deselects it.
3. Enter the API key. The Show/Hide control changes only its visual masking.
4. Save. Save is disabled until a provider is selected and the key field is
   nonblank. A name is not required.

If the name is blank, the displayed name defaults to the provider name plus
`key`. Names are trimmed and limited to valid, non-control text. The provider
is bound to the credential when it is saved and cannot later be changed.

The secret is stored in the operating-system credential vault. The encrypted
profile stores non-secret metadata such as credential identity, provider,
name, preset, creation time, and state. The API key text must never be rendered
back into the interface or stored in the activity database.

A newly added key appears in All keys. Adding a key does not make it active and
does not replace the current active key.

## Key identity and presentation

Each key card displays:

- provider logo and provider name;
- editable key name;
- selected preset and resolved model;
- lifetime estimated spend;
- current limit exposure, limit, percentage, and progress bar;
- a three-dot action menu.

The key name saves when the input loses focus. No separate checkmark or cancel
button is required. A persisted creation timestamp identifies and sorts keys;
the old user-visible sequential key number is not used. All keys are sorted by
creation time, with credential identity as a stable tie-breaker.

## Active key movement

Keys move vertically between the two buckets:

- Drag an unpaused key from All keys into Active key to activate it.
- Drag the active key back into All keys to leave no active key.
- Dropping a different key into an occupied Active key bucket activates the new
  key; the previous active key returns to All keys.
- A key card can be dragged from its non-interactive surface. Inputs, selects,
  links, and action buttons retain their normal interaction.
- With keyboard focus on an eligible card, Enter or Space performs the same
  bucket move.

Dragging is bounded to the key-bucket region. Paused keys and keys with failed
removal cleanup cannot be activated or dragged.

## Key menu actions and states

The three-dot menu dismisses after an action, outside click, focus leaving the
menu, or Escape.

### Test key

Testing first shows a review popup with the selected key, model, and
conservative reservation. The provider call occurs only after confirmation.
The test creates ordinary monitoring/accounting activity. A paused key can be
explicitly tested without unpausing or activating it. A cleanup-failed key
cannot be tested.

### Pause and unpause

Pausing makes a key unavailable for ordinary requests and greys its card. If it
was active, the Active key bucket becomes empty; another key is never selected
automatically. Unpausing returns the key to an available state but does not
make it active.

### Remove

Removal always requires confirmation.

- On success, the provider credential and its per-key cap configuration are
  removed, the key disappears from My Keys, and no replacement active key is
  selected. Historical activity remains in Data under a removed-key entry.
- If secure cleanup cannot finish, the key remains visible in My Keys, paused
  and unusable, with a **Removal failed** state. The menu offers **Retry
  removal**. Cleanup is not silently retried in the background.
- A failed state does not prove whether every individual cleanup step did or
  did not happen. Retrying is idempotent and is the supported completion path.

Removing a key is intentionally different from deleting its historical Data.
See [AI data and monitoring](ai-data-and-monitoring.md#delete-removed-key-data).

## Presets and models

Each key owns its preset selection. The menu presents preset and resolved model
together, for example `Balanced: <model>`. A preset is selectable only when the
signed local model catalog has an enabled entry for that provider and preset.
Unavailable presets remain visible as unavailable or coming soon. Changing one
key does not change any other key or the active-key selection.

## Per-key spending

Each key has one unified all-time spending limit rather than separate weekly,
monthly, and yearly limits.

- New keys begin at **Unlimited**. The progress bar is empty and reads zero
  percent when unlimited.
- Entering a positive amount creates or changes the limit. Values support up to
  six decimal places in the displayed currency.
- Exposure is counted spend plus reserved and unresolved spend. Reservations
  keep concurrent or uncertain requests from bypassing the cap.
- **Restart cap** establishes a new zero cap-usage baseline. It does not reduce
  lifetime spend and does not delete Data activity.
- **Remove limit** returns the key to Unlimited. It does not reduce lifetime
  spend and does not delete Data activity.
- **Total spent** is the durable lifetime estimate for that credential. It is
  independent of graph retention and cap resets.

When a key reaches its limit, new AI work using that key is refused before it
can exceed the available reservation.

## General spending

General spending uses the same visual and editing behavior as a per-key limit,
but applies across all keys.

- It starts Unlimited.
- Its exposure includes usage reserved or counted across all key identities.
- Restarting or removing it does not change lifetime totals or Data activity.
- A request must fit both the selected key's limit and the general limit. If
  either limit blocks the reservation, the request does not proceed.
- Removing a key or clearing graph activity does not reduce the general
  lifetime total.

