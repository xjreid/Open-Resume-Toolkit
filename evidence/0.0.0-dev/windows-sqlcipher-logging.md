# Windows SQLCipher logging mitigation

Date: 2026-09-02. Base commit: `e643574`. Local verification: macOS arm64.
Status: targeted mitigation implemented; Windows CI confirmation pending.

## Diagnostic evidence

The supplied windows-2025 log starts at `2026-09-02T22:53:08.5295386Z`.
At approximately 23:04:54Z, the isolated `native_startup` test successfully opens
an in-memory connection and reads `cipher_version`, then crashes after
`native-startup: opening encrypted profile`. It reports stack overflow / exit
`0xc00000fd (STATUS_STACK_OVERFLOW)`. At approximately 23:05:06Z the separate
`import_storage` process fails at the same encrypted-profile-opening stage.
The test can therefore fail without concurrent import tests or import mapping.
OpenSSL missing-debug-symbol warnings are not the reported failure.

This log has no native backtrace. The responsible call chain is an inference
from the narrowed stage and inspected pinned source, not a captured Windows
stack trace or a locally reproduced Windows crash.

## Matching source defect and mitigation

`rusqlite 0.40.2` selects `libsqlite3-sys 0.38.2`, bundling SQLCipher 4.14.0.
In that amalgamation, `sqlcipher_mlock` logs a warning when Windows `VirtualLock`
fails. `sqlcipher_fprintf` formats Windows stderr output with `sqlite3_vmprintf`
and `sqlite3_malloc`. With `cipher_memory_security` enabled, these allocations
can attempt another memory lock and recursively log its failure.

Upstream's [SQLCipher changelog](https://github.com/sqlcipher/sqlcipher/blob/master/CHANGELOG.md)
documents a Windows logging-allocation crash repair in 4.18.0; 4.16.0 separately
removed the warn-level lock-failure message. Our pinned source predates both.
This is a strong match to the observed failure, not yet proof the CI crash is fixed.

`.cargo/config.toml` supplies `SQLCIPHER_OMIT_LOG` and
`SQLCIPHER_OMIT_DEFAULT_LOGGING` through `LIBSQLITE3_FLAGS` with `force = true`.
The native build script tracks this environment input and rebuilds when changed.
This compiles out the diagnostic logger instead of raising stack limits,
serializing away the original failing tests or disabling memory protection.
No dependency version or encrypted database format changed. Encryption,
per-page authentication, memory wiping/lock attempts and integrity checks remain.
OS memory locking remains best effort. ORT's sanitized errors and static progress
labels remain available; raw SQLCipher diagnostic logging is intentionally absent.

Before keying an encrypted connection, storage queries the fixed target
`PRAGMA cipher_log = 'stderr'`. The pinned implementation returns text `"1"`
(`SQLITE_ERROR`) when the logger is compiled out, versus `"0"` when it can be
enabled. Any unexpected status fails closed with `CipherUnavailable`.
No document text or file path is supplied to the logger. Cargo must be invoked
within the repository's configuration ancestry; builds missing the policy are
not permitted to silently open profiles. Recheck these semantics on upgrades.

## Verification and next acceptance step

- Native dependency rebuilt locally with the new flags.
- Isolated `native_startup` passes through logger rejection, profile creation,
  save/publish, integrity checking and reopen.
- Storage regression asserts effective `cipher_memory_security` and
  `cipher_status` both report `"1"`; ciphertext/no-plaintext and recovery tests
  remain active. Import review/commit-race/restart tests pass locally.
- Full local verification is recorded in `manifest.json`.
- All three desktop CI targets now run the full storage suite as well as the
  separate startup and original import tests. No failing tests are skipped.
- Next acceptance step: push and inspect native windows-2025 execution. If it
  still fails, retain the new last stage and obtain a native debugger backtrace;
  do not mark it fixed or weaken encryption based only on macOS success.

Only temporary synthetic profiles and an in-memory vault were used. No personal
profile or Keychain/Credential Manager access was needed. The previous packaged
desktop app was not rebuilt, and therefore does not contain this mitigation yet.
