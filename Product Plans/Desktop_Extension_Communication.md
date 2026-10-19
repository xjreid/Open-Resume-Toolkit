# Desktop and browser-extension communication

## Purpose

Chrome and Edge extensions deliver deliberate captures and the separately
opted-in planned M5 [application-answer feature](Application_Answer_Autofill.md)
through browser native messaging and authenticated local IPC, not a public
localhost web server. Autofill is approved scope, not current implemented behavior.

## Components and authority

1. **Content script** — existing capture reads explicitly selected text. A separate planned monitor observes supported foreground application fields only under revocable desktop authorization; it never performs whole-page/background collection.
2. **Extension service worker** — validates type and size, sanitizes the current URL, and communicates with the registered native host. It does not present a parallel job-review or tailoring workflow.
3. **Native-messaging host** — a minimal executable installed with the desktop app that validates browser messages and relays them through protected local IPC.
4. **Desktop app and overlay** — decide whether to accept the capture, own the application workspace, access credentials, call providers, store content, and display results.

Browser content cannot enumerate local records, access credentials, change the
tracker, select a final document, execute a local command, or initiate an AI call.
The planned answer capability returns only answers authorized for bound fields;
explicit extension-popup/desktop interactions alone can search the answer library.

## Current implementation boundary

Chrome 0.3.0 implements desktop-authorized two-corner rectangle capture through
the explicit macOS development host. The overlay owns review and AI dispatch.
Chrome uses `scripting`, `nativeMessaging`, `alarms`, and declared HTTP/HTTPS site
access because an overlay click cannot grant `activeTab`; it has no toolbar
popup/action or background content reader. Users may restrict site access.
Question text can be entered manually in Answers. Edge remains capture-disabled.
Production launch/repair/identity/Store requirements below remain open; see the
[implementation map](../docs/development/current-state.md).

Planned M5 adds the Application answers desktop library and a compact toolbar
popup for approvals, Use once/alias linking, Undo, and stop actions. It is distinct
from capture review and makes no AI call. Follow the [feature authority](Application_Answer_Autofill.md)
and [technical plan](../Implementation%20Plans/Browser%20Extensions/Application_Answer_Autofill_Plan.md);
neither the popup, monitor, library, nor v2 answer protocol is implemented today.

## Capture behavior

- Initial actions are job-description capture, application-question capture, capability check, acknowledgement, and user-facing error.
- Rectangle capture requires explicit user activation. Unrelated/background collection, browsing-history collection, whole-page polling, and automatic submission remain prohibited. Only the separately enabled M5 answer modes authorize bounded foreground application-field observation/filling under the feature authority.
- The overlay is the sole review surface: it shows the returned text and sanitized URL and lets the user edit, remove the URL, capture again, or continue.
- URL fragments and known sensitive/tracking query parameters are removed; the user may remove the URL entirely.
- A new capture cannot silently merge with a different active application workspace.

## Native messaging and local IPC

M5 qualifies functional Chrome/Edge integration with the clearly identified
current-user development app/host; paid Apple app signing is not required before
M7. M7 qualifies signed production desktop/helper identity, identity-scoped vault
access, final Store identities/publication and Store-installed integration.
Authenticated messages, exact origins, permission/capture authorization, bounds,
replay rejection and safe failure remain requirements at both stages. Testing
Store-ready extension code against the development app uses an explicit test
configuration, never an automatic fallback from production to development.

- The native host validates the exact extension origins, protocol version, action, message identifier, schema, size, and freshness.
- Production manifests allowlist the published Chrome and Edge extension identifiers; wildcard origins are prohibited.
- Development and production use different identifiers, host names, manifests, and IPC endpoints.
- The native host communicates with the desktop through a per-user protected mechanism such as a Windows named pipe and macOS Unix-domain socket or equivalent.
- The IPC handshake uses operating-system access control and an installation-scoped secret or equivalent defense so another local process cannot impersonate the extension bridge casually.
- Messages are bounded and expire quickly. Repeated control commands are idempotent; captures have session/replay checks and unconfirmed payloads are never automatically resent. The implemented browser contract is protocol v1; a broader production compatibility window remains planned.
- The bridge contains no AI key and retains no permanent resume or application content.
- Planned answer messages negotiate protocol v2 while preserving v1 capture. Older peers report autofill unavailable; no broader fallback is permitted. Apply field/document/configuration/answer-revision binding, expiry, and revocation to lookup/capture/approval results, with no durable browser content queue.

Exact message-size, timeout, retry, protocol-version, and compatibility values belong in implementation planning and must remain within browser-native-messaging constraints.

## Planned production desktop launch behavior

The current development host reports an unavailable desktop without launching
it. Open ORT manually. The following launch/repair behavior remains release work.

If the app is not running, the native host may launch it through a platform-approved mechanism and complete a time-bounded handshake. If launch or connection fails, the extension provides actions to:

- Open ORT manually
- Verify that the desktop app is installed
- Repair the browser connection
- Update an incompatible component

It must not repeatedly spawn processes or silently discard the reviewed capture.

## Planned production installation and repair

Current development registration is explicit through the checked-in setup
helpers; it is not a shipped automatic installer or Browser connections screen.

- The desktop installation places the native host and manifest in controlled locations.
- Normal installation or first-run setup registers per-user Chrome and Edge native hosts without requiring manual registry or filesystem edits.
- Microsoft Store MSIX packaging may require a first-run registration/verification flow and specific registry-virtualization configuration. This must be proven during implementation and Store certification.
- The app provides a **Browser connections** screen showing installed, connected, incompatible, or repair-needed status for each browser.
- Updates atomically preserve or replace the bridge and manifest. Version mismatches fail closed and show repair guidance.
- Uninstall or a dedicated disconnect action removes ORT-owned registrations without changing unrelated browser settings.

## Permissions

- Use `scripting`, `nativeMessaging`, `alarms`, and declared HTTP/HTTPS host access for the implemented desktop-owned capture flow. An overlay click cannot grant `activeTab`; its current capture has no toolbar/shortcut path. The planned answer popup is a separate approved surface, not a replacement capture workflow.
- Host access supports user-chosen job sites. Injection requires a bounded authenticated capture session or, for planned autofill, a fresh foreground monitor authorization under an explicitly enabled mode. Installing/granting host access alone never authorizes reading. Users may restrict browser site access.
- Current Store disclosures describe two-corner capture. Before shipping autofill, update them for foreground question/answer processing, website disclosure at fill time, modes, popup decisions, exclusions, and retained desktop data. Restricted pages remain unavailable; no permission workaround is offered.
- The current extension has no persistent storage, startup content scripts, cookies, browsing-history access, remote code, or external network calls. Planned autofill retains no durable answer database; transient bound field data is cleared/revalidated on revocation or restart.
- Extension source is published with the desktop source, and store packages must be traceable to repository revisions.

## Chrome and Edge variants

The extensions should share one codebase where practical while retaining distinct published identifiers. The native-host `allowed_origins` list includes both exact identifiers. Store review, update timing, and version skew are handled independently.

## Failure behavior

- Repeated controls are handled idempotently; duplicate/replayed capture delivery cannot overwrite another session.
- Oversized, stale, malformed, unrecognized, or unauthorized messages are rejected before desktop processing.
- If delivery is rejected or unconfirmed, capture controls are removed and no text is resent automatically. Existing desktop fields stay available; inspect them before capturing again. No content is queued across worker restarts.
- No extension failure may expose local database paths, secrets, stack traces, or unrelated application content.
