# Chrome Web Store dashboard fields

Prepared October 6, 2026 against the Chrome 0.3.0 package and the publisher's
screenshots. Keep this item as a draft, with **Private** distribution for development
testing. The desktop companion is still being qualified; a signed public desktop
release and public reviewer download/setup links are not available yet.

## Store listing

- **Title:** Open Resume Toolkit (read from the package).
- **Summary:** Read from the package; leave the current summary.
- **Category:** Workflow & Planning.
- **Language:** English.

### Description — paste this

```text
Bring job descriptions from Chrome into your Open Resume Toolkit desktop workspace.

Open Resume Toolkit is the browser companion for the Open Resume Toolkit desktop app. Start Capture from the desktop application overlay, click two opposite corners around the job text, and review the selected text and source link in the desktop app.

• Choose the text you want with a two-click selection box.
• See live highlights showing which text is included.
• Scroll while selecting longer job descriptions.
• Review and edit the captured job description and page link in the desktop app.
• Cancel with Escape or the desktop Cancel control.

The extension has no separate popup or editor. Capture is controlled from the desktop overlay and requires a compatible desktop app with its browser connection enabled.

The extension sends selected text, the cleaned page URL, and the page title only to the local desktop app through Chrome native messaging. It does not upload captures to a server, use analytics, or save captures in extension storage. Capturing does not start AI processing. Any later AI use is a separate action in the desktop app.

Capture works with rendered text on normal HTTP and HTTPS pages. Chrome internal pages, text inside embedded frames, and text available only as images or PDF content are not supported. Open embedded job pages in their own tab before capturing.

Current testing is limited to macOS on Apple Silicon with the compatible development desktop build and native browser host. This listing is for private development testing while desktop integration is qualified.

Support: xjrspam1@gmail.com
```

The last testing paragraph describes the current private release. Update it to the
actual supported signed desktop release/platforms before public distribution.

### Graphic assets

Generated files are in `artifacts/extension/chrome/store-listing/`:

| Field | File | Dimensions / format |
| --- | --- | --- |
| Store icon | `store-icon-128.png` | 128×128 PNG; 96×96 approved artwork, transparent padding |
| Screenshot 1 | `screenshot-01-select-job.png` | 1280×800, 24-bit RGB PNG, no alpha |
| Screenshot 2 | `screenshot-02-scroll-job.png` | 1280×800, 24-bit RGB PNG, no alpha |
| Small promo tile | `small-promo-440x280.png` | 440×280, 24-bit RGB PNG, no alpha |
| Marquee promo tile | `marquee-promo-1400x560.png` | 1400×560, 24-bit RGB PNG, no alpha; optional |

Upload the PNGs individually, not the assets ZIP. The small promo tile is required
by Google's image documentation even though the pictured field has no asterisk.
Leave **Global promo video** blank. Both screenshots can be used; only one is
required and the maximum is five.

The icon and tiles render the approved October 4 Open Folio SVG masters, with no
new identity or generated logo. Screenshots show the real packaged extension UI
in Chrome on a fictional sample careers page, using the current Hanken font and
Open Folio colors. Native transport is mocked for these pictures; they do not
claim a signed desktop installation. The installed desktop app was not opened.

### Additional fields

- **Official URL:** None, until there is an owned and verified website.
- **Homepage URL:** Leave blank for now.
- **Support URL:** Leave blank for now; a support email is not a webpage URL.
  Use `xjrspam1@gmail.com` in the developer account support/contact email field
  where offered. It is also included in the listing and privacy policy.
- **Mature content:** Off.
- **Item support visibility:** Enable when publishing so users can ask for help;
  it does not need to be changed to save this draft.

## Privacy

Each justification below is shorter than the dashboard's 1,000-character limit.

### Single purpose description — paste this

```text
Capture job-description text from a user-selected rectangle on the active web page and deliver that text, the source page link, and title to the locally installed Open Resume Toolkit desktop app for review. Capture starts only from the desktop app's application overlay and is completed by the user clicking two corners on the page.
```

### scripting justification — paste this

```text
The scripting permission injects the packaged capture code into the active page only after the user starts Capture in the connected desktop app. It draws the two-click selection box, previews the rendered text inside the chosen area with live highlights, and reads that text when the user completes the selection. The code runs in Chrome's isolated world. It is not injected automatically on every page, and it does not read password or editable form values.
```

### nativeMessaging justification — paste this

```text
The nativeMessaging permission connects the extension to the registered Open Resume Toolkit native host on the same computer. This local connection receives desktop-authorized capture and cancel commands, exchanges connection status, and delivers the user's selected text, cleaned source URL, and page title to the desktop app for review. The companion desktop app is required for the extension's single purpose. Native messages are data and commands, not executable code; captures are not sent to a remote server by the extension.
```

### alarms justification — paste this

```text
The alarms permission schedules recovery of the local desktop connection after the desktop app disconnects or Chrome suspends or restarts the extension's background worker. The alarm performs a content-free connection/status check so the desktop overlay can start the next user-requested capture. It does not read web pages, collect browsing activity, or retry previously captured page content.
```

### Host permission justification — paste this

```text
Access to HTTP and HTTPS sites is required because users capture job descriptions from different job boards and employer websites. Capture is initiated in the native desktop overlay, which does not grant Chrome's activeTab permission. The host permissions allow on-demand injection into the active page and access to its source URL for that user-requested capture. There is no startup content script or passive collection of page text. Only text in the user's selected area is delivered to the local desktop app. Users can restrict site access in Chrome.
```

### Are you using remote code?

Choose **No, I am not using remote code**. The screenshot currently has Yes
selected; change it. All extension JavaScript is included in the ZIP. Native
messaging to the locally installed companion is not remotely hosted extension
JavaScript or WebAssembly. The remote-code justification should disappear or
remain blank after selecting No.

### Data usage

The data-type checklist is above the area shown in the supplied screenshots.
Disclose **Website content** (the selected text) and **Web history** (the captured
source page URL and title, not a complete browsing-history log). Do not claim that
no data is handled just because it stays on the user's computer.

The extension does not intentionally request account identity, health/payment
records, credentials, personal messages, geolocation, or behavioral analytics.
Selected website text can contain personal information, such as a recruiter's
name or contact details. The privacy policy explicitly covers that possibility;
if captures in your intended use include such information, also select the
matching **Personally identifiable information** category.

Check all three certification statements, consistent with the current extension
and the publisher's stated practices:

1. Data is not sold/transferred outside permitted uses.
2. Data is not used/transferred for unrelated purposes.
3. Data is not used for creditworthiness or lending.

### Privacy policy URL

This field remains pending. Host the prepared `privacy-policy.html` at a publicly
accessible HTTPS address, then paste that address. A product website is not
required just to host a policy; a static page can provide the policy. Do not paste
a local filesystem path or an invented URL. Local-only capture still requires a
policy under Google's rules.

The prepared policy includes the publisher-supplied contact email
`xjrspam1@gmail.com`. No hosting or publication was performed by this task.

## Distribution

- **Payments:** Free of charge. The extension has no purchase/subscription UI.
- **Visibility:** Private for the current development testing phase.
- Add the Google account used to install/test Chrome as a trusted tester in
  developer account settings. It can differ from the support email.
- **Google Group dropdown:** None is fine when using individual trusted testers.
- **Countries/regions**, if offered: choose the intended availability. For this
  private testing stage, include the country where the tester's account is used.

Private items still go through store review. Unlisted allows anyone with the link;
Public makes the item discoverable. Neither is needed for the current draft.

## Test instructions

- **Username:** Leave blank; there is no extension account/login.
- **Password:** Leave blank; do not enter a Google password or AI API key.
- **Additional instructions:** The following text is below 500 characters.

```text
Requires the compatible Open Resume Toolkit desktop app and native messaging host on macOS Apple Silicon. No extension login or AI key is needed. Install the supplied testing build and host; enable Browser connection in desktop Settings. On an HTTP/HTTPS job page, choose Capture in the desktop overlay, click two corners around text, and verify the editable text/link in the app. Escape cancels. Desktop download/setup instructions must accompany this submission.
```

Do not submit for review with only that text: provide a working downloadable
desktop testing build plus concrete native-host setup instructions that the
reviewer can actually use. Those links are pending. Saving the listing as a draft
does not require pretending that a public installer already exists.

## Next step after saving the draft

Obtain this item's extension ID and public key from the dashboard, configure the
matching store-test identity, and install the explicit development adapter using
`development-testing.md`. This makes the production extension's local identity
match its desktop host for testing. It does not sign the desktop app or complete
the M7 native identity work.

## Sources

- [Image requirements](https://developer.chrome.com/docs/webstore/images)
- [Privacy dashboard fields](https://developer.chrome.com/docs/webstore/cws-dashboard-privacy)
- [Local processing and privacy policies](https://developer.chrome.com/docs/webstore/program-policies/user-data-faq)
- [Distribution settings](https://developer.chrome.com/docs/webstore/cws-dashboard-distribution)
- [Current listing categories](https://developer.chrome.com/docs/webstore/best-practices)
