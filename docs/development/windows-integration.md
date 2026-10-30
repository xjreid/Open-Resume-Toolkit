# Windows development integration

The Windows integration targets Windows 11 x64 and preserves the existing
macOS development channel. The first qualification machine is Windows 11 25H2.
Windows Server CI remains portability evidence, not Windows desktop qualification.

## Execution boundary

The initial source-preparation request excluded building, installing, launching,
or live-testing the development app. The subsequent October 29, 2026 request
explicitly authorizes building and installing the Windows development app for
the current user. Native adapter checks remain separate from live application
qualification. Record build/install results and unrun runtime checks separately.

## Source implementation checklist

- [x] Reproducible Windows prerequisites, bootstrap, and check commands.
- [x] Explicit device-local Windows credentials, including cross-process creation.
- [x] Private ACLs, reparse rejection, local application paths, and profile locks.
- [x] Windows persistence, backup/recovery, and no-clobber export adapters (library checks).
- [x] Contained metered-Wasm parser supervisor and deadline-aware pipes (native fixtures; packaged helper remains unrun).
- [x] Authenticated Chrome development named-pipe bridge and per-user registration source.
- [x] Windows drag-out composition, private temporary-file leases, shutdown, and platform-aware UI.
- [x] Helper-complete development packaging source and per-user NSIS configuration.
- [x] Focused Windows automated checks and preservation of macOS CI gates.
- [x] Codex availability gate: Windows containment remains unproven, so it stays disabled without privileged setup.

These checkboxes describe the completed source-preparation stage. Its verification
record below predates the subsequent build/install authorization. At that stage,
actual app/helper packaging, installed-resource checks, live Chrome/
drag/quit testing and native macOS regression execution remain unrun. They are
later qualification work, not implied by successful source checks.

Implement Windows FFI in a small reviewed native crate; retain the unsafe-code
prohibition in application/domain/storage crates. Public renderer commands never
accept helper paths, executable identities, ACLs, or sandbox policies.

## Credential policy

Database keys and provider credentials use the current Windows user's Credential
Manager with **Local** persistence (`CRED_PERSIST_LOCAL_MACHINE`). This persistence
means the same user's subsequent logons on this computer; it does not grant other
users access. Do not use the backend's default Enterprise persistence, which may
roam in managed environments. Namespace entries by channel, installation, profile,
and credential identity. Never overwrite an existing database key or silently
regenerate a missing key. Vault failures have no plaintext fallback.

The generic-credential boundary is the Windows account, not executable identity:
other processes running as the same user can access generic credentials. No
malware-resistant or hardware-bound key-storage claim is made. SQLCipher retains
the encrypted profile; passphrase-encrypted backups remain the portable recovery
mechanism and restore with a new OS-local database key.

## Codex gate

Full parity remains incomplete unless filesystem, process, credential, and network
containment of the entire Windows app-server process is proven. Do not substitute
proxy environment variables or Codex's internal tool sandbox for that boundary.
Do not add privileged setup, weaken containment, silently switch AI modes, or
enable Windows Codex on the strength of compilation alone. A failed standard-user
proof leaves Codex visibly unavailable and records the remaining parity blocker.

## Source-preparation verification record

Planning baseline: 16 document-evidence/documentation tests and four embedded-asset
checkout tests passed. The latter required execution outside the agent sandbox
because Git could not create disposable repositories within its temporary root.
No development application was built or launched. The initial checkout had no
Rust toolchain, JavaScript dependencies, or MSVC tools available; Node/pnpm differed
from the repository pins. An isolated pinned Rust toolchain is now available in
ignored `target/windows-toolchain` for adapter/source checks.

Implemented foundation: a Windows-only native adapter crate confines FFI to its
security, pipes, parser and registration modules. Protected user/SYSTEM DACLs apply at private directory creation;
local NTFS paths reject junction/reparse redirection and alternate streams, and
held data/list-directory handles prevent ancestor replacement. Application-managed
files reject hard links and receive checked private ACLs. Broadened existing file
permissions are refused instead of silently repaired. Source reads may use a
native-dialog-selected removable/non-NTFS drive; managed storage and export
publication require local NTFS. Both paths refuse remote and reparse redirection.
Removable-drive reads have not been exercised on a physical removable device.

Credential checks passed on Windows outside the agent's own AppContainer sandbox:
five vault unit tests, seven native security tests, two synthetic OS-vault lifecycle
tests, migration of an Enterprise credential to Local without changing its bytes,
and a two-process exclusive-creation regression. All created credentials used
random synthetic test identities and were removed. No application profile or real
provider credential was used. The test fixtures intentionally preserve production
ACL requirements instead of allowing broadly inherited temporary directories.

One bounded negative check caught that attribute-only Windows handles do not
prevent directory rename. The adapter now requests data/list-directory access;
the directory replacement test passes. Checks inside the agent sandbox initially
failed because the new ACLs deliberately exclude that sandbox's AppContainer SID;
the outside-sandbox run tests the intended standard-user application boundary.

Microsoft-signed C++ Build Tools and Windows SDK were installed as prerequisites.
Rust 1.98.0 and checksum-verified portable Strawberry Perl 5.42.3.1 are isolated in
`target/windows-toolchain`. The existing Git Perl lacks the modules required by
OpenSSL. The storage/platform source checks and Clippy with warnings denied passed.
After private-file enforcement, 76 storage unit tests, six guardrail integration
tests, the separate SQLCipher startup test, two publication/backup integration
tests, 12 input/export tests, and seven native security tests passed. These are
library tests, not an ORT application build. The new Windows check script also
passed in `-CheckOnly` mode (formatting, all-target library Clippy with warnings
denied, and source checks). Its first MSVC environment activation caused a fresh
vendored OpenSSL configuration. `tools/check-windows.ps1` provides repeatable
library-only checks and optional synthetic native vault qualification.

PowerShell parsing and documentation-link checks passed. Desktop production and
test targets now typecheck under Clippy with warnings denied on Windows, including
the development bridge feature. Native adapter/helper source checks pass as well.
The first desktop check hit a native compiler failure inside the agent sandbox;
the check outside it passed. No debug/release desktop executable or installer has
been created, installed, or launched. No browser or live in-app testing has
occurred. Native macOS checks remain unrun on this machine.

The source-secret scan passed with checksum-pinned Gitleaks 8.30.1. The Rust
dependency license gate passed for 795 locked Rust and 167 JavaScript packages.
The Windows web-security
checker now converts file URLs using `fileURLToPath` instead of treating a URL
pathname as a native Windows path; the actual web-security scan passed. Sixteen
existing document-evidence/documentation tests also passed. Node 24.16.0 and
pnpm 11.19.0 were provisioned inside ignored tooling using the pinned official
archive checksums. Locked JavaScript dependencies were installed with lifecycle
scripts disabled. TypeScript checks passed for desktop and extension, and all
51 desktop web component files passed (346 tests) in the virtual DOM. The first
virtual-DOM run inside the agent sandbox could not rename its test cache and ran
no tests; the outside-sandbox run passed. Rust's pinned `wasm32-wasip1` target and
the checksum-verified PDFium archive are cached as build prerequisites. No bundled
Wasm guest or actual parser-helper executable was built; helper Rust sources were
typechecked.

Keep this record current as implementation proceeds. Unchecked work is unfinished,
not implicitly accepted. Edge capture, planned answer autofill, public signing,
Store delivery, and production updates remain separate work.

The native parser supervisor now creates a fresh zero-capability AppContainer,
copies and pins the compiled-hash helper inside a private profile stage, applies
the one-process kill-on-close Job at process creation, verifies its token/image
and effective child/dynamic-code/extension-point/Win32k policies while suspended,
then sends source bytes over exactly three inherited pipes. It returns output
only after successful exit, bounded EOF and confirmed Job/profile cleanup. Pipe
buffers stay allocated until overlapped I/O completion is confirmed. Unconfirmed
cancellation retains the bounded allocation and rejects output rather than
freeing memory that Windows might still use. The source document is never staged
on disk by this adapter. The helper uses the Windows GUI executable subsystem to
avoid console-host initialization under its single-process policy; it has no UI.

Four disposable native parser tests passed with the full policy on this machine:
valid bounded output, image mismatch/pre-cancellation, timeout/output flood and
subsequent reuse, and resource-denial controls. A live parent-created synthetic
credential and private file were inaccessible, helper-stage writes and child
creation failed, and a confirmed-live loopback listener received no helper
connection. Windows filtering produced a bounded timeout for the denied socket;
the test does not claim an immediate access-denied socket error. Optional
mitigation diagnostics used during development have been removed; no fallback
policy can enable import. Native checks used synthetic credentials only.

Windows import composition now uses the existing bounded extraction decoder and
requires both the compiled helper digest and `appcontainer-job-wasm-v1` policy
marker. No unpinned build enables import. Actual guest/helper packaging and
end-to-end extraction with the packaged helper remain unrun.

The opt-in Windows Chrome development bridge uses the same HMAC/session/nonce,
expiry and response-binding protocol as macOS, over current-user/SYSTEM-only
local named pipes. A single pipe instance is retained between clients, remote
clients are rejected, and the client forbids server impersonation. Reads/writes
and shutdown cancellation are bounded. A receipt acknowledgment avoids losing
unread response bytes at disconnect without an unbounded flush. Three native
bridge tests passed for roundtrip/exclusive ownership/restart/cleanup,
replay/tampering/direction/session rejection, and incomplete-peer shutdown/public
root refusal. The default production gate stays disabled. Explicit per-user
Chrome development registration commands are implemented in the native host;
neither command has been run, and no Chrome registry entry has been changed.

Three Windows drag-artifact library tests passed for private materialization,
live-lease preservation, stale cleanup, hard-link refusal and preservation of
unknown contents. No drag interaction or other application runtime check has run.

Twelve native Windows unit tests now cover the filesystem, mutex, pending-pipe
buffers and synthetic registry ownership cases. Four contained-parser fixtures
pass with the full policy; successful extraction transport also checks that the
unique profile directory no longer exists after the OS deletion call. Pending
I/O objects are pinned, and kernel-mutated buffers/overlapped state use explicit
interior mutability; no buffer reference is exposed until completion. All five
effective process-policy attributes use stable owned backing through creation.
Synthetic registry tests create/delete only a random test leaf, preserve Unicode
values, and refuse another manifest's ownership. A native-host unit test proves
complete/no-clobber image publication, private metadata and tamper refusal. No
real Chrome registration command has been executed.

Windows packaging has five source-only tests covering current-user NSIS,
fixed helper paths, case-insensitive environment cleanup and executable
architecture/subsystem checks. The packaging entry point and Windows SDK tool
environment setup have been written. The environment-only preflight passed on
this machine, including native Perl/Rust/Node/pnpm and Tauri CLI version lookup.
It caught and fixed Python's Windows child-PATH executable lookup difference;
the launcher now resolves native tool paths explicitly and avoids batch wrappers.
The packaging/build entry point itself remains deliberately unexecuted. CI adds
Windows adapter, bridge and packaging checks while keeping native macOS gates.
Windows Server CI is portability evidence; it does not replace the Windows 11
containment probes above or future application qualification.

The repository tooling audit found Windows-specific CRLF, Python UTF-8 and file-
URL issues. The checks now preserve their original validation while handling
those platform representations. The secret-scanner link fixture uses a Windows
junction, so it can exercise refusal without symlink-creation privileges.

All 48 repository tooling tests passed with no skips after those portability
fixes. Four native vault tests also passed after the final mutex refactor,
covering persistence conversion, database/provider roundtrips and removal,
overwrite refusal and competing-process creation. Credential-using parser probes
now require both the separate `native-probes` feature and an ignored-test plus
environment opt-in. Ordinary shared fixture support never opts into OS credential
creation. The PowerShell `-NativeContainment` and `-NativeVault` switches set and
restore only their respective test opt-ins.

Final source verification passed through
`tools/check-windows.ps1 -CheckOnly -DesktopSource -NativeContainment`: library
and desktop/test-target Clippy with warnings denied, helper source checks, all
four explicitly opted-in parser fixtures and all ten development IPC tests.
The ordinary parser test invocation skipped all four credential-using fixtures.
All five packaging configuration tests, the source-secret/web-security scans,
dependency-license gate and documentation-link checks passed. No desktop
executable or NSIS output exists in the checked development output locations.

Codex discovery and installation still return `PLAN_PLATFORM_UNSUPPORTED` before
starting a Windows runtime. The tested zero-capability parser cannot supply the
controlled network access Codex needs, and no qualified Windows app-server
adapter exists in this implementation. This records an unproven containment
boundary, not a claim that Windows containment is impossible. The user-approved
outcome is explicit unavailability, with no privileged installer or weaker
fallback. API-provider mode remains separate and uses the device-local vault.

## Windows build and installation — October 29, 2026

The subsequent user request authorized the first development build and installation.
The pinned DOCX/PDF guests, Windows parser helper, Chrome development host, frontend
and desktop executable built successfully. Tauri produced an unsigned x64 NSIS
installer using current-user mode. The frontend artifact secret scan passed, and
NSIS/tool-plugin downloads passed Tauri's checksum validation. WebView2 was already
present. OpenSSL debug-symbol and Cargo PDB-name collision warnings were nonfatal;
the optimized desktop build completed.

The first final packaging check incorrectly applied the 64 MiB helper limit to
the 82,432,512-byte desktop executable. The check now uses a separate 256 MiB
desktop bound and reads only bounded PE headers. Helper size, architecture and
GUI/console subsystem requirements remain unchanged. The corrected packaging
entry point completed successfully with cached dependencies.

Post-install validation exposed a second manifest issue: Tauri places
`__TAURI_BUNDLE_TYPE_VAR_NSS` in the NSIS payload and restores
`__TAURI_BUNDLE_TYPE_VAR_UNK` in the loose executable after bundling. The installed
image differed only in those three marker bytes. The manifest producer now hashes
the exact expected NSIS image, requires one unambiguous original marker, and also
records the restored executable's hash. This matches the
[Tauri bundler's patch/restore implementation](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle.rs).
The existing manifest was corrected only after confirming the unchanged installer,
unchanged loose executable and exact full installed-image hash. Seven packaging
tests passed, including large-desktop bounds and marker-ambiguity rejection.

The installer completed with exit code 0 for the current Windows user at
`%LOCALAPPDATA%/Open Resume Toolkit Dev`. The installed desktop,
parser and native-host hashes match the build manifest. All 20 helper/resource
files, including PDFium notices and parser manifest, match their staged originals.
The HKCU uninstall entry, uninstaller and Start menu/Desktop shortcut targets were
verified. No app process was observed, and no launch flag was supplied. Chrome
registration was not performed. Codex remains unavailable on Windows.

Build output and receipts are in ignored
`target/windows-development-20261030T030210134429Z/`:
`build-manifest.json` and `install-receipt.json`. The installer SHA-256 is
`5886ab8fe1075651468bdb3d014a22d29d98861ffc7753c8141acf8a71b57715`;
the installed desktop SHA-256 is
`2a75cb49854c82ad4e73ed49b6d76dbc81620ab2b711050159d0c3b03f0d58fe`.
Installed app startup, packaged-helper execution, live Chrome/drag/quit workflows,
native macOS regressions and production signing qualification remain unrun.

## Windows menu-bar correction — October 29, 2026

The user reported the File/Edit/Window/Help row beneath the Windows title bar.
The desktop builder now attaches the existing native editor menu only on
non-Windows targets. Windows creates no application menu, while its title-bar
Close/Alt+F4 handlers and macOS menu construction remain unchanged.

Windows desktop/test-target Clippy passed with warnings denied. The development
app was rebuilt and the current-user installation updated successfully. The
installed desktop and all 20 helper/resource files matched the new manifest.
The installed desktop SHA-256 is
`af0f384b9ea20d895300c1ee5e752aa77b5df7ae87969eca6f62d56ef4df41c4`.
The new build/install receipts are in ignored
`target/windows-development-20261030T031617374311Z/`. Automated visual verification
could not proceed because computer-use access to Open Resume Toolkit Dev was not
approved. No visual pass is claimed, and the separate save-warning banner in the
user's screenshot was not addressed by this menu-only change.

## Code-review corrections — October 29, 2026

The four review findings have source fixes and focused regressions:

- Chrome setup/removal now holds one native operation guard across image,
  manifest and registry changes. Desktop registration reads take the same guard.
  The Chrome manifest supplies both consumers' extension identity, removing the
  independently committed Windows registration metadata. Failed registry commits
  restore the previous complete manifest; a synthetic two-process test verifies
  removal cannot interleave with publication. Existing metadata is ignored for
  connection and removed during unregistration.
- Drag startup cleanup streams the root inventory on a background thread with
  constant inventory memory, yielding between batches. The complete shallow schema,
  private ACL, reparse/hard-link and live-owner checks still apply to each artifact.
  A root with more than 512 entries no longer disables cleanup or starves later
  expired exports; unrelated entries and active leases remain preserved.
- Storage startup and lifecycle acquisition now live in the platform composition
  module. A legacy profile becomes a recoverable UI error instead of aborting
  shell construction. Empty legacy directories are accepted, and nonempty legacy
  data remains unchanged. Both retry controls retry native storage acquisition.
  Recovery instructions explain backup, retaining the old folder and restoring
  from Settings. No new profile is created while legacy data blocks startup.
- Packaging retains and verifies an independent installer copy in each
  timestamped output directory before recording its path/hash. Later builds
  cannot invalidate the retained installer of an earlier build.

Windows formatting and desktop/native all-target Clippy passed with warnings
denied. The Rust regression run passed 265 tests, including 133 desktop tests
(six opt-in desktop tests remain ignored), the new cross-process/rollback,
manifest-validation, cleanup-overflow and legacy-state cases, and existing
storage/recovery tests. Nine packaging tests, all 347 frontend tests and all 48
tool regressions passed. Desktop TypeScript validation, source Gitleaks,
web-security and documentation-link checks also passed. An isolated
headless-browser preview with
synthetic native responses showed recovery guidance without page overflow at
1080×760 and 720×520; this is frontend evidence, not an installed WebView pass.

These source fixes have not been packaged or installed during this correction
pass. Live Chrome registration and native macOS regression checks remain unrun.
