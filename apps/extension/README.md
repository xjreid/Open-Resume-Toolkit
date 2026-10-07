# Open Resume Toolkit Chrome extension

Chrome 0.3.0 is a Manifest V3 desktop companion. All capture/review controls stay
in ORT: no extension popup, toolbar action, competing editor, or automatic AI.
The TypeScript implementation, build tools, regression tests and packaging policy
are maintained here. Edge remains a capture-disabled scaffold and is separate work.

## Workflow

With ORT connected and a normal job page active in Chrome, press **Capture** in
the desktop overlay. Click the first corner, move the pointer to preview the blue
rectangle and highlighted words, then click the opposite corner. Scrolling keeps
the first corner anchored to the page or job-description scroll panel. Shrinking
the region removes excluded words. Received job text and its cleaned link fill
the editable desktop fields; capture never starts AI. Questions can be entered
in the overlay's Answers tab.

Desktop Cancel or Escape cancels capture. Navigation, tab/window changes, page
resize and the two-minute deadline also cancel. Focus may move from Chrome to
ORT for cancellation without abandoning the selected Chrome tab. Hidden text,
form values and editable fields are skipped. Main-frame DOM text and accessible
open shadow trees are supported; images, canvas, PDFs, closed shadow roots and
embedded frames are not OCR'd. Open embedded job pages in their own tab.

Chrome's site-access setting must allow the page. Internal Chrome/Web Store pages
and other restricted pages cannot be captured. A failure removes capture controls
and leaves existing desktop content available. An unconfirmed delivery is never
resent automatically; inspect the desktop fields before capturing again.

## Privacy and permissions

- `scripting`: inject the isolated-world rectangle only after desktop authorization.
- `nativeMessaging`: communicate with the exact registered local ORT host.
- `alarms`: recover a disconnected or suspended worker without a rapid host-launch loop.
- HTTP/HTTPS host access: a desktop overlay click cannot grant `activeTab`, and
  this workflow supports user-chosen job sites. Site access permits injection;
  it does not initiate reading. Chrome users may restrict access to specific sites.

There are no startup content scripts, extension storage, remote code, telemetry,
external network calls, cookies, browsing-history APIs or AI credentials. Idle
messages contain only bridge/session state. Selected text is held in memory,
bounded to 128 KiB, and delivered once through native messaging. URLs are bounded
to 4 KiB and stripped of credentials, fragments and known sensitive/tracking
query fields. Site-specific sensitive fields may still require removal in ORT.

The service worker checks the exact extension sender, tab, frame, document and
URL. Requests/responses are limited to 256 KiB and eight queued native requests.
Disconnect or timeout invalidates queued requests, including captures. A live
connection has a serialized 500 ms heartbeat. Disconnected workers release the
port and recover through a 30-second Chrome alarm (Chrome may delay alarms).
Restarting the worker recreates its recovery alarm. No content is queued across
worker restarts or saved to disk.

## Build, test and upload

From the repository root:

```sh
pnpm_config_verify_deps_before_run=false pnpm --filter @ort/extension test
pnpm_config_verify_deps_before_run=false pnpm --filter @ort/extension lint
pnpm_config_verify_deps_before_run=false pnpm --filter @ort/extension test:browser
pnpm_config_verify_deps_before_run=false pnpm --filter @ort/extension test:browser:store-contract
pnpm_config_verify_deps_before_run=false pnpm --filter @ort/extension package:chrome
```

The upload is `artifacts/extension/chrome/open-resume-toolkit-chrome-0.3.0.zip`.
The adjacent JSON receipt records source/artifact hashes and dirty-source status.
Packaging checks the exact file inventory, permissions, native host, local module
imports, PNG dimensions and ZIP CRCs. The manifest is at the ZIP root. There are
no source maps, test fixtures, native binaries, credentials or profile files in
it. ZIP ordering and timestamps are deterministic. Do not upload the BETA ZIP.

`test:browser` uses headless Chrome with synthetic pages and mocked native replies.
`test:browser:store-contract` uses the unchanged production JavaScript/host name,
a local public identity, actual native messaging, authenticated development IPC
and temporary encrypted desktop review. Both use disposable browser profiles.
Neither launches the installed ORT app, uses real AI credentials or qualifies
production signing. Set `ORT_BROWSER_EXECUTABLE` if Chrome is installed elsewhere.

## Same extension, development and production native bridges

The upload always calls **`com.openresumetoolkit`**. It has no automatic fallback
to a development host. [Native protocol v1](../../packaging/extension/chrome/native-protocol-v1.md)
is the browser-facing compatibility boundary for M5 and M7. Process identity,
Keychain access and native authentication are implemented inside the desktop/host;
no signing secret or authentication capability enters the extension.

Upload the ZIP as a **draft**, then obtain its Item ID and public key from Google.
[Store setup](../../packaging/extension/chrome/README.md) explains how to configure
an unpacked copy with that same identity and explicitly register the development
bridge under the production host name. This changes native registration and the
local manifest public key, not extension logic. The registration cannot overwrite
an unrelated/signed host and must be removed before production host installation.

M7 still implements/qualifies the signed desktop/host, protected vault identity,
installation/repair and Store-installed release. Signing alone does not implement
those controls. Preserve protocol v1 when replacing the native transport; final
Store review may require package/listing updates. This task does not close M5's
Edge work or authorize public release.

The separate BETA package continues to call `com.openresumetoolkit.dev` and uses
`dist/chrome-dev-bridge`; its setup is in [development-testing.md](../../packaging/extension/chrome/development-testing.md).
