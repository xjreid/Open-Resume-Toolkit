> Historical 0.1.0 popup milestone. The current 0.2.0 overlay-controlled workflow
> is recorded in [chrome-overlay-capture.md](chrome-overlay-capture.md).

# Chrome extension implementation — September 29, 2026

This records the initial production-source milestone. The later explicit unsigned
development BETA connection is documented in [chrome-development-bridge.md](chrome-development-bridge.md).

Status at that milestone: Chrome store-source package implemented; installed desktop delivery remains
unavailable. The existing installed ORT app was not launched, replaced, or modified.
The user's normal Chrome profile was not used or modified. Nothing was uploaded
to the Chrome Web Store.

## Changed behavior

The `0.1.0` Chrome Manifest V3 package provides a toolbar/shortcut popup for job
description and application question capture. The service worker authenticates
the exact popup sender, binds the opened tab, reads only selected main-frame text,
normalizes/bounds content, cleans the URL, checks native readiness, and rechecks
both the URL and document identity before forwarding. Acknowledgements must
identify the request. Duplicate clicks, unavailable hosts, version mismatch,
navigation/reload, invalid text, and unconfirmed delivery have bounded safe paths.
No content is persisted or automatically retried. The desktop URL-review cleanup
now strips the same additional sensitive query keys as the extension.

The permission-free development package and deferred Edge scaffold stay disabled.
The Rust native host handles a strict content-free capability probe and explicitly
reports the missing signed transport. It still rejects capture requests and
wrong origins without enabling a transport or registering itself.

## Observed checks

- Extension TypeScript build and 25 automated tests passed.
- `cargo test --locked -p ort-ipc -p ort-native-host`: 4 IPC and 4 host tests passed.
- Two focused Rust desktop tests passed: the same Chrome payload fixture reaches
  encrypted review and accepted Stage 1 data; pending/replacement/question review
  preserves existing work and rejects stale revisions.
- Desktop URL-review Vitest: 1 test passed, including the new sensitive keys.
- Clippy with warnings denied passed for IPC, native host, and desktop targets.
- Actual Chrome `154.0.8037.58` loaded the final package in a disposable profile.
  Its real popup, sender identity, activeTab/scripting access, isolated selection,
  title, cleaned URL, document identity check, real missing-native-host feedback,
  simulated host-unavailable feedback, empty selection, and return to enabled
  controls passed. No storage API or local/sync extension storage files were
  present. The successful native acknowledgement was mocked inside the test
  worker; no live desktop delivery was performed.
- The actual popup screenshot was inspected for legibility and layout. This was
  a default-scale visual check, not a complete accessibility qualification.
- ZIP validation passed: CRCs, SHA-256 receipt, sorted root entries, stable
  timestamps, manifest version, and exact equality with the browser-tested files.
- Source security/secret scans, applicable formatting, Rust formatting, and
  `git diff --check` passed.

The first browser-harness attempts exposed test synchronization/API-context and
fixture whitespace issues. These were fixed in the harness, then the final package
passed the real-Chrome smoke. Loopback/browser execution required a sandbox
escalation; it ran only the disposable-profile test and synthetic fixture server.

## Artifacts and remaining gates

Generated artifacts are under `artifacts/extension/chrome` (ignored by Git):
the `0.1.0` ZIP, its file/checksum receipt, `chrome-qa.json`, and
`popup-chrome.png`. Source, tests, shared payload fixture, build/package scripts,
submission guide, and privacy-policy draft are retained in the repository.

These checks establish Chrome capture behavior and compatibility with the desktop
review contract. They do **not** establish a live Chrome → installed host → ORT
connection. Authenticated platform transport, narrow Keychain sharing and signing,
Connect/Repair/Disconnect registration, final store ID allowlisting, desktop
notification/focus and bounded launch, and the installed-path acceptance test
remain prerequisites. The ZIP is suitable for a draft upload to obtain an ID;
store review/public distribution must wait for those gates.

See `apps/extension/README.md` and `packaging/extension/chrome/README.md`.
