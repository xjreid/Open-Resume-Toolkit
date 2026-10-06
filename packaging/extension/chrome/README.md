# Chrome Web Store package and development testing

The production Chrome extension is built from repository TypeScript and
`apps/extension/manifest/chrome-store.json`. Its stable native-host name is
`com.openresumetoolkit`, and its current package version is 0.3.0.

The supplied Store ID is now configured and the installed development app/native
adapter were updated without launching the app. Follow
[local-store-test-setup.md](local-store-test-setup.md) for the current ready-to-test
installation and unpacked extension. The general setup below remains reproducible.

## First upload

Build with `pnpm_config_verify_deps_before_run=false pnpm --filter @ort/extension package:chrome`.
Upload `artifacts/extension/chrome/open-resume-toolkit-chrome-0.3.0.zip` using the
Chrome Web Store Developer Dashboard's **New item** flow. Keep it as a draft.
Google accepts a ZIP with `manifest.json` at its root; a locally signed CRX is
not needed for dashboard upload. Google assigns the Item ID. In **Package → View
public key**, copy the public key and Item ID. Neither is an account credential.
Do not provide a private key, password or publisher token.

Official instructions:
[upload/publish](https://developer.chrome.com/docs/webstore/publish/) and
[matching development identity](https://developer.chrome.com/docs/extensions/reference/manifest/key).

## Configure the same extension for the development app

After obtaining the dashboard ID/key, save the PUBLIC key to a local text file
(PEM headers/newlines are accepted). From the repository root:

```sh
node tools/configure-chrome-store.mjs --public-key-file /absolute/path/chrome-public-key.txt --extension-id YOUR_ITEM_ID
pnpm_config_verify_deps_before_run=false pnpm --filter @ort/extension build:chrome:store-test
node tools/dev-browser-bridge.mjs install --store-test
```

The first command verifies that the RSA public key actually hashes to the Item ID
and records the public identity in `apps/extension/manifest/chrome-store-key.json`.
The upload build excludes that key; the local `dist/chrome-store-test` build adds
it so **Load unpacked** uses Google's assigned ID. Both builds have identical
extension logic and the same production native-host name. Do not create a second
Store item or replace this package with a BETA package for testing.

The third command explicitly builds an origin-restricted development native host
and registers it as `com.openresumetoolkit` for the configured ID. It refuses to
overwrite a host owned by another installation. Its executable lives in the ORT
DEVELOPMENT data directory, and its registration clearly identifies `store-test`.
It uses only the temporary current-user development capability. This opt-in
native registration is the test adapter; the extension contains no fallback or
runtime switch. Only one extension ID is active in the development app at a time.

The desktop app must have been built with `--features dev-browser-bridge`, plus
the normal parser-helper packaging inputs. The currently installed default build
may need rebuilding with that feature; confirm before testing. Once ready,
restart ORT to load the registration, load `apps/extension/dist/chrome-store-test`
unpacked in Chrome, and verify that its ID matches the dashboard Item ID. Remove
or disable the old BETA extension while testing this identity. Allow site access
in Chrome. The bridge-enabled dev app enables its registered connection on launch;
Settings → Browser connections can disable it. Recovery after starting ORT may
take 30 seconds or longer if Chrome delays the alarm.

A short walkthrough should cover Capture, the two corners/live highlights,
scrolling, Cancel/Escape, cleaned URL, editable received text, no automatic AI,
and Continue/Finish/reopen in the tracker. Store-installed and signed-production
walkthroughs remain separate checks; an unpacked draft is not Store-installed.

To remove the explicit adapter **before installing the signed production host**:

```sh
node tools/dev-browser-bridge.mjs uninstall --store-test
```

Quit/disable the development connection before changing identities. The script
removes only its own registration and host, and does not change browser profiles,
resume data, provider credentials or a different host registration.

## M7 replacement boundary

Replace the test adapter with the signed production native host and desktop
installation, retaining native-host name and [protocol v1](native-protocol-v1.md).
Implement and verify signed peer/application identity, scoped Keychain access,
installation-secret authentication, bounded framing, expiry/replay rejection,
Connect/Repair/Disconnect and version compatibility. These changes stay in the
native code. Default production native builds fail closed until these gates pass.
Signing credentials alone do not complete the bridge.

Final Store publication, Chrome review, installed production capture and app/host
installation/repair are M7 work. Do not advertise public production desktop
compatibility before it is verified. Extension source is complete for the Chrome
M5 browser contract; Edge qualification remains separate.

## Submission materials

Use [store-listing.md](store-listing.md) for the single-purpose description,
permission explanations, testing notes and checklist. Complete the public privacy
policy URL/support contact and current desktop screenshots before submission.
Use [privacy-policy-draft.md](privacy-policy-draft.md) as a factual starting point.
Initial draft upload does not require claiming production bridge readiness.
