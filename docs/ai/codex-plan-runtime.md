# Codex runtime qualification

ORT supervises an optional, separately installed official Codex app-server.
The Codex page offers an explicit runtime installer after the main app
is installed. ORT never installs or updates it automatically, adopts the
desktop Codex app's credentials, or falls back to a paid API key after failure.

## Qualified release

The compatibility manifest is
`apps/desktop/src-tauri/codex-compatibility.json`. Only the macOS Apple Silicon
executable from the [official Codex 0.162.0 release](https://github.com/openai/codex/releases/tag/rust-v0.162.0)
is accepted. Its executable SHA-256 is:

```
8238273076041e55beded03ba1d51e034f997b7d100b0642d36daf036620fe57
```

Runtime discovery requires an exact digest, executable access for the app,
a canonical path without symlinks,
and root ownership with no group/world write permission for the executable and
its containing directories. A user-writable application bundle is intentionally
rejected. These checks establish official release provenance without relying on
an invalid or absent release-binary code signature.

The recommended protected location is:

```
/Library/Application Support/Open Resume Toolkit/Codex/codex
```

Download `codex-aarch64-apple-darwin.tar.gz` from the release above, extract it,
and verify the executable digest with `shasum -a 256`. An administrator can
then install the verified executable (substitute the actual extracted path):

```sh
sudo install -d -o root -g wheel -m 755 "/Library/Application Support/Open Resume Toolkit/Codex"
sudo install -o root -g wheel -m 755 /path/to/codex-aarch64-apple-darwin "/Library/Application Support/Open Resume Toolkit/Codex/codex"
```

Return to Codex or choose **Check installation**. ORT also checks the fixed
CodexCLI application locations recorded in `codex_runtime.rs`; the same digest
and protected-directory rules apply. Other releases require a new qualification
and compatibility-manifest update; arbitrary PATH executables are never used.

## Installation readiness

Opening the Codex page or returning focus to it runs a main-window-only,
offline installation check, even with Enable Codex off. It checks the protected
locations, permissions, executable access, supported architecture, size, Mach-O
header and pinned digest on a background thread. It does not start Codex,
connect an account, read credentials, enable the provider or send network requests.
Live authentication, network access and runtime policy are checked only after
the user enables Codex.

While checking, Enable Codex cannot be switched on. Missing or rejected runtimes
show the explicit install action and its optional details. A verified runtime
hides all installation guidance, download controls and old installer messages;
when disabled, the page presents Enable Codex before any account controls.
Successful installation checks readiness again without enabling or connecting.
Already-enabled users can still disable Codex if the runtime becomes unavailable.
The saved preference and API-key restoration behavior remain unchanged.

## Optional development installer

The main app installer remains unchanged. In **Codex**, choose **Install
Codex runtime**. First installation requires macOS administrator approval;
installation does not sign in, enable Codex, or change other Codex installs.

The compiled compatibility manifest pins all of the following:

- Exact official GitHub release URL and macOS ARM64 archive entry.
- Archive length **98,089,521 bytes** and SHA-256
  `98e9413a1bd167ae573723939ff178e6d44f1bb21d5cda960dbeb2fa65f49104`.
- Executable length **246,508,160 bytes**, ARM64 Mach-O executable header, and
  the previously qualified executable digest above.

Downloads use certificate-validated HTTPS, no inherited proxies, no credentials,
fixed host restrictions, a bounded redirect count, exact Content-Length, streamed
size limits, connection/body deadlines, and an overall 180-second deadline.
Cancellation interrupts network waits. A private temporary directory holds the
archive. Its full checksum is checked before gzip/tar parsing. The parser permits
one exact regular-file entry, validates its tar checksum, rejects links, devices,
prefixes and extensions, and checks bounded zero-only padding/trailers. It never
extracts paths supplied by an archive.

The payload is verified again without executing it. Installation never starts a
Codex process, including when Enable Codex is off. Only the enabled, supervised
runtime owner can launch Codex after installation. macOS requests approval after
the archive and payload checks succeed.

A fixed bootstrap runs through the system Perl with a cleared environment and
taint mode. It opens the staged helper with no-follow/non-blocking flags, rejects
non-regular, hard-linked or oversized inputs, and copies from that descriptor
into a private root-owned directory. It verifies the copy against the compiled
helper digest before executing only that protected copy. Replacing the
user-owned helper while approval is pending cannot change the executable run as
root. Arguments cross the AppleScript boundary as quoted literals; the protected
helper is invoked without shell evaluation.

A separately packaged, checksum-bound **ORT Codex Installer** helper then performs
the privileged payload copy. Its only input is the staged payload path. Its destination,
version and digest are compiled in. It has no network, shell, subprocess, model,
configuration, credential or general installation functions. Payload opens use
no-follow/non-blocking flags and reject non-regular files, hard links and wrong
sizes. The helper checks protected root-owned parent directories from the root
down, refuses links and writable paths, copies from an open descriptor into a
private root-owned file, verifies that copy independently, syncs it, sets 0755,
and replaces the single destination by atomic rename. Verification and copy
failures before that rename preserve any existing runtime. A post-commit I/O or
verification failure asks users to refresh and check the installation before retrying.

The application verifies the installed path, permissions and digest without
starting it. Once administrator approval starts,
finish/cancel the macOS prompt; the app does not claim it can interrupt an atomic
protected copy. Temporary files are removed when the operation completes. There
is no persistent privileged service. A five-minute approval timeout is recoverable
by refreshing the runtime status.

Development packaging uses ad-hoc signatures and is not notarized. It assumes
an untampered ORT application and explicit OS authorization; checksums do not
protect a compromised user account or operating system. Public delivery requires
production signing, notarization and a separate installer security review.

Build the complete development app without installing or launching it:

```sh
python3 tools/build-development-app.py
```

The helper's embedded digest is supplied to the desktop build from its verified
package manifest. A desktop built without the helper/digest refuses installation.
The helper accepts no destination or checksum overrides and rejects non-root
callers. Installing the Codex runtime remains a separate user-initiated action.

Installer verification includes 10 offline copy-boundary tests for tampered and
changing inputs, bounded reads, links/special files, unsafe destinations, write
failures, repeatable atomic replacement, and preservation of an existing runtime.
The desktop suite checks the download allowlist, response limits, cancellation,
archive rejection, and literal AppleScript arguments without requesting elevation.
Component and generated-contract tests cover opt-in dispatch, progress, recovery,
operation locks, readiness after completion, and malformed native status. Eight
fixture captures at 1080×760 and 720×520 passed overflow and visual review.

The explicit `official_download_verifies_without_administrator_or_account_access`
test passed using the real HTTPS client, both pinned hashes, the strict parser,
and contained signed-out startup. The actual administrator prompt and root copy
remain user-initiated checks; automated tests do not install the runtime or connect
an account.

## Evidence recorded October 8, 2026

The exact release executable was downloaded to a temporary qualification
location and its digest verified. The real `app-server --listen stdio://`
completed initialization, non-experimental model discovery and signed-out
`account/read` under the production containment policy. The offline
`runtime_qualification` test passed. No user credentials, browser sign-in,
provider inference, or plan usage were used by qualification.

The independent `os_containment_qualification` test compiled a small probe and
executed it under the same default-deny policy. It confirmed scratch writes
work, while outside-file reads/writes, fork, shell execution, and other local
network destinations are denied. Only the supervised gateway port was reachable.
Both qualification tests are explicit/ignored during ordinary automated runs:

```sh
ORT_CODEX_QUALIFY_PATH=/temporary/verified/codex cargo test --locked -p ort-desktop runtime_qualification -- --ignored
cargo test --locked -p ort-desktop os_containment_qualification -- --ignored
```

Generated protocol schemas and [managed authentication/quota documentation](https://learn.chatgpt.com/docs/app-server#auth-endpoints)
were checked against that release. Browser login/logout, renewal, turn results,
quota changes, cancellation, retry events, malformed messages and unexpected
tools are tested using deterministic local stdio responses. Successful live
login/inference remain user-initiated checks through an explicitly connected
account; offline qualification does not claim to prove account-specific model
access or quota availability.

## Containment and confidentiality

Every ORT profile has a separate canonical `CODEX_HOME`. Codex manages ChatGPT
login and renewal using `cli_auth_credentials_store = "ephemeral"`: authorization
stays in its supervised process's memory, with no Keychain or credential-file
fallback. ORT checks effective `config/read` settings before account commands
and blocks a managed-policy override requiring persistent storage. Existing
persistent Codex credentials are not adopted. ORT stores only
an opaque activity connection ID and non-secret settings. The sandbox grants no
credential-database file access or Keychain authorization rights. Native TLS
certificate validation still requires the existing SecurityServer/trust services;
the ephemeral credential backend does not call the Keychain store.

**Enable Codex** is a saved, profile-scoped runtime permission and exclusive AI
provider choice, independent of sign-in. It is always visible on the Codex page,
including before login or runtime installation. Disabled status reads do not
start Codex. Turning it on from the page first requires successful offline
installation verification; an already-enabled preference remains available to
turn off if installation becomes unavailable. Login and workflow session acquisition recheck permission under the
runtime lock, so an older queued poll cannot recreate a disabled runtime.

While enabled, every AI request uses Codex. This remains true while signed out:
the overlay identifies Codex and replaces model/reasoning controls with
“AI is disabled until an account is connected.” My Keys is grayed out and noninteractive with
“Codex must be disabled to use API keys.” Native key mutations, tests and spending
changes are also blocked. Enabling does not pause or change the selected API key.
Disabling restores that key immediately; manual key pauses remain intact. Older
Codex-created key pauses migrate away without changing the selected key.

Quitting ORT, replacing/switching profiles, cancellation that stops the runtime,
or a runtime failure ends memory-only sign-in. The enabled preference, model,
reasoning, reserve and activity history remain saved. On relaunch an enabled
profile can start the runtime for status checks, but the user must sign in again.
Each new signed-in process receives a fresh activity identity.

The bottom **Sign out** control keeps Codex enabled, cancels current Codex work,
and destroys the old process and its memory-only
authorization. Later status checks may start a fresh signed-out process because
permission remains enabled. Unchecking **Enable Codex** also signs out and stops
the process, and prevents all future runtime starts until reenabled. It can stop
an active Codex operation. Neither action requires starting a missing/broken
runtime to forget credentials. Managed-directory cleanup failure remains blocked
and recoverable through **Retry sign-out**.

The macOS sandbox denies process forks and execution of anything except the
verified runtime, and allows file content only in the private auth/scratch roots
plus required system libraries. The runtime gets a cleared environment and
explicitly restrictive config; no inherited API key, user Codex config, MCP,
plugin, agent, web, shell or file tools. An unexpected tool/permission event
terminates the operation. Threads are ephemeral, history is disabled, stderr
is discarded and runtime tracing is disabled. Raw prompts and responses are
not monitoring, diagnostic, or backup records.

All provider egress crosses a bounded local CONNECT gateway. It accepts only
TLS port 443 for `auth.openai.com`, `chatgpt.com` and `api.openai.com`; ClientHello
SNI must match the requested host, and DNS destinations must be public IPs.
The gateway does not decrypt or record provider traffic. The runtime receives
explicit HTTP/HTTPS proxy environment variables pointing only to this gateway.
Its `respect_system_proxy` feature is disabled: in Codex 0.162.0, macOS system
discovery can select a DIRECT route ahead of those variables, which the sandbox
then denies. The sandbox remains the enforcement boundary even if a transport
tries to bypass the proxy. Login may bind only the documented localhost callback
port 1455.

Login failure notifications are mapped locally to fixed transport, token exchange,
credential-store, workspace restriction, declined/cancelled, or unknown-failure
codes. Raw provider errors, authorization codes and callback URLs are not retained
in monitoring or logs.

The gateway explicitly resets accepted sockets to blocking mode before applying
read/write deadlines. macOS can inherit nonblocking mode from its listener; that
previously closed the tunnel before the client sent its post-CONNECT TLS flight.
An offline delayed-flight regression reproduces that condition on all platforms
and confirms malformed TLS is still rejected before any upstream connection.
The public authentication-endpoint TLS check failed before this correction and
passed afterward. The real contained 0.162.0 runtime also received the expected
HTTP 401 rejection for a deliberately invalid OAuth code, proving token-exchange
transport without completing sign-in, opening a browser or using account credentials.
Both network checks are explicit/ignored in ordinary runs:

```sh
cargo test --locked -p ort-desktop authentication_tls_through_gateway_qualification -- --ignored
ORT_CODEX_QUALIFY_PATH=/temporary/verified/codex cargo test --locked -p ort-desktop oauth_token_transport_qualification -- --ignored
```

The OAuth transport check refuses to run while either documented callback port is
occupied, so it cannot intentionally cancel another pending sign-in. Successful
account authorization still requires a user-initiated login.

## Memory-only authentication qualification

The default-deny sandbox is incompatible with the pinned runtime's legacy
Keychain credential backend. A disposable credential test reproduced a missing
keychain recovery prompt; no real credentials or Keychain settings were changed.
On October 8, 2026 the user explicitly selected memory-only login. No raw Keychain
database access or authorization-right exceptions were added.

The explicit `memory_auth_runtime_qualification` test runs the pinned official
executable under the production sandbox and configuration, changing only the
forced login method to permit a synthetic API-key storage probe. There is no
reachable provider gateway, browser, Keychain storage, inference or real account.
It verifies same-process authentication, logout, authentication loss after
process replacement, absence of `auth.json`, and rejection of a pre-existing
synthetic credential file. Managed OAuth uses the same ephemeral storage after
token exchange. Ordinary tests cover reconnect identity, retained preferences,
explicit enabling, profile isolation, ended-session messaging and no API fallback.

```sh
ORT_CODEX_QUALIFY_PATH=/temporary/verified/codex cargo test --locked -p ort-desktop memory_auth_runtime_qualification -- --ignored
```

An operation freezes connection, model, reasoning and inputs. Each pass has a
120-second total deadline, including runtime acquisition/startup. Cancellation
also interrupts startup and waiting for the runtime lock. Authorization is
revalidated and quota fetched before every pass; a quota read has a 10-second
deadline. Every reported window must meet an enabled reserve. Equality allows
dispatch; missing or invalid data blocks it. The reserve cannot stop consumption
already in flight or consumption in other apps.

Reported per-thread cumulative tokens are replaced, never summed, across events.
Each thread contains one ORT pass, so thread totals include that pass's internal
response cycles and are not account lifetime totals. Cached input is separated
from ordinary input; reasoning is a subset of output. Missing post-dispatch
usage remains unknown. Internal retry notifications are reported as such,
without claiming an underlying HTTP request count. Plan monetary cost is always
$0 with `not_tracked` provenance and does not consume dollar-spending caps.

## Overlay settings and naming

The main app names this connection **Codex**. The fixed overlay banner uses
**API key** or **Codex** above its model control. Codex exposes model and
reasoning selectors there, with unsupported choices disabled. Overlay saves
change only model and reasoning; connection enablement and usage reserves remain
main-window controls. The shared model-change event reloads the latest settings
in both windows without waiting for a quota refresh. Polls already in flight
cannot overwrite newer selections.

Known account-wide quota windows appear as remaining percentages in the banner;
unknown quota is omitted. A failed or invalid refresh clears the previous
snapshot rather than showing stale percentages as current. Usage refreshes every
30 seconds while the overlay is
open and after an AI operation completes. The API key model selector continues
to use its existing command and synchronization event.

## Responsiveness and protocol review — October 8, 2026

Settings saves use a non-secret, profile-scoped runtime snapshot. Ordinary local
status reads do not force token renewal; authorization and quota remain fresh
checks before each pass. UI polls skip busy protocol ownership and are coalesced
by one React controller. A background refresh can be interrupted by disconnect.
Ephemeral threads are explicitly unsubscribed on every nonfatal exit after
creation, including model mismatch, accounting failure and turn-start rejection,
rather than accumulating for the lifetime of the memory-only session.

Stdio writes use non-blocking pipes with a deadline established before dispatch.
Cancellation is checked before every write and during backpressure, so disable,
sign-out and shutdown can retire a process that stops reading. Partial or timed
out writes retire the session rather than reusing an incomplete protocol frame.
Every exchange also checks the runtime owner's shared interruption signal,
independently of its caller's cancellation callback. Sign-out destroys the
interrupted process directly; its memory-only credentials and login state end
with that process.
Both providers use one material stage loop; completed passes carry their original
start time and reported retries through subsequent local validation failures.

Qualified passive model/auth/warning events are accepted. Unknown notifications
remain protocol failures; actual tool/permission events remain containment
failures. Both terminate the process safely, which ends memory-only sign-in.
A protocol failure by itself is not evidence of access outside the OS sandbox.
Startup also verifies restrictive effective feature flags and explicitly disabled
utility tools using the pinned runtime's supported raw configuration layers.
See `docs/ai/codex-integration-review.md` for findings and verification.
