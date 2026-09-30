# Test Chrome with the unsigned ORT development app

The BETA extension and the macOS development bridge use the real capture and
review implementation. Apple Developer enrollment is not required for this
local native messaging test. The app is ad-hoc signed for local execution; it is
not Developer ID signed or notarized. The production transport remains gated.

## Start testing now

The prepared outputs are:

- App: `target/release/bundle/macos/Open Resume Toolkit Dev.app`
- Extension folder: `apps/extension/dist/chrome-dev-bridge`
- Dashboard upload: `artifacts/extension/chrome/open-resume-toolkit-chrome-dev-0.2.1.zip`

1. Open `/Applications/Open Resume Toolkit Dev.app`. The installed app uses your
   existing development profile. Successful job captures automatically replace the
   overlay’s Job Description and Job URL fields, without a confirmation popup.
2. In Chrome, open `chrome://extensions`, enable **Developer mode**, choose **Load
   unpacked**, and select `apps/extension/dist/chrome-dev-bridge`. For an existing
   installation, press its **Reload** button. Approve the new HTTP/HTTPS site
   access permissions. No extension popup or pinning is needed.
3. Confirm the initial local ID is `bpmgibmpojhppibpboiodpnifkimocjn`. The exact-ID
   host is installed by `node tools/dev-browser-bridge.mjs install`. It has been
   registered on this Mac; rerun after rebuilding the native host or changing
   dashboard identity. It does not launch ORT or enable the app connection.
4. In ORT, open **Settings → Browser connections → Enable development connection**.
   Keep ORT running. The connection starts disabled after each app launch. The
   overlay's browser badge becomes connected when Chrome is communicating.
5. Open a normal job page in Chrome. On the ORT overlay, press **Capture**. Click
   the top-left of the desired text, move the pointer to preview the rectangle and
   highlighted words, scroll down if needed, then click the bottom-right. The text
   and cleaned page link fill the overlay’s editable job fields. Scroll inside the
   description to read it or move through the single-line URL. Capture preserves
   the overlay’s position and does not start AI work.
6. To cancel, press the same overlay button again; it reads **Cancel** after the
   first corner. Escape also cancels browser capture. Questions are entered
   directly in the overlay’s Answers tab.
7. Disable the connection in ORT Settings or quit ORT when finished.

Selection covers rendered DOM text in the chosen main-frame page/panel area,
including text scrolled out of view between the two clicks. Image/canvas,
PDF, closed shadow root, and embedded frame text are not OCR'd; open embedded job
pages in their own tab. Hidden text and form values are skipped. Chrome internal
and Web Store pages may reject access. Scrolling after the first click keeps the region anchored and updates the live
text highlights. Resizing, navigation, or changing Chrome tabs cancels the region. Capture expires after two
minutes. If a result cannot be confirmed, check the overlay fields before retrying.

Ordinary site access is necessary because desktop Capture cannot grant Chrome's
`activeTab` permission. Injection occurs only after Capture; idle connection
messages contain no page text. There is no extension storage, telemetry, automatic
background scraping, or external network request. URL cleanup removes known
tracking/authentication fields; check the URL field before tailoring.

## Add the BETA item to your Google dashboard

1. Go to <https://chrome.google.com/webstore/devconsole> and choose **Add new item**.
   Upload the development ZIP above. Keep it as a draft while configuring its ID.
   Use the BETA name; keep the production ORT listing separate.
2. Open the item's **Package** tab and choose **View public key**. Copy the public
   key and the **Item ID**. Google assigns the store identity; it can differ from
   the initial local ID. Do not use a private key or publisher credential here.
3. You can send the Item ID and public key to Codex to complete synchronization.
   To do it yourself, save the copied public key to a text file (PEM headers are
   accepted), then run these commands from the repository root:

   ```sh
   node tools/dev-browser-bridge.mjs configure-key --public-key-file /absolute/path/chrome-public-key.txt --extension-id YOUR_ITEM_ID
   pnpm --filter @ort/extension package:chrome:dev
   node tools/dev-browser-bridge.mjs install
   ```

   The first command verifies that the key matches the ID and increments the
   development package version when the key changes (for example, 0.2.1 → 0.2.2).
   No desktop rebuild is needed just to change the extension ID.
4. Disable the ORT connection. Remove the old unpacked BETA installation from
   Chrome and load the rebuilt `dist/chrome-dev-bridge` folder. Confirm that its
   ID equals the dashboard Item ID. Re-enable the app connection and repeat the
   capture steps above. This tests the same source with the final store ID before
   store review.
5. Upload the new version ZIP to the **same** dashboard item. A draft is not a
   store-installable extension. Loading the matching unpacked source lets you
   begin testing immediately while the item remains a draft.
6. For store-installed testing, add your testing Google account under **Account →
   Trusted testers**, choose **Distribution → Private**, and select trusted testers.
   Complete the listing, permission explanations, screenshots, public privacy
   policy URL and test instructions. The description must explain that this is
   a macOS development BETA requiring the locally installed ORT dev app and host.
7. Submit for review only after those materials and a reviewer-accessible dev app
   and host setup are prepared. Private visibility also requires Google's review.
   Once approved, install with the testing Google account. Remove the unpacked
   copy first; both installations use the same ID. The same registered host works
   with the store-installed BETA.

Google's references: [stable development ID](https://developer.chrome.com/docs/extensions/reference/manifest/key),
[private testing and BETA labels](https://developer.chrome.com/docs/webstore/cws-dashboard-distribution),
[native host registration](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging).
No dashboard upload, submission or publication was performed by this task.

## Rebuild and verify

```sh
pnpm --filter @ort/extension test
pnpm --filter @ort/extension test:browser:dev
cargo test --locked --offline -p ort-ipc --features dev-browser-bridge
pnpm --filter @ort/desktop tauri build --features dev-browser-bridge --config src-tauri/tauri.preview.conf.json --bundles app -- --locked --offline
node tools/dev-browser-bridge.mjs install
```

If pnpm 11 asks to reinstall unchanged dependencies after a script change, prefix
that command with `pnpm_config_verify_deps_before_run=false`. Chrome QA needs
permission to launch Chrome and open local sockets and a loopback fixture server.
It creates disposable browser and encrypted database profiles; it does not use
your normal Chrome profile, your actual database keys, or AI credentials.

The real Chrome test verifies overlay-authorized two-click rectangles through
the actual persistent native port, authenticated socket, and encrypted intake.
It checks live highlight expansion/shrinkage and cleanup, whole-page and panel
scrolling with off-screen beginning/middle paragraphs, cancellation after scrolling,
job/question acceptance, partial text, cancellation and late-result
rejection, empty selection, same-URL reload, session cleanup, and no unintended
page clicks. Manual installed app focus/review and store testing remain the steps
above. This does not qualify production IPC or store approval.

## Development boundary and removal

This is an explicit development exception to the signed production design. It is
compiled only with `dev-browser-bridge` on macOS and activated only for the exact
`com.openresumetoolkit.dev` app identity. The host allows only its compiled
`ORT_DEV_CHROME_EXTENSION_ID`; production and Edge IDs are not accepted by that
build. The BETA extension uses `com.openresumetoolkit.dev`, and never falls back
between development and production host names.

Enabling creates a private current-user directory `/private/tmp/ort-dev-bridge-UID`
(mode 0700), a socket (0600), and an ephemeral random authentication key in
`session.json` (0600). Requests and replies use session IDs, separate HMAC domains,
nonces, expiry, bounded frames and read deadlines. Disconnect/quit removes the
session key and socket; enabling generates a fresh key. A crashed app can leave
its previous private session until the next enable replaces it. The override
`ORT_DEV_BRIDGE_DIRECTORY` exists only in this feature for isolated QA.

The development key is readable by programs under your own macOS account.
**This does not authenticate signed processes or defend against malicious programs
running as you.** It never exports the database key, provider keys or production
installation secrets. Captures go into the normal encrypted desktop pending
review store; they are not logged or written into bridge files.

The installer writes a content-free extension ID and native binary under
`~/Library/Application Support/com.openresumetoolkit.dev/browser-bridge-dev`,
and the Chrome manifest under
`~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.openresumetoolkit.dev.json`.
No administrator access or production host registration is needed.

To remove it, disable the connection, then run
`node tools/dev-browser-bridge.mjs uninstall` and remove the BETA extension in Chrome.
The uninstall command preserves your ORT database and other Chrome registrations.
