# Open Resume Toolkit Chrome extension

Chrome 0.2.1 provides silent, overlay-controlled rectangle capture. There is no
extension popup, toolbar action, shortcut, or send/on/off button. All user
controls live in ORT. Unsigned macOS development delivery is implemented in the
explicit BETA channel; production transport remains gated.

## Workflow

After development host setup, ORT enables the browser connection automatically
on each app launch. Settings → Browser connections can disable it until the
next launch. The overlay reports Connected only while the extension is actively
communicating; otherwise Capture stays disabled. With a normal job page active
in Chrome, press Capture on the ORT overlay. Click the
top-left corner of the text. Move the pointer to preview the blue rectangle and the highlighted words, then
click the bottom-right corner. ORT receives visible text inside the rectangle,
a sanitized page link, and the title. Job captures automatically replace the
overlay’s editable Job Description and Job URL fields without a confirmation
popup or resetting the overlay position. The description has a fixed height and
scrolls vertically; the URL stays on one line. Capture does not start AI work.
Questions are entered directly in the overlay’s Answers tab.

Press the same overlay button again to cancel. It reads Cancel after the first
corner. Escape also cancels. Scrolling keeps the first corner anchored to the
page or job-description scroll panel, so a region can span multiple screens.
Highlights update when the pointer moves or the page scrolls; shrinking the box
removes excluded words from the highlight. Selection expires after two minutes;
navigation, changing browser tabs/windows, or resizing cancels the region. Losing browser focus to the
ORT overlay allows cancellation without abandoning the selected Chrome tab.

Capture covers rendered DOM text in the chosen main-frame page/panel area, including
text scrolled off-screen between corners and
accessible open shadow trees. It skips hidden text, form values, editable fields,
and content outside the box. Open embedded job pages in their own tab. Image,
canvas, PDF, closed shadow root, and cross-frame text are not OCR'd. Chrome may
reject access to its internal pages, Web Store pages, or restricted sites.

## Permissions and privacy

- `scripting`: inject the isolated-world rectangle/highlight tool after desktop Capture.
- `nativeMessaging`: maintain a local connection to the exact registered ORT host.
- HTTP/HTTPS host access: required because a click in the desktop overlay cannot
  grant Chrome's temporary `activeTab` permission. Chrome must allow access to
  the site being captured. Site access alone does not trigger text collection.

No startup content script, extension storage, remote code, telemetry, or external
network requests are used. Idle polling exchanges content-free status/commands.
Each capture is authorized by a desktop-created UUID and target. The worker
checks the exact content-script sender, main frame, tab, URL, and document ID.
The desktop revokes authority immediately on Cancel, rejecting late delivery.

Text is normalized and bounded to 128 KiB; URLs to 4 KiB and native frames to
256 KiB. URL credentials, fragments, and known tracking/authentication query
fields are removed. Review remaining links for site-specific sensitive fields.
A persistent native port serializes bounded requests. Delivery requires a matching
acknowledgement and is never automatically retried after an uncertain result.

## Build and test

```sh
pnpm --filter @ort/extension test
pnpm --filter @ort/extension lint
pnpm --filter @ort/extension test:browser
pnpm --filter @ort/extension test:browser:dev
pnpm --filter @ort/extension package:chrome
pnpm --filter @ort/extension package:chrome:dev
```

`test:browser` runs real Chrome in a disposable profile with a loopback fixture
and mocked native responses. `test:browser:dev` uses the actual native binary,
authenticated development socket, and encrypted desktop intake/review functions.
Neither uses your normal browser profile, real database, or AI credentials. The
latter verifies live highlight expansion/shrinkage and cleanup, whole-page/panel
scrolling, cancellation after scrolling, job/question delivery, partial text, empty boxes,
page reload, and session removal. Installed GUI focus and store-installed behavior
still require the manual walkthrough in
[`development-testing.md`](../../packaging/extension/chrome/development-testing.md).
Set `ORT_BROWSER_EXECUTABLE` if Chrome is installed elsewhere.

The development unpacked folder is `apps/extension/dist/chrome-dev-bridge`.
ZIPs and checksum/file receipts are generated under `artifacts/extension/chrome`:
`open-resume-toolkit-chrome-dev-0.2.1.zip` and the corresponding production
`open-resume-toolkit-chrome-0.2.1.zip`. Python 3 is required for packaging. Receipts
record baseline commit, dirty-source status, and hashes; ZIPs use deterministic
ordering and timestamps and include license/notices. No private key is packaged.
Reload after rebuilding and approve the changed site permissions.

The legacy `dev:chrome` permission-free package and Edge scaffold remain capture
disabled. Default native-host builds report `SIGNED_BRIDGE_REQUIRED`; the store
package never falls back to the development host. The BETA uses a current-user
capability and cannot authenticate signed processes. Signing alone does not
implement the remaining production gates.

If pnpm 11 tries reinstalling unchanged dependencies after script changes, prefix
its command with `pnpm_config_verify_deps_before_run=false`.
