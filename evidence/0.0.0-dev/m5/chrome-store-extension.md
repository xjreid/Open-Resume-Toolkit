# Chrome Store extension implementation — 2026-10-06

Chrome 0.3.0 is implemented in repository TypeScript and packaged as a Chrome Web
Store upload ZIP. This completes the requested Chrome browser code/package work;
it does not close all of M5 or qualify the signed M8 native bridge.

## Result

- Preserves the silent desktop-overlay rectangle workflow, live highlights,
  scroll anchoring, exact-document authority, bounded text/URL intake, cancellation
  and no automatic AI.
- Replaces a continuous disconnected 500 ms native-host retry loop with a
  browser-owned 30-second recovery alarm. Connected polling remains serialized
  at 500 ms, and the alarm also recovers an unexpectedly stopped worker.
- Bounds native requests/responses to 256 KiB and queues to eight operations.
  Disconnect/timeout invalidates queued content. Stale-port responses, malformed
  Unicode and mismatched progress-protocol versions fail closed.
- Freezes the production host name `com.openresumetoolkit` and documents native
  protocol v1, including persistent ordered port responses. M8 authentication
  is internal to desktop/native-host code, with no secret sent to the extension.
- Adds verified dashboard public-key/ID synchronization and matching unpacked
  Store-identity builds. An explicit `--store-test` registration connects this
  unchanged production extension contract to the development app. It cannot
  overwrite an unrelated production host. No automatic fallback was introduced.
- Adds deterministic packaging with exact file/security checks, root manifest,
  local module/PNG checks, source/file SHA-256 receipts and ZIP CRC verification.
- Supplies upload, setup, removal, listing and privacy documentation. The publisher
  supplied a support contact and final Google identity; the public privacy URL,
  reviewer installation links and Store-installed evidence remain follow-ups.

## Verification

- Extension TypeScript: passed.
- Extension regressions: 32 passed, 0 failed, including suspension/recovery,
  stale queues, frame/queue limits, document authority, protocol rejection,
  leaked/symlinked package files and mismatched dashboard identities.
- Actual Chrome 154.0.8037.98 with synthetic pages and mocked native replies:
  rectangle/highlight expansion/shrinkage, scrolling/panel capture, cancellation,
  URL cleanup, partial/empty text, same-URL reload and no extension storage passed.
- Same production JavaScript and native-host name with a temporary public key:
  real Chrome native messaging → authenticated development socket → temporary
  encrypted desktop intake/review passed (one explicit Rust integration test).
  The user's installed ORT app, profile, keys and normal Chrome profile were not used.
- Web security, repository secret scan and patch whitespace checks passed.
- Production build/package passed. ZIP CRCs and all 17 file hashes verified;
  independent repeated packaging produced the same archive digest.

The Chrome test exposed an unbound worker timer invocation that Node did not
reproduce. Wrapping timer calls with their proper global receiver fixed it; the
corrected real-Chrome and real-native integration checks both passed.

Local artifacts (ignored generated output):

- `artifacts/extension/chrome/open-resume-toolkit-chrome-0.3.0.zip` (34,479 bytes)
- ZIP SHA-256: `37df342a98545ecca96d878156ef71ff1959bb93c8a950b04dd36459c2ba25c8`
- Adjacent `.zip.json` source/artifact receipt
- `chrome-qa.json` and `chrome-store-contract-qa.json`
- Synthetic capture screenshots; the production-contract screenshot was visually inspected.

The upload has production host configuration, no dashboard key/private material,
no test data/native binary, and no development-host fallback. The source changes
are local and uncommitted. The user uploaded the package and saved a draft;
automation has not pushed code or submitted/published the Store item.

## Store identity synchronization and local installation

The supplied public key derives `bejnkiimonenibbhhnoifnbleadkonbn` and is recorded
in `apps/extension/manifest/chrome-store-key.json`. The Chrome/native/encrypted-intake
test was rerun successfully with that exact ID and `com.openresumetoolkit`.
Test tooling now consistently selects the configured Store identity in contract mode.

The exact-ID development adapter is registered in the current user's Chrome native
host directory as `com.openresumetoolkit`. Its copied binary matches the executable
tested with real Chrome. The development app was rebuilt with `dev-browser-bridge`
and installed at `/Applications/Open Resume Toolkit Dev.app`, retaining
`com.openresumetoolkit.dev`, its pinned sandboxed parser helper, and approved branding.
All 25 installed file entries and deep/strict signatures were verified. The previous
bundle and native registration were retained. The app remained closed and profile
data/provider credentials were not modified. Receipts are in
`target/chrome-store-setup-2026-10-06/`.

The unpacked test ZIP adds only the dashboard public key to the manifest; all
JavaScript matches the production upload. The original upload ZIP is unchanged.
The user can follow `packaging/extension/chrome/local-store-test-setup.md` for
manual installed-app testing. Store-installed and signed-production testing remain
unqualified.

M8 still implements/qualifies signed app/host identity and Keychain controls,
production installation/repair and Store-installed integration. Edge is separate
outstanding M5 work. Signing alone does not implement the missing native controls.
