# macOS termination bridge

This first-party crate connects AppKit's `applicationShouldTerminate:` /
`NSTerminateLater` protocol to the desktop's existing close decision. It accepts
no documents, paths, credentials, renderer payloads or process-launch requests.
The pinned Tao 0.35.3 delegate has no termination-decision method; the usual
Tauri exit callback cannot intercept native `terminate:` requests.

## Boundary and unsafe exception

Workspace-wide `unsafe_code = "forbid"` stays unchanged. This crate uses
`unsafe_code = "deny"` with a single explicit exception on its private FFI
module. The two calls have scalar arguments/results and a static function
pointer. A `OnceLock` retains the callback for process lifetime; panics are
caught before returning across the ABI. There is no borrowed pointer, buffer,
ownership transfer or caller-supplied Objective-C object.

The Objective-C implementation checks the main thread before touching its
state. It adds only a missing delegate method, obtains the method ABI encoding
from an Objective-C declaration, and refuses an existing/inherited method.
It never replaces the delegate or an existing implementation. Installation is
one-shot; desktop startup fails before opening storage if installation fails.
A future runtime update that adds this method therefore requires a deliberate
integration change rather than silently replacing the upstream behavior.

Requests coalesce while pending. An AppKit selector delivers the callback in
default, modal-panel and event-tracking run-loop modes. A false callback cancels
the native request. Only a main-thread reply to an outstanding request can
approve termination; missing, repeated and worker-thread replies cannot. The
desktop independently validates its main-window, single-use close attempt and
atomically excludes new file/render operations before approving.

This is not recovery for Force Quit, a crash or power loss. Neither a timeout nor
a missing renderer response approves a quit. Real logout/shutdown and installed
editor behavior require the remaining native qualification matrix.

## Dependency record

- First-party code uses the workspace's `GPL-3.0-only` license and normal ORT
  source attribution. It does not copy upstream Tauri/Tao implementations.
- Build-only `cc = 1.4.4` was already locked in the workspace. It compiles the
  small Objective-C source against the macOS system AppKit framework; it adds no
  downloaded binary, runtime network behavior or new third-party package.
- No new parser, cryptography, vault or credential-handling dependency is added.
  The crate's security role is lifecycle authorization only.
- Removal boundary: replace `install`/`reply` with an upstream native lifecycle
  API once that API and its nested-run-loop behavior pass the same tests.

Run `node tools/check-termination-macos.mjs` for isolated native regression and
static analysis. The optional `ort-desktop` example `termination_probe` exercises
Tauri main-thread scheduling using a blank webview and a unique synthetic app
identity; its first native menu Quit cancels and its second approves. It does not
call the production startup function, register app commands or open a profile.
See [checkpoint evidence](../../evidence/0.0.0-dev/m2-native-termination.md).
