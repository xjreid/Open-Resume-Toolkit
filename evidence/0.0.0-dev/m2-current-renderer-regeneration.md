# M2 current-renderer regeneration — 2026-09-06

High-reasoning source/receipt boundary work on the M1-complete baseline. This is
an uncommitted local checkpoint; the installed application remains the qualified
M1 build and Step 5 remains incomplete.

## Behavior and boundaries

Profile and authenticated portable history now offer a separate, explicit
**Regenerate with current renderer** action using the selected PDF style. The
original **Verify & replay** action still compares every historical receipt field
and exposes nothing on mismatch. Regeneration compares document SHA-256 and
schema version against the historical receipt, permits presentation changes,
and returns the current renderer/template/font tuple and generated PDF hash.
It never falls back automatically after replay failure or substitutes a newer
source. Superseded active drafts remain unavailable; immutable older publications
and exact sources retained in authenticated backups are supported.

New main-window-only commands accept canonical manifest/archive tickets and an
explicit bundled style. They accept no paths, document bodies, template content,
or caller-supplied receipts. Both use the existing export lease, bounded native
renderer, preview cache/expiry, accessible text, and exact-byte export. Portable
sessions retain at most 20 exact sources for ten minutes, including sources with
historical renderer identifiers. Archive authentication, release/expiry and
read-only isolation remain in place; portable regeneration adds no active-profile
history or documents.

Profile regeneration records the current receipt using existing history identity
and retention rules: the newest 100 source/revision/PDF identities are retained,
and identical identities increment their existing counter. Existing receipts are
not rewritten to claim a new tuple. No storage migration or backup-format change
was introduced. Generated request schemas are additive; response shapes are
unchanged. History metadata permits bounded historical bundle identifiers, while
actual preview responses still require the installed bundle and known templates.
The client additionally binds regeneration to the requested style, source,
revision, document hash and document schema version.

The preview persistently labels regenerated output and identifies the old/current
renderer and style even after PDF loading updates the status message. It explicitly
says this is not an exact replay. The detailed current receipt remains available.
Unsupported glyph, layout, byte, expiry and source errors fail through existing
bounded paths. Final style/native visual qualification is not claimed here.

## Verification

- `CI=true pnpm check`: 87 desktop tests, 24 contract tests, formatting,
  TypeScript/builds, static security and license checks passed.
- `cargo test --workspace --all-targets --locked`: 204 passed, 9 explicitly gated
  tests ignored. A subsequent focused run passed all eight native PDF tests after
  adding the final original-receipt/publication preservation assertions.
- Workspace Clippy with all targets/features and warnings denied passed; final
  focused desktop Clippy and Rust formatting also passed.
- Contract generator rerun was byte-identical to the generated working-tree
  snapshot. Comparison against Git remains intentionally different because these
  generated contract changes are not committed.
- Native tests cover changed source/schema refusal, all three selected styles,
  strict historical replay, older publication/current draft selection, original
  receipt preservation, independent new receipt storage, archive authentication,
  exact-source limits and ticket expiry. Client tests reject substituted response
  source, revision, document hash/schema and style. Live UI tests verify the
  persistent label, explicit command and absence of save/publish/replay side effects.

Logs: `target/m2-regeneration-web-check.log`, `target/m2-regeneration-rust-tests.log`,
`target/m2-regeneration-native-final.log`, `target/m2-regeneration-clippy.log`, and
`target/m2-regeneration-generated-check.log`. No native account switch, Keychain
access, application installation, commit or push was performed.

Next Medium work: final editor/style presentation review and remaining layout
conveniences. Native regeneration acceptance remains part of later qualification.
The sandboxed import adapter and lifecycle/cleanup boundaries still require High.
