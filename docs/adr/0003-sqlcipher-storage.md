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

## Consequences

The development core now includes schema v1, bounded structured resume records,
optimistic revisions, immutable published snapshots, settings, diagnostics
schema, and encrypted same-device checkpoints. Runtime persistence remains
gated until database/WAL, native vault, corruption, migration, recovery, and
signed Windows/macOS platform suites pass. The portable cross-device backup
container remains separate work and must not reuse the device-bound database
key.
