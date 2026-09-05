# ADR 0003: SQLCipher encrypted storage boundary

- Status: development implementation accepted; platform activation gated
- Target milestone: M1

## Decision

Persistent user records use `rusqlite 0.40.2` with the
`bundled-sqlcipher-vendored-openssl` feature. A random 256-bit key is held by
the operating-system vault through `keyring 4.2.0`; there is no plaintext
fallback. Cipher parameters are explicit and authenticated/structural integrity
checks run at open. The exact dependency review is recorded in
`../dependencies/M1-security-dependencies.md`.

The 2026-09-05 macOS creation repair uses the already locked
`security-framework 3.7.0` safe add-only API in the same User-domain Keychain.
This closes the check-then-upsert race between independent vault instances;
load/delete continue through `keyring`. It does not change or qualify native
access-control policy. Other platforms retain their existing adapter pending
later qualification.

## Consequences

The development core now includes schema v1, bounded structured resume records,
optimistic revisions, immutable published snapshots, settings, diagnostics
schema, and encrypted same-device checkpoints. Runtime persistence remains
gated until database/WAL, native vault, corruption, migration, recovery, and
signed Windows/macOS platform suites pass. The portable cross-device backup
container remains separate work and must not reuse the device-bound database
key.

## Local macOS development qualification (Step 4)

The local desktop's intended database-key access is its own signed application
identity and explicit macOS per-item user authorization. The adapter uses the
native Keychain creation policy; it does not add a broad trusted-application
list or automatically authorize another process. The native host has no database
secret port and remains gated. The locally self-signed certificate and
`com.openresumetoolkit.dev` designated requirement were held stable across the
qualified rebuilds and moves. A distinct helper signed by the same certificate
could find installed-item metadata but could not read the key with UI disabled.
This is observed native caller isolation, not a universal defense against
malware in an unlocked user session.

The real standard account separately passed restore/restart, file and key
namespace denial, locked-Keychain refusal/nonmutation, and deletion/rekey with
developer-profile preservation. Signed disposable native tests cover actual
process termination, migration interruption, missing/wrong keys, ciphertext
corruption and bounded HFS+ ENOSPC. See
`../../evidence/0.0.0-dev/m1-macos-storage-qualification.md` for artifact hashes,
user-observed versus automated evidence, and remaining CI/distribution limits.
