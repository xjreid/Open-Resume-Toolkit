# Chrome Store listing preparation

Use [dashboard-fields.md](dashboard-fields.md) for the October 6 copy-and-paste
walkthrough, current private-testing description, approved Open Folio assets,
and outstanding public policy/reviewer links. The prepared publishing policy is
[privacy-policy.html](privacy-policy.html).

## Single purpose

Capture text from a user-chosen rectangle on a job page and deliver it to the
locally installed Open Resume Toolkit desktop application for review.

## Suggested listing description

Open Resume Toolkit connects Chrome to your local ORT desktop app. In ORT's
application overlay, choose Capture, click two corners around the job text, and
review the captured text and cleaned page link in the desktop app. Live highlights
show what the box includes, and scrolling supports longer job descriptions.

The extension requires the ORT desktop app and its browser connection. It has
no separate popup/editor and never starts AI automatically. It sends selected
text only to the local desktop host. There are no extension accounts, analytics,
remote scripts, browser-history collection or saved capture queues.

Escape or desktop Cancel ends a capture. Chrome internal pages, PDF/image text,
closed shadow roots and embedded frames are not supported. Open embedded job
pages in their own tab. You can restrict site access through Chrome settings.

State the actual supported desktop release/platform when publishing. Current
integration qualification is macOS Apple Silicon DEVELOPMENT; signed production
installation is M7 work. Do not claim Windows/Edge/production release support.

## Permission explanations

- `scripting`: draw the rectangle/highlights in Chrome's isolated world and read
  the rendered text inside the explicitly selected area.
- `nativeMessaging`: send selected text to the registered local ORT native host
  and exchange content-free status/capture commands with the desktop app.
- `alarms`: reconnect after the desktop stops or Chrome suspends/restarts the
  worker, without continuously restarting a disconnected native process.
- HTTP/HTTPS site access: users capture from arbitrary job sites using the
  desktop overlay; this gesture cannot grant `activeTab`. Site access allows the
  requested injection. No startup script or passive navigation reads page text.

## Privacy disclosures to complete

The extension handles selected website text, the source URL and page title only
for local delivery/review. It does not retain content in extension storage or
transmit it over the network. Later desktop AI use is separately initiated and
controlled by the desktop app. Local processing still needs accurate disclosure
in the dashboard. Match Google's current data categories to these facts; do not
claim that the extension never handles website content or URLs.

Publish the reviewed privacy policy at a public URL. Supply the publisher's actual
support contact, effective date, source URL and desktop download/setup instructions.
No placeholder or invented contact should be submitted. Prepare current screenshots
of the rectangle on synthetic job text and the desktop review, using no personal data.

## Review/test instructions

Explain the companion-app requirement and provide a working compatible desktop
build plus native-host setup. The reviewer must be able to see Connected, invoke
Capture, select text, see the editable result, and cancel/retry safely. No AI key
is required to test capture. Distinguish explicit development testing from signed
production support. Choose draft/private tester distribution while qualifying the
integration; store installation/review and public release are separate steps.

No upload, review submission, or public publication is performed by the build tools.
