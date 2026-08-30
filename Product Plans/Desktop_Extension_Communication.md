# Desktop and browser-extension communication

## Purpose

Chrome and Edge extensions send only user-reviewed captures to the locally installed ORT desktop application. The initial design uses browser native messaging and authenticated local IPC, not a public localhost web server.

## Components and authority

1. **Content script** — reads only the text selected after an explicit user action and sends it to the extension service worker.
2. **Extension service worker** — validates type and size, sanitizes the current URL, presents review, and communicates with the registered native host.
3. **Native-messaging host** — a minimal executable installed with the desktop app that validates browser messages and relays them through protected local IPC.
4. **Desktop app and overlay** — decide whether to accept the capture, own the application workspace, access credentials, call providers, store content, and display results.

Browser content can never directly read local records, access credentials, change the tracker, select a final document, execute a local command, or initiate an AI call without desktop authorization.

## Capture behavior

- Initial actions are job-description capture, application-question capture, capability check, acknowledgement, and user-facing error.
- Capture requires explicit user activation. Continuous page scanning, browsing-history collection, automatic form reading, and automatic submission are prohibited.
- The user reviews and may edit captured text and the sanitized URL before sending it.
- URL fragments and known sensitive/tracking query parameters are removed; the user may remove the URL entirely.
- A new capture cannot silently merge with a different active application workspace.

## Native messaging and local IPC

- The native host validates the exact extension origins, protocol version, action, message identifier, schema, size, and freshness.
- Production manifests allowlist the published Chrome and Edge extension identifiers; wildcard origins are prohibited.
- Development and production use different identifiers, host names, manifests, and IPC endpoints.
- The native host communicates with the desktop through a per-user protected mechanism such as a Windows named pipe and macOS Unix-domain socket or equivalent.
- The IPC handshake uses operating-system access control and an installation-scoped secret or equivalent defense so another local process cannot impersonate the extension bridge casually.
- Messages are bounded, expire quickly, and are idempotent. The protocol supports a deliberate compatibility window and fails safely when an update is required.
- The bridge contains no AI key and retains no permanent resume or application content.

Exact message-size, timeout, retry, protocol-version, and compatibility values belong in implementation planning and must remain within browser-native-messaging constraints.

## Desktop launch behavior

If the app is not running, the native host may launch it through a platform-approved mechanism and complete a time-bounded handshake. If launch or connection fails, the extension provides actions to:

- Open ORT manually
- Verify that the desktop app is installed
- Repair the browser connection
- Update an incompatible component

It must not repeatedly spawn processes or silently discard the reviewed capture.

## Installation and repair

- The desktop installation places the native host and manifest in controlled locations.
- Normal installation or first-run setup registers per-user Chrome and Edge native hosts without requiring manual registry or filesystem edits.
- Microsoft Store MSIX packaging may require a first-run registration/verification flow and specific registry-virtualization configuration. This must be proven during implementation and Store certification.
- The app provides a **Browser connections** screen showing installed, connected, incompatible, or repair-needed status for each browser.
- Updates atomically preserve or replace the bridge and manifest. Version mismatches fail closed and show repair guidance.
- Uninstall or a dedicated disconnect action removes ORT-owned registrations without changing unrelated browser settings.

## Permissions

- Prefer `activeTab`, `scripting`, `nativeMessaging`, and the narrowest permissions that implement deliberate capture.
- Broad host permissions require a documented product need, explicit store disclosure, and security review.
- Extension storage contains only non-sensitive preferences and pending non-content state when necessary.
- Extension source is published with the desktop source, and store packages must be traceable to repository revisions.

## Chrome and Edge variants

The extensions should share one codebase where practical while retaining distinct published identifiers. The native-host `allowed_origins` list includes both exact identifiers. Store review, update timing, and version skew are handled independently.

## Failure behavior

- Duplicate messages return the original acknowledgement or are ignored safely.
- Oversized, stale, malformed, unrecognized, or unauthorized messages are rejected before desktop processing.
- If the desktop rejects a capture, the extension keeps the reviewed text only long enough to let the user retry or copy it, then clears it.
- No extension failure may expose local database paths, secrets, stack traces, or unrelated application content.

