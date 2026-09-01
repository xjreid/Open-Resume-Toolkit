# Browser extension and local IPC plan

## Status and requirements

- Status: approved baseline; Store/package registration probes required
- Owner: browser/IPC maintainer
- Milestone: M5 and release hardening in M7
- Product authority: `../../Product Plans/Desktop_Extension_Communication.md`, `Security_Privacy_and_Open_Source.md`, and `Configuration_Limits_and_Defaults.md`

Non-goals: scraping an entire page, running automatically, modifying job sites, collecting browsing history, calling AI providers, or operating without the desktop application/native host.

## Extension architecture

One TypeScript codebase produces Chrome and Edge Manifest V3 packages. Browser-specific checked-in templates provide extension name, Store ID, icons later supplied by the aesthetic plan, and native-host allowlist. Production, preview, and development IDs are distinct.

Modules:

```text
content/selection.ts        reads current selection only after service-worker request
background/capture.ts       coordinates user gesture, confirmation, native message
popup/                      status, editable review, Send/Cancel, safe errors
shared/contracts.ts         generated schemas and protocol negotiation
shared/url.ts               URL normalization/sanitization
manifest/*.json             browser/channel templates
```

The service worker is the authority. Content scripts are treated as exposed to hostile pages and cannot talk to the native host directly.

## Permissions

Target permissions:

- `nativeMessaging` for the native host;
- `activeTab` and `scripting` for explicit selection capture after the extension action/shortcut supplies the browser user gesture;
- `contextMenus` only if the approved capture UX uses it;
- no broad host permissions;
- no browsing-history, tabs history, clipboard, downloads, identity, or persistent storage permission beyond small non-content settings.

Use `optional_host_permissions` only for an explicitly enabled overlay-initiated convenience path and scope the grant to sites the user approves. Do not request `<all_urls>` by default. Store disclosures describe the exact selected-text flow. A permission expansion requires product/security review.

## Capture flow

1. From overlay Stage 1 or the Answers tab, the user selects page text. The overlay's Capture control arms/explains the request; in the default least-permission path the user invokes the extension toolbar action or registered keyboard shortcut to provide the browser-required gesture.
2. Service worker requests the current selection from a minimal content script.
3. Content script returns plain text and the current document URL/title; HTML is discarded.
4. Service worker normalizes line endings/Unicode, enforces the 128 KiB captured-text limit, and sanitizes the URL by removing credentials, fragments, and known tracking parameters.
5. Service worker opens a native-messaging connection and transmits the bounded capture immediately to the authenticated desktop recipient. The extension has no competing editable review or tailoring UI.
6. The overlay shows editable text/URL with Capture again and Continue for job text, or review and Generate answer for question text.
7. Desktop returns accepted/rejected/version status; the extension shows only compact bridge status/error feedback and clears content from memory.

No content is written to extension local/sync storage. If the desktop is unavailable, the extension may keep the current payload only while the confirmation/native-call page is alive; it does not queue it for later.

Restricted browser pages, empty selections, over-limit content, invalid URLs, and unsupported encodings produce local actionable errors.

### Optional overlay-initiated capture feasibility gate

An optional direct response to the overlay Capture button may use a bounded `runtime.connectNative()` session so desktop can signal the extension, but only when all of the following are proven in Chrome and Edge:

- the user explicitly granted a narrow optional site permission that permits script access without a fresh extension action;
- the native session is authenticated, short-lived, reconnectable, and does not keep the Manifest V3 service worker alive indefinitely;
- no default broad host access is added and store disclosures remain accurate;
- failures fall back to the extension action/shortcut flow without losing reviewed overlay state.

Until those gates pass, the toolbar/shortcut gesture flow is the shipped behavior even though capture is initiated conceptually from the overlay workflow.

## Native-messaging contract

Chrome/Edge native messaging uses JSON messages over `stdin`/`stdout` with the browser-defined 32-bit length framing. `stderr` is diagnostics only and must be redacted. The host manifest contains an absolute executable path, `stdio` type, and exact `allowed_origins`; wildcards are forbidden.

Request envelope v1:

```json
{
  "protocolVersion": 1,
  "requestId": "uuid",
  "sentAt": "RFC3339 UTC",
  "kind": "capture.selection",
  "payload": {
    "text": "...",
    "url": "https://example.test/job",
    "title": "Optional title",
    "browser": "chrome"
  }
}
```

Response is the common `ok/value` or `ok/error` shape with desktop/host/protocol versions and retryability. Unknown fields are tolerated only for additive minor versions. Unsupported major versions return `PROTOCOL_INCOMPATIBLE` and a repair/update action.

Limits are lower than browser maximums: 256 KiB total envelope, 128 KiB captured text, 4 KiB URL, bounded JSON depth/fields, one request/response in the default host process, a five-second capability-acknowledgement target, and one desktop-launch attempt of up to 15 seconds. A separately negotiated bounded session is allowed only for the optional overlay-initiated feasibility path.

## Native host

`ort-native-host` is a memory-only Rust adapter using the generated contract crate. Startup receives the browser origin information the platform supplies and validates it against compiled production/preview/development IDs. It reads exactly one frame, validates it before desktop launch, and writes exactly one response.

If desktop is not running, the host launches the known installed executable with a content-free activation flag, waits for the protected IPC endpoint, forwards the in-memory request, and exits. It never puts page text/URL in arguments, environment, a temporary file, logs, or a crash report.

Executable location is resolved relative to the installed host package/registered manifest and validated against the expected application identity; an extension request cannot choose it.

## Authenticated local IPC

Transport:

- Windows: local named pipe with current-user SID ACL and remote clients disabled;
- macOS: Unix-domain socket inside an application-owned `0700` directory with `0600` socket mode.

Desktop generates a 256-bit installation secret stored in the OS credential vault. Installer/first-run host setup gives the host access through the same user vault entry; the secret never appears in the native-host manifest.

The platform implementation must prove that only the intended desktop and native-host identities receive this IPC entry while database and provider entries remain desktop-only. On Windows, the HMAC secret supplements named-pipe ACL/origin/replay checks but does not claim to resist arbitrary malware already running as the same user, because Generic Credential data is available inside that user boundary. On macOS, signed builds authorize both code identities using a reviewed Keychain ACL/code requirement or access group. If an unsigned preview cannot share one Keychain item without a weaker fallback or misleading prompts, browser integration is disabled for that preview package; the secret is never copied into a plaintext permissions-only file.

Handshake:

1. host connects and sends protocol versions, installation ID, origin ID, request nonce, and client random;
2. desktop verifies endpoint peer identity where the OS supports it and replies with server random/challenge;
3. both authenticate the transcript with HMAC-SHA-256 using the vault secret;
4. host sends the capture envelope with sequence and 60-second expiry;
5. desktop checks HMAC, origin, size, expiry, and replay cache before creating a capture intent.

Message length precedes each authenticated frame. Failed authentication receives no detailed oracle. The socket accepts only the native-host request family; it is not a general desktop API.

## Desktop delivery behavior

Desktop copies accepted capture data into the sole current overlay workspace and returns its ID. It raises/focuses the overlay according to OS policy and shows Stage 1 job review or Answers-tab question review. AI does not start automatically.

If another capture arrives while review is active, the overlay asks whether to replace the pending capture; it never silently overwrites accepted job text or another tab's work. Pending intents are encrypted only after authenticated receipt and remain subject to the one-workspace retention rules.

## Installation and registration

### Windows direct installer

NSIS installs the host executable and per-user native-host manifests, registers Chrome under the documented HKCU native-messaging key and Edge under its documented HKCU key, and substitutes the absolute manifest path. Repair re-verifies files, registry values, ACLs, extension IDs, and versions.

### Windows Store fallback

Before committing to MSIX, build a proof package verifying whether Store identity/virtualization permits the required registry/native-host registration and update persistence. If it cannot meet the product contract, the Store build must disable extension integration and the website/store listing must say so; do not claim repair support that cannot work.

### macOS

The DMG cannot rely on installer scripts. On first user-initiated **Connect Browser** action, the desktop writes manifests in the per-user Chrome and Edge NativeMessagingHosts locations with the absolute current application/host path and user-only permissions. It repeats verification after the application is moved and offers in-app repair. Because the initial app is unsigned, packaging/install instructions must test quarantine and browser launch behavior explicitly.

Uninstall removes host binaries/manifests/registry entries owned by the exact product/channel identity but not unrelated browser data. Normal desktop uninstall preserves local ORT data according to the product plan.

## Versioning and rollout

Desktop, host, and extension publish a compatibility tuple. The protocol supports current and previous major during staged Store rollout when safe. A newer extension cannot send unsupported behavior; an older extension receives an upgrade/repair instruction.

Release sequencing:

1. ship desktop/host that accepts the new additive protocol;
2. publish Chrome/Edge extension update;
3. after Store adoption, remove old protocol only in a later desktop release.

Emergency desktop disablement is local capability-based; the extension itself still collects nothing until the user invokes it.

## Security tests

- malicious page mutates DOM/selection during capture;
- page tries to impersonate extension messages;
- HTML/script/control characters and deceptive Unicode;
- credentials/fragments/tracking values in URLs;
- oversized/truncated/negative-length/deep JSON frames;
- unapproved extension origin and development/production ID mix-up;
- forged HMAC, replay, expired request, wrong install ID, local cross-user connection;
- unrelated same-user process and native-host attempts to read database/provider vault entries; other-user attempts to read the IPC entry;
- symlink/reparse/registry path substitution;
- desktop absent, slow start, crash mid-request, two simultaneous captures;
- host/stdout contamination and seeded-data log scan;
- install/repair/update/uninstall on both browsers and every release channel.

## Accessibility and privacy tests

Extension action/status and errors are keyboard/screen-reader operable at 200% scaling. The overlay review owns the labeled edit/accept controls and announces receipt without trapping focus. Privacy tests inspect browser storage after success/failure/restart and verify no captured content remains.

## Completion criteria

- A deliberate selection can reach desktop review on Chrome and Edge/Windows and macOS.
- No tested passive navigation/selection event creates storage or native traffic.
- Unauthenticated, replayed, expired, malformed, oversized, or wrong-origin messages cannot create a workspace.
- Installation, repair, update, version skew, and uninstall have clean-machine evidence per supported channel.
- The extension contains no provider credentials, AI logic, telemetry, or persistent job content.
