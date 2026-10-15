# Provider catalog

`direct-v1.json` is the exact-byte, versioned direct-provider catalog bundled by
M3. `direct-v1.sig` is its Ed25519 signature and `direct-v1.pub` is the dedicated
catalog trust key. This key is intentionally independent from future updater
trust. Runtime verification also enforces chronology, minimum app version,
expiry, rollback, supported-operation, price-dimension, and emergency-disable
rules before an entry is selectable.

The catalog files are forced to LF in `.gitattributes` because the signature
covers the JSON's exact bytes. The deterministic checkout test exercises real
Git conversion with Windows-style `core.autocrlf=true` so a platform checkout
cannot silently invalidate the bundled signature.

The current development catalog contains three entries with legacy Balanced metadata used for
the M3 adapter boundary and a Gemini Economy entry for `gemini-3.5-flash-lite`.
The current UI selects these explicit model IDs; tier names are legacy catalog/migration metadata. Other tier/model candidates are not automatically available. A
provider model-list response may hide these entries; it cannot add an untrusted
model.

The `2026-09-23.1` catalog retains the September 15 Balanced entries and adds
Gemini Economy using Google's September 23 model and standard pricing pages.
It represents standard synchronous text-token prices only: provider batch,
long-context, storage, tools, taxes, credits, promotions, and account-specific
terms are not flattened into these rates. Its development signing private key
was ephemeral and discarded; the protected catalog publishing workflow and
durable release key remain M7 distribution work.
