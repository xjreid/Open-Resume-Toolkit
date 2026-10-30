# Windows direct packaging

Windows 11 x64 development packaging is implemented in
[`tools/build-development-windows.py`](../../../tools/build-development-windows.py).
The shared [`build-development-app.py`](../../../tools/build-development-app.py)
entry point dispatches here on Windows and preserves macOS packaging.

Source preparation stopped before building, installing, launching or live-testing
the development app. The subsequent October 29, 2026 request authorizes a Windows
build and current-user installation. See the [verification record](../../../docs/development/windows-integration.md)
for the actual result. Chrome native-host registration remains a separate explicit
setup action.

## Prerequisites and source checks

Use PowerShell 7, Python 3.11+, Rust 1.98.0 with `clippy`, `rustfmt`, and the
`wasm32-wasip1` target, Microsoft C++ Build Tools with the x64 tools/Windows SDK,
and native Windows Perl. Git/MSYS Perl cannot build vendored OpenSSL. The tested
portable Perl version is Strawberry 5.42.3.1. WebView2 Evergreen Runtime must be
installed before the app is used; this development installer deliberately does
not download or install it. See [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows).

```powershell
python tools/setup-windows-js.py
# Installs checksum-pinned Node 24.16.0 and pnpm 11.19.0 into ignored target/.
& ./target/windows-toolchain/node-v24.16.0-win-x64/node.exe ./target/windows-toolchain/pnpm/bin/pnpm.cjs install --frozen-lockfile --ignore-scripts
pwsh -File tools/check-windows.ps1 -CheckOnly -DesktopSource
# Optional native synthetic fixtures; never application credentials or profiles:
pwsh -File tools/check-windows.ps1 -NativeVault -NativeContainment
```

The check script and build entry point detect the isolated toolchain under
`target/windows-toolchain` when present. Otherwise Rust/Perl must be deliberately
installed on PATH. The build entry point loads the installed Visual Studio native
tool environment without changing global PATH or requiring elevation.

## Build

```powershell
python tools/build-development-app.py --download-parser-guests
```

Omit the download flag when the pinned PDFium archive is already cached. The build
compiles the same metered DOCX/PDF Wasm guests used on macOS, verifies their pinned
assets, builds the Windows helper and Chrome development native host, and checks
their PE architecture/subsystem before packaging. Independent helper copies are
placed in `helpers/`; the parser hash and `appcontainer-job-wasm-v1` policy are
compiled into the desktop app. An unpinned shell build keeps import disabled.

The generated configuration uses `currentUser` NSIS installation and the
`com.openresumetoolkit.dev` identity. It preserves the CSP and capability files.
The build produces an **unsigned development installer** and a hash manifest,
recorded through `target/latest-windows-development.txt`. It does not run that
installer, launch the app, register Chrome, or install Codex. The October 29 local
build and separate current-user installation are recorded in the verification
record linked above; executable/helper/resources and shortcut targets passed
post-install verification. The app remains unlaunched.

Each new build retains an independent installer copy beside its timestamped
`build-manifest.json`. The recorded `installerPath` and SHA-256 refer to that
copy, so subsequent builds cannot overwrite an earlier build's installer.

The manifest's `desktopSha256` describes the NSIS payload. Tauri changes its
bundle-type marker during packaging, then restores the loose build executable;
`unbundledDesktopSha256` records that restored image separately. The development
packager requires exactly the expected marker before calculating the packaged
hash and refuses ambiguity. Architecture/subsystem checks retain the helper's
64 MiB bound and use a separate 256 MiB desktop bound for embedded fonts/assets.

## Optional Chrome development registration

After a built app has been installed deliberately, invoke its bundled native host:

```powershell
& 'C:/path/to/installed/app/helpers/ort-native-host.exe' --register-development
# To remove that exact registration:
& 'C:/path/to/installed/app/helpers/ort-native-host.exe' --unregister-development
```

These commands use a private per-user host directory and only
`HKCU/Software/Google/Chrome/NativeMessagingHosts/com.openresumetoolkit.dev`.
They refuse a registration belonging to another installation. They never register
Edge or the production host name. A running copy may remain on disk after
unregistration; no process is force-killed. Use the separate BETA extension with
the repository's matching development public key. The desktop connection is
available only in the explicit development feature and trusts the Windows account.

Windows setup, removal and desktop registration reads share one current-user
operation lock. The Chrome manifest is the authoritative extension identity;
Windows no longer creates a separate `registration.json`. Existing registrations
remain readable, and removal also cleans up the older metadata file. A failed
registry commit restores the previous manifest before releasing the lock.

## Recovering an earlier Windows profile

If a nonempty legacy `profiles` folder exists beneath the development app's Local
or Roaming AppData directory, the app opens with recovery guidance and leaves that
profile untouched. Empty legacy folders do not block startup. Use the earlier
build to create and verify an encrypted backup, then move its legacy `profiles`
folder to a safe location. Choose **Retry storage** and restore the backup from
**Settings**. Retain the old profile until the restored data has been verified.
The app does not automatically migrate or discard legacy data.

Database/provider keys use device-local Credential Manager entries; profiles use
private local NTFS storage. Copying a macOS database alone does not transfer its
key: use an encrypted portable backup. Codex remains unavailable on Windows until
its required process and network containment is established. No privileged Codex
setup is added. See the [implementation and verification record](../../../docs/development/windows-integration.md).

Public signing, production installers, Store delivery and updater integration
remain separate release work.
