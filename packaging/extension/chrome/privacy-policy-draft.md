# Open Resume Toolkit Chrome extension — privacy policy draft

This is background material for the integrated extension and development BETA.
The October 6 dashboard task prepared [privacy-policy.html](privacy-policy.html)
with the publisher-supplied contact and effective date. Use that standalone page
for hosting, and [dashboard-fields.md](dashboard-fields.md) for the form entries.
It must still be hosted publicly before store submission.

## Data handled

When you press Capture in the ORT overlay and click two corners on the active
web page, the extension previews rendered text inside that rectangle with local live
highlights. The first corner remains anchored while you scroll, so the selected
area can include paragraphs that have scrolled off-screen. After the second
click it reads the enclosed main-frame DOM text, page URL, and title for delivery. It removes URL
credentials, fragments, and known sensitive/tracking query parameters. That
cleanup does not guarantee that every site's URL is free of sensitive information.
You can review/edit the text and remove the URL in the ORT desktop overlay.

The extension does not automatically read entire pages, browsing history,
unselected form values, passwords, cookies, resume files, or AI provider keys.

## Use and transfer

Captures are used solely to prepare the job-description or application-question
review in your locally installed ORT desktop application. The extension maintains a
content-free local status/command connection and requires a desktop-authorized
capture session before sending a capture through
Chrome native messaging to the registered local ORT host. If the connection is
unavailable, page content is not sent to the host. Capturing does not initiate AI.

The extension itself sends no page content to an ORT server, analytics service,
advertiser, or AI provider. Any later AI processing you initiate in the desktop
application is governed by that application's settings and privacy disclosures.

## Storage and retention

The extension requests no persistent storage permission and does not save or queue
job text or page links. It holds the current capture temporarily in memory during
the bounded operation. It does not log captured content. Captures accepted by the
desktop follow the desktop application's local encrypted storage and deletion
rules, which must be described in its privacy documentation.

## Controls

The extension has no popup or capture buttons. The desktop overlay arms capture
and cancels it; Escape also cancels the box. Navigation, tab changes, resizing,
or timeout cancel selection. Scrolling keeps selection active and updates highlights. The alarms permission schedules content-free connection recovery after disconnection
or worker suspension. HTTP/HTTPS site
permission enables on-demand capture; it does not cause background page reading.
You can restrict site access in Chrome, avoid capture, edit or discard a
capture in desktop review, disable or uninstall the extension in Chrome, and use
the desktop application's supported data deletion controls.

## Development BETA

The macOS development BETA requires you to install the local development host and
explicitly enable its connection in the ORT app. A temporary authentication key
is stored in a private file readable by your own macOS account while enabled.
It is removed on disconnect or normal quit. This development connection trusts
programs running under that account and does not authenticate signed processes.
It does not export your database key or AI provider keys. The BETA extension uses a separate identity. The production extension can
be tested against an explicitly registered development adapter under its normal
host name; this does not establish signed process identity. That adapter must be
removed before production native-host installation.

## Contact and effective date

- Public support email: **xjrspam1@gmail.com**
- Effective date of the prepared publishing page: **October 6, 2026**
