# Portable backup container v1

- Extension: `.ort-backup`
- Writer format: 1.9 (reader accepts 1.0–1.9)
- Status: native export, read-only authenticated validation, restart-staged
  replace-restore, rollback, and exact safety-copy cleanup commands integrated;
  macOS Apple Silicon development implementation. Cross-platform native and
  production release qualification remain separate; synthetic hostile-input,
  retirement and size-bound regressions are ordinary automated checks.

## Planned M5 extension — not implemented

The [answer-library technical plan](../../Implementation%20Plans/Browser%20Extensions/Application_Answer_Autofill_Plan.md)
reserves schema 10/backup 1.10 on the current baseline for typed saved answers,
reviewed aliases, and user-created website exclusions. It requires new inventory/
bounded validation and content-schema mapping, historical-reader defaults to an
empty library, safe old-reader rejection, and replacement restore with automation
Off. Active modes, browser/page grants, pending corrections, and temporary values
are excluded. This is not a current writer/reader or header-format change;
format 1.9 and its existing vectors remain the implemented contract below.

## Fixed header

All integers are unsigned big-endian. The complete 76-byte header is AEAD
associated data.

| Offset | Bytes | Meaning | v1 value/policy |
| ---: | ---: | --- | --- |
| 0 | 4 | magic | `ORTB` |
| 4 | 2 | format major | `1` |
| 6 | 2 | format minor | writer `9`; reader `0`–`9` |
| 8 | 1 | KDF identifier | `1` = Argon2id v1.3 |
| 9 | 3 | reserved | zero |
| 12 | 4 | memory KiB | writer 65,536; reader 65,536–262,144 |
| 16 | 4 | iterations | writer 3; reader 3–10 |
| 20 | 4 | lanes | exactly 4 |
| 24 | 1 | salt length | 16 |
| 25 | 1 | nonce length | 24 |
| 26 | 2 | reserved | zero |
| 28 | 8 | ciphertext length | bounded before allocation/KDF |
| 36 | 16 | random salt | per backup |
| 52 | 24 | random XChaCha nonce | per backup |

The encrypted payload is deterministic JSON containing a manifest and canonical
portable profile records. Version 1.1 adds at most 100 content-free PDF render
manifests, ordered newest first, to the draft, published-resume and setting
records defined by 1.0. A render manifest contains the exact source revision,
hashes, renderer/template/font identities, bounded page/byte counts, generation
times and repeat count. It never contains PDF bytes, resume content, filesystem
paths or preview tickets. The payload manifest includes schema versions,
inventory, and SHA-256 of the canonical profile. It contains no database key,
vault reference, provider credential, native IPC secret, general diagnostics, cache, or
index.

Version 1.2 supports document schema 2. Version 1.3 adds bounded, content-free
AI operation and attempt records (at most 10,000 and 20,000), with authenticated
inventory counts and database schema 3. It excludes API keys, prompt/response
content, and the active OS-vault connection reference. Spending-cap policies and
counters are not transferred to a new credential identity by restore. Version
1.4 uses database schema 4 and adds each attempt's catalog effective date and
exact bounded pricing components so historical cost provenance survives restore.

Version 1.5 adds retained tracker records and application state. Version 1.6
removes count-based publication and AI-activity limits. Storage retains every
publication; backup never silently prunes that history. The authenticated
payload and container still have the shared 64 MiB size budget, and oversized
exports fail explicitly. Earlier readers reject the new minor version rather
than attempting an archive outside their count policy.

Version 1.7 uses database schema 6 and preserves bounded AI failure diagnostics
(error codes, HTTP/finish reasons, measured page count, and locally authored
validation rules). It excludes provider messages, prompts, response bodies,
resume/job text, and credentials. Older archives restore without these optional
diagnostics.

Version 1.8 uses database schema 7 and preserves connection source, opaque
plan identity, reasoning, monetary-cost tracking and reported retry metadata.
Legacy records default to direct API activity. Plan records carry $0 cost marked
not tracked, while missing token usage stays unknown. Plan policy and managed
Codex authorization are excluded; restore requires a fresh connection and never
inherits the old profile's enabled plan or key reference.

Version 1.9 uses portable database-content schema 8 and adds typed AI lifetime
totals and imported guardrail policies. These remain separate from removable
activity history. Restored policies stay inactive until explicit binding. The
current encrypted database is schema 9, whose tracker search projection is
derived and therefore not added to the portable schema.

Export first checks aggregate source bytes and record counts in the same database
snapshot before materializing rows. Collection counts exact serialized record
bytes with a 64 MiB budget and a 100,000-total-record safety bound, with category
bounds for settings, tracker entries, render manifests, and accounting records.
The canonical profile is serialized once into a bounded buffer; those bytes are
reused for validation/hash/encryption. Key derivation/encryption run after releasing
the desktop storage lock. Output includes header/tag overhead in addition to the
payload limit. An oversized profile fails explicitly, without pruning history.

Reader validation is ordered: fixed header length/magic, versions and reserved
bytes, KDF policy, ciphertext length/exact file length, Argon2id derivation, AEAD
authentication, bounded JSON parsing, manifest/hash/inventory validation, then
domain validation. The authenticated header version must match the encrypted
manifest; version 1.0 requires database schema 1 and no render history, while
1.1–1.2 require database schema 2, 1.3 requires database schema 3, and 1.4
requires database schema 4; 1.5–1.6 require database schema 5; 1.7 requires database schema 6; 1.8 requires database schema 7; 1.9 requires portable database-content schema 8. Wrong passphrase, ciphertext modification,
truncation, and malformed encrypted content return the same invalid-backup
category.

## Development test vector

- Passphrase: `vector passphrase` (synthetic fixture only)
- Salt: sixteen `0x11` bytes
- Nonce: twenty-four `0x22` bytes
- Created time: `2026-09-01T12:00:00Z`
- Version 1.9 SHA-256 of the complete container:
  `41fdeea304c3b00de67968489fd7021af0c36cf7d480ee7fedd6de42e9bfd5b0`
- Legacy version 1.8 SHA-256 (still read and verified):
  `06b0048880e68b58fc0d52d4ee701d446d1c13cf8fb51afc450c9ab33ccb1605`
- Legacy version 1.6 SHA-256 (still read and verified):
  `93bea2f949c31e7b89f53f698a9a1c35fc7e18a7a90f4d25add4aeb7995f6ab4`
- Legacy version 1.0 SHA-256 (still read and verified):
  `bad075c8e1369c6aa67f4b41d422826e84cde14070e43724caa063cae26e90aa`

The vector is generated and verified in the `ort-backup` unit suite. Any format
change must intentionally update the format version or explain and review a
vector change.

## Native export boundary

The desktop command creates format 1.9 only from the already-open encrypted
profile. Its generated IPC request contains bounded request metadata and the
user-entered passphrase, but no path, profile records, overwrite flag, key, or
vault reference. The request types deliberately do not implement `Debug` or
`Clone`. Rust owns and zeroizes the passphrase after the one operation; the
frontend clears both controlled passphrase fields when dispatch begins.

A native Save dialog selects a new `.ort-backup` destination. The backend converts
that selection directly to a held-directory, single-use capability and publishes
the exact encrypted bytes with the shared private-stage/no-clobber writer. The
renderer receives only cancellation or bounded format/byte/cleanup/durability
metadata. Current-profile replacement is not implemented by this command.

## Native validation boundary

The M2 desktop can select one existing `.ort-backup` through a native Open dialog.
The generated request contains only bounded request metadata and the passphrase;
it cannot carry a path, replacement flag, destination profile, or content. Rust
clears its owned passphrase after the operation. The shared file-operation lease
prevents concurrent dialog work and makes normal quit wait for completion.

The platform adapter opens the native-selected parent as a held capability,
requires a regular `.ort-backup` final entry, disables final-component symlink
following, and checks the fixed maximum length before allocation. It reads at most
one byte beyond that limit and rejects empty or oversized files and any short or
growing read. The container reader then enforces the full validation order above,
and authentication rejects changed bytes. Wrong passphrases and authenticated-
content failures remain deliberately indistinguishable.

Only a content-free authenticated summary returns to the main window: container
bytes/version, application/schema versions, creation time, and bounded draft,
published, setting, render-manifest, and AI-activity counts. No file path, filename, resume
content, setting value, hash, passphrase, or native error crosses the command
response. Validation does not open, close, copy, or replace the active profile.

## Restart-staged replacement boundary

Replace-restore is a separate, explicitly confirmed command. Its generated
request contains only the passphrase and exact phrase `REPLACE SAVED PROFILE`;
paths, merge options, profile identifiers and destinations are rejected. The
selected final entry passes through the same bounded, no-follow native input
adapter and the same non-oracular authentication behavior as validation.

Before any replacement, ORT creates a new private sibling profile with a new
database key and vault identity, imports the portable records transactionally,
and verifies SQLCipher integrity. Only then does it durably create a fixed,
content-free restart marker. The open active database is not swapped in place.
At the next startup, after acquiring the exclusive stable sibling profile lock,
fixed-name directory renames retain the previous encrypted
profile as a local safety copy and promote the staged profile. The marker remains
until the promoted profile opens successfully. Startup recognizes interruptions
before, between, and after the renames and either completes promotion or restores
the untouched old directory.

The response exposes only cancellation or the facts that restart is required and
a safety copy will be retained. The current profile, staged/safety paths, vault
references, file selection and record contents never cross to the renderer.

## Retained safety-copy management

A content-free status command reports three booleans only: whether the fixed
safety copy exists, a replacement/rollback awaits restart, or confirmed cleanup
is pending. It does not return a path, profile or vault identity, timestamps,
sizes, record counts, or content.

Rollback requires the exact phrase `ROLL BACK SAVED PROFILE`. It verifies the
retained encrypted profile and creates a same-key encrypted checkpoint in the
fixed staging slot before committing a `rollback_ready` marker. At startup, the
redundant old safety directory is removed without deleting its still-needed key,
the current profile becomes the new safety copy, and the verified checkpoint is
promoted through the existing restart state machine. Neither profile is silently
discarded.

Permanent cleanup requires `DELETE SAFETY COPY`. The exact safety directory is
first renamed to a fixed deletion-pending slot. A durable inventory records its
provider credential addresses before provider credentials, the database key, and
known profile files are removed. The inventory remains usable after the database
key is gone and until profile-file cleanup commits. Provider credentials remain
available while the safety copy is retained for rollback. Startup
repeats vault deletion idempotently and completes an interrupted cleanup before
opening the active profile. User-controlled exports/backups and the active
profile are outside this boundary. Symlinks, unexpected entry types, overlapping
recovery state, and unverifiable safety profiles fail closed.

Delete-all uses the same credential retirement service for active, staged, safety,
and deletion-pending profiles. A provider-vault failure leaves its durable intent
and inventory retryable on restart; malformed rosters cannot silently discard
secret references. The process ownership lock remains held while a fresh store
is initialized after deletion.
