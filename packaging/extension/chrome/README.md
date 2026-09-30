# Chrome packaging

Chrome's Manifest V3 package is built from `apps/extension/manifest/chrome-store.json`.
See `apps/extension/README.md` for build/test commands and current limitations.

The isolated **unsigned macOS BETA connection is now implemented**. Follow
[`development-testing.md`](development-testing.md) to test it immediately and
create a private Chrome dashboard item. The prerequisites below apply to the
production channel.

## Draft upload and identity

Run `pnpm --filter @ort/extension package:chrome`. Upload the resulting
`artifacts/extension/chrome/open-resume-toolkit-chrome-0.2.1.zip` through the Chrome
Web Store Developer Dashboard's Add new item flow, and leave the item as a draft.
This provides the final Chrome extension ID before enabling native integration.
Record that ID and the public key from the package page. A public extension key
is safe to use for matching an unpacked test installation's identity; never put
private signing material in the extension.

The package uses the stable native-host name `com.openresumetoolkit`.
The explicit development BETA package uses `com.openresumetoolkit.dev`. Desktop/native
host channel identities must agree; the store extension does not fall back to a
development host. `ORT_CHROME_EXTENSION_ID` is the compile-time store-origin input
to the native host. Development origins have separate existing compile-time
variables. No origin is enabled by default, and origin checks precede frame reads.

## Desktop prerequisites before submission

1. Implement and qualify protected desktop/native-host transport: exact process
   identities, installation-scoped vault authentication, expiry/replay checks,
   bounded framing, and no content logs or plaintext secret fallback.
2. Sign the macOS app and native host and prove the narrow shared Keychain access.
   Default builds remain gated; the explicitly opted-in development BETA uses a
   separate, current-user capability and cannot qualify this production gate.
3. Implement user-initiated Connect/Repair/Disconnect in Browser connections,
   registering an absolute host path and exact store extension origin.
4. Deliver captures through `accept_authenticated_capture`, emit
   `ort:browser-capture`, and focus desktop review. Check offline launch,
   version mismatch, and pending-capture conflicts on the supported macOS path.
5. Run a real Chrome → installed host → desktop review walkthrough with the final
   extension ID. The development browser test exercises real native delivery in a disposable
   profile; installed GUI and production identity still require this gate.

Signing credentials alone do not complete these prerequisites. Other browsers
and Windows integration remain separate milestones.

## Native response contract

A persistent native port polls `bridge.poll` with a client UUID. The desktop
returns repeated idempotent start/cancel commands for its own capture session.
`capture.event` reports first-corner progress or safe failure codes. Cancel
revokes desktop authority before browser cleanup. Only the matching active
selecting session and target may submit `capture.selection`. Poll/event messages
are strict and content-free.


Content-free readiness request:

```json
{"protocolVersion":1,"kind":"bridge.status"}
```

A connected host returns `ok: true`, protocol version 1, and `value.ready: true`
only after confirming its authenticated desktop capability. Capture responses
return `value.requestId` matching the request UUID on acceptance. Failure responses
return `ok: false` and a safe error code such as `CAPTURE_PENDING`,
`PROTOCOL_INCOMPATIBLE`, or `BRIDGE_UNAVAILABLE`. A default host without the production transport answers the
probe with `SIGNED_BRIDGE_REQUIRED` and `value.ready: false`; it never forwards
captures. The extension doesn't infer delivery from a bare `ok: true`.

## Store submission materials

Prepare the listing description, screenshots, support contact, public source
release, privacy policy URL, and review instructions for installing the supported
desktop app. The listing must say that the desktop app is required and explain
the supported operating system and visible-text rectangle capture limitations.

Permissions to justify in the Privacy tab:

- HTTP/HTTPS host access: overlay clicks cannot grant `activeTab`; access enables
  injection only after the user arms capture in the desktop overlay.
- `scripting`: render the two-corner box and live highlights, support scrolling, and read
  the enclosed rendered DOM text in the
  current main frame, in Chrome's isolated world.
- `nativeMessaging`: send the bounded capture to the locally installed ORT host.

Use `privacy-policy-draft.md` as an implementation-specific starting point, verify
it against the finished desktop bridge, and publish it at an accessible URL before
submission. Local processing still requires a data-handling disclosure.

Begin with Private visibility and designated trusted testers. Private distribution
still requires store review. Obtain approval and test the store-installed package
before switching to Public. No package has been uploaded or submitted by this task.
