# ADR 0006: Portable backup cryptographic container

- Status: development prototype accepted; format freeze gated
- Target milestone: M1

## Decision

Portable backups use canonical records rather than a copied SQLCipher database.
Argon2id v1.3 derives a 256-bit key from a user-owned passphrase and random
128-bit salt. XChaCha20-Poly1305 encrypts/authenticates the complete payload and
authenticates the bounded fixed header as associated data. The implementation
pins `argon2 0.6.0`, `chacha20poly1305 0.11.0`, and `sha2 0.11.0` with secret
clearing features where provided.

## Consequences

Restoring creates a fresh local SQLCipher key and vault identity. Device-bound
keys and credentials never enter the container. The prototype remains disabled
in the UI until Windows/macOS cross-restore, file atomicity, hostile-input/fuzz,
memory-pressure, cancellation, and user-recovery behavior pass.

