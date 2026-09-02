# M1 security dependency review

- Reviewed: 2026-09-01; native logging configuration amended 2026-09-02
- Scope: encrypted local database and operating-system database-key storage
- Release state: approved for development; Windows/macOS platform proof still required

## Selected components

| Boundary | Pinned dependency | Configuration | Reason |
| --- | --- | --- | --- |
| Encrypted SQLite | `rusqlite 0.40.2` | `backup`, `bundled-sqlcipher-vendored-openssl` | Maintained Rust API with a reproducible SQLCipher/OpenSSL source build on Windows and macOS |
| OS credential vault | `keyring 4.2.0` | default platform stores | Uses macOS Keychain and Windows Credential Manager without exposing either API to the webview |
| Random keys | `getrandom 0.4.3` | OS random source | Generates a local 256-bit database key without command-line or environment transport |
| Secret clearing | `zeroize 1.9.0` | key wrapper and temporary encodings | Clears owned key buffers and temporary raw-key strings on drop/use |
| Stable identifiers | `uuid 1.24.1` | UUIDv7 | Time-sortable, non-semantic identifiers for install, profile, document, and record identities |
| Backup KDF | `argon2 0.6.0` | Argon2id v1.3; `alloc`, `zeroize` | Memory-hard derivation with explicit format parameters |
| Backup AEAD | `chacha20poly1305 0.11.0` | XChaCha20-Poly1305; `alloc`, `zeroize` | Authenticated payload encryption with a 192-bit random nonce |
| Backup hashes | `sha2 0.11.0` | SHA-256 | Canonical content inventory and published format-vector verification |

The lockfile is the final transitive version authority. Security updates require a
new review, full storage suite, cross-platform build, and encrypted-format
compatibility check before merge.

## Database format decisions

The database is keyed before schema access. It explicitly requests SQLCipher
compatibility level 4, 4096-byte pages, 256,000 KDF iterations,
PBKDF2-HMAC-SHA-512, HMAC-SHA-512, per-page HMAC, a zero-byte plaintext header,
and allocation memory wiping. Foreign keys, secure deletion, untrusted schema
mode, full synchronous writes, a bounded busy timeout, and encrypted WAL mode
are then enabled. Startup runs SQLCipher authentication plus SQLite integrity
checks.

No plaintext SQLite fallback exists. A missing, corrupt, or inaccessible vault
key stops profile opening and never creates a replacement database or identity.

## SQLCipher Windows logging mitigation (2026-09-02)

The lockfile selects `libsqlite3-sys 0.38.2`, whose bundled SQLCipher is 4.14.0.
Its Windows diagnostic logger allocates through SQLite while reporting a failed
memory lock. With allocation memory security enabled this can recursively enter
the same logger. This matches the new Windows encrypted-profile startup failure;
SQLCipher's [4.18.0 changelog](https://github.com/sqlcipher/sqlcipher/blob/master/CHANGELOG.md)
records an allocation-related Windows logging crash fix, and 4.16.0 removed the
extra warn-level lock-failure log. Native Windows confirmation remains pending.

Repository `.cargo/config.toml` forces `SQLCIPHER_OMIT_LOG` and
`SQLCIPHER_OMIT_DEFAULT_LOGGING` through the native dependency's supported
`LIBSQLITE3_FLAGS` input. This removes SQLCipher diagnostic logging from all builds,
not ORT's sanitized error reporting. It does not disable `cipher_memory_security`,
memory-lock attempts, keying, per-page authentication or integrity checks.
Memory-lock attempts are still best-effort OS operations, not proof that every
page is locked. No cryptographic format or dependency version changed.

Profile opening checks that `PRAGMA cipher_log = 'stderr'` returns the pinned
implementation's disabled-logger status before applying a key. It fails closed
if the policy is missing. Native startup tests assert that runtime configuration
cannot reactivate the logger; storage tests assert effective memory security and
keying remain enabled. CI now runs the full storage suite on all desktop targets.
Future SQLCipher upgrades must review these macros and status semantics again;
the mitigation is not a substitute for reviewing newer dependency releases.
See `../../evidence/0.0.0-dev/windows-sqlcipher-logging.md`.

## Vault boundary and remaining proof

- macOS development uses the legacy Keychain adapter because unsigned builds
  cannot honestly claim a signed application access group. Signed preview and
  release builds must prove their code-requirement/access policy across first
  run, app moves, updates, and native-host access before persistence is enabled.
- Windows uses a channel/install/profile-scoped Generic Credential. This protects
  against other accounts and offline disk access, but not malware already
  running as the same signed-in user. Windows VM tests must prove namespace,
  cross-user, update, uninstall, and denial behavior.
- Ordinary unit/integration tests use an in-memory vault and synthetic content;
  they never create OS credentials.
- `just test-platform-vault` is the explicit opt-in native proof. It creates one
  randomized credential in the platform-test namespace, verifies round-trip and
  overwrite denial, then deletes the credential through an idempotent cleanup
  guard.

## References

- [rusqlite feature documentation](https://docs.rs/rusqlite/0.40.2/rusqlite/#optional-features)
- [keyring platform stores](https://docs.rs/keyring/4.2.0/keyring/)
- [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/)
- [SQLCipher design](https://www.zetetic.net/sqlcipher/design/)
- [RustCrypto Argon2](https://docs.rs/argon2/0.6.0/argon2/)
- [RustCrypto XChaCha20-Poly1305](https://docs.rs/chacha20poly1305/0.11.0/chacha20poly1305/)
