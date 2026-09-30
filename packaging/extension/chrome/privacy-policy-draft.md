# Open Resume Toolkit Chrome extension — privacy policy draft

This is a draft for the integrated extension and development BETA. Fill in the
public contact information and effective date, verify the disclosures for the
chosen distribution, and host this policy before store submission.

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
or timeout cancel selection. Scrolling keeps selection active and updates highlights. HTTP/HTTPS site
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
It does not export your database key or AI provider keys. The production host
and extension use a separate identity.

## Contact and effective date

- Public support email: **to be supplied by the publisher**
- Effective date: **to be set for the integrated release**
