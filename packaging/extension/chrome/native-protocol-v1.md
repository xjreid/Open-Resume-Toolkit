# Chrome extension ↔ native host protocol v1

This is the implemented browser-facing contract for Chrome 0.3.0. M8's signed
native transport must preserve it so the browser implementation remains usable
without an authentication-specific update. Native-host/desktop authentication
is internal and is never performed by page scripts or the extension.

## Connection and bounds

- Host name: `com.openresumetoolkit`; the separate BETA uses `com.openresumetoolkit.dev`.
- Chrome `runtime.connectNative()` port; browser-defined 32-bit native-endian
  byte-length framing around UTF-8 JSON. The currently supported Mac is little-endian.
- Multiple serialized request/response pairs per port. Exactly one response per
  request, in order; no unsolicited responses. The production host must support
  this persistent connection, not terminate after every poll.
- Protocol version 1 on requests/responses; incompatible versions fail closed.
- Maximum request/response 256 KiB; text 128 KiB; sanitized URL 4 KiB;
  title at most 500 characters. No provider credentials or local file paths.
- Five-second request deadline; at most eight queued native requests. A timeout
  or disconnect invalidates all queued work. Capture is never automatically retried.
- Live content-free poll every 500 ms after the previous poll completes. The
  worker releases unavailable connections and uses a 30-second Chrome recovery
  alarm. Chrome may delay it. Host startup/status must not repeatedly launch ORT.

## Content-free requests

`bridge.status`: `{ "protocolVersion": 1, "kind": "bridge.status" }`.

`bridge.poll`: `{ "protocolVersion": 1, "kind": "bridge.poll", "clientId": "UUID" }`.

Successful poll:

```json
{"ok":true,"protocolVersion":1,"value":{"ready":true,"commands":[]}}
```

`ready` means an authorized desktop can receive captures. `commands` contains
at most two repeated/idempotent commands:

```json
{"kind":"capture.start","sessionId":"UUID","target":"job","expiresAt":1791320000000}
{"kind":"capture.cancel","sessionId":"UUID"}
```

`target` is `job` or `question`; `expiresAt` is integer UTC epoch milliseconds,
in the future and no more than 120 seconds away. Desktop creates the session,
owns the destination and revokes it before issuing cancellation. Only one live
browser client may own a desktop capture session. Unknown commands cannot arm
capture. No page scripts are injected by status/polls without a valid start.

`capture.event` contains protocolVersion, kind, clientId, sessionId, and phase
(`started`, `selecting`, `cancelled`, `failed`). Failed events may contain only
these safe codes: PAGE_UNAVAILABLE, PAGE_CHANGED, EMPTY_SELECTION,
CAPTURE_TOO_LARGE, CAPTURE_EXPIRED, CAPTURE_INVALID, BRIDGE_UNAVAILABLE,
DELIVERY_UNCONFIRMED. Event success is `{ "ok": true, "protocolVersion": 1 }`
with optional native version/value metadata. It authorizes no AI operation.

## Content delivery

```json
{
  "protocolVersion": 1,
  "kind": "capture.selection",
  "requestId": "desktop-session-UUID",
  "sentAt": "2026-10-06T21:00:00Z",
  "payload": {
    "text": "User-selected job text",
    "url": "https://example.test/job?jobId=42",
    "title": "Job page",
    "browser": "chrome",
    "target": "job"
  }
}
```

The extension validates the exact active tab, main-frame/document ID, URL and
sender before delivery. Desktop independently validates session, client/origin,
target, freshness (60 seconds), size and replay protections before encrypted
intake. Page content cannot select an application workspace or invoke AI.

Acceptance MUST include the same request ID:

```json
{"ok":true,"protocolVersion":1,"value":{"requestId":"desktop-session-UUID"}}
```

Bare `ok`, a mismatched ID, missing response or incompatible version never proves
delivery. On uncertainty the page controls are removed and no capture is resent.

## Failures and production boundary

Native failures use `ok: false`, protocolVersion, and a bounded `error` with a
safe code/retryability. No stack trace, database path, captured content or secret.
Representative errors: BRIDGE_UNAVAILABLE, SIGNED_BRIDGE_REQUIRED,
PROTOCOL_INCOMPATIBLE, BROWSER_BUSY, STORAGE_UNAVAILABLE, CAPTURE_EXPIRED,
CAPTURE_INVALID and CAPTURE_TOO_LARGE. Connection failures never enable capture.

Exact extension origins are allowlisted by the host manifest and native binary.
For explicit development testing, a locally registered host with the production
name relays to the current-user development socket. For M8, the signed host uses
verified native identities and a scoped vault secret instead. No authentication
secret crosses this browser contract. Native version/capability metadata may be
extended compatibly; a breaking change needs deliberate protocol/version rollout.
