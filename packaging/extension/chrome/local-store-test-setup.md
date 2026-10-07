# Test the saved Store draft with the installed development app

Configured October 6, 2026:

- Store extension ID: `bejnkiimonenibbhhnoifnbleadkonbn`
- Native-host name: `com.openresumetoolkit`
- Desktop app identity: `com.openresumetoolkit.dev`
- Installed app: `/Applications/Open Resume Toolkit Dev.app`
- Native adapter: `~/Library/Application Support/com.openresumetoolkit.dev/browser-bridge-dev/ort-native-host-store-test`
- Unpacked extension: `apps/extension/dist/chrome-store-test`
- Test ZIP: `artifacts/extension/chrome/open-resume-toolkit-chrome-unpacked-test-0.3.0.zip`

The supplied public key was verified to derive the exact Store ID and saved in
`apps/extension/manifest/chrome-store-key.json`. The test build has the same
JavaScript as the production upload, with the public key added to its manifest
so loading unpacked preserves the Store ID. The original dashboard ZIP is unchanged.
Use the test ZIP locally rather than replacing the saved dashboard package with it.

The desktop app was rebuilt with `dev-browser-bridge`, preserving its development
identity and pinned sandboxed parser helper. Its signature and all 25 installed
file entries were verified. The native host permits only the Store ID above and
matches the tested executable byte for byte. The previous app and native
registration were retained for rollback. The installed app was not opened;
resume/profile/provider credentials were not modified.

## Manual test

1. Open `chrome://extensions` in Chrome and enable **Developer mode**.
2. Disable an old ORT BETA extension if present. Choose **Load unpacked**, then
   select `/Users/xavierreid/Open-Resume-Toolkit/apps/extension/dist/chrome-store-test`.
   Alternatively extract the test ZIP and select the folder containing `manifest.json`.
3. Verify ID `bejnkiimonenibbhhnoifnbleadkonbn`. Allow site access for the
   HTTP/HTTPS job page you will test. There is no popup or need to pin the extension.
4. When ready, open **Open Resume Toolkit Dev** yourself. Its registered
   development connection enables on launch. If needed, choose **Settings →
   Browser connections → Enable development connection**. Allow up to 30 seconds
   for reconnect; Chrome may delay the alarm longer.
5. Open the application overlay and confirm its browser badge says **Connected**.
6. Open a normal job page in Chrome. Choose **Capture** in the desktop overlay,
   then click two opposite corners around job text. Inspect the live highlights
   and scroll before the second click to include longer descriptions.
7. Confirm the editable job description and cleaned link appear in the desktop
   overlay. Capture does not start AI or require an AI key. AI tailoring remains
   a separate desktop operation with its own requirements.
8. Start another capture and cancel with Escape or desktop Cancel. Confirm that
   the prior job fields remain. Quit or disable the connection when finished.

A draft is not Store-installable. The unpacked copy permits immediate local
testing while the public policy URL, reviewer installer/setup, and private Store
review/publication remain pending.

## Verification

- All 32 extension regressions passed.
- Real Chrome 154.0.8037.98 with the exact Store ID and `com.openresumetoolkit`
  passed native capture into a temporary encrypted desktop workspace. This covers
  live highlights, page/panel scrolling, cancellation, URL cleanup, partial/empty
  selections, navigation rejection, and no unintended page clicks.
- Installed native helper matches the tested binary; bundle signatures passed.
- Formatting, web security, secret, and patch whitespace checks passed.
- Manual installed-app GUI testing is left for the user; automation kept it closed.

Generated installation, native-host, and test-package receipts are in
`target/chrome-store-setup-2026-10-06/`.

## M7 replacement

Keep `com.openresumetoolkit` and protocol v1. Quit the development app and remove
this explicit adapter using `node tools/dev-browser-bridge.mjs uninstall --store-test`
before installing the signed production host. Stronger native identity checks
remain app/host work; no extension host-name change is required.
