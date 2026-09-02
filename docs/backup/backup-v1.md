# Portable backup container v1 prototype

- Extension: `.ort-backup`
- Format: 1.0
- Status: development prototype; cross-platform file and hostile-input suites remain release gates

## Fixed header

All integers are unsigned big-endian. The complete 76-byte header is AEAD
associated data.

| Offset | Bytes | Meaning | v1 value/policy |
| ---: | ---: | --- | --- |
| 0 | 4 | magic | `ORTB` |
| 4 | 2 | format major | `1` |
| 6 | 2 | format minor | `0` |
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
portable profile records. The manifest includes schema versions, inventory, and
SHA-256 of the canonical profile. It contains no database key, vault reference,
provider credential, native IPC secret, diagnostics, cache, or index.

Reader validation is ordered: fixed header length/magic, versions and reserved
bytes, KDF policy, ciphertext length/exact file length, Argon2id derivation, AEAD
authentication, bounded JSON parsing, manifest/hash/inventory validation, then
domain validation. Wrong passphrase, ciphertext modification, truncation, and
malformed encrypted content return the same invalid-backup category.

## Development test vector

- Passphrase: `vector passphrase` (synthetic fixture only)
- Salt: sixteen `0x11` bytes
- Nonce: twenty-four `0x22` bytes
- Created time: `2026-09-01T12:00:00Z`
- SHA-256 of the complete container:
  `bad075c8e1369c6aa67f4b41d422826e84cde14070e43724caa063cae26e90aa`

The vector is generated and verified in the `ort-backup` unit suite. Any format
change must intentionally update the format version or explain and review a
vector change.

