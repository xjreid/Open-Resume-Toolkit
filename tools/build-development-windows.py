#!/usr/bin/env python3
"""Build an unsigned, helper-complete Windows x64 development installer.

Explicit build entry point only. Never installs, launches, registers Chrome,
installs Codex, signs a release, or changes the macOS package path.
"""
import argparse
import datetime
import hashlib
import json
import os
import pathlib
import platform
import shutil
import struct
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
TARGET = "x86_64-pc-windows-msvc"
POLICY = "appcontainer-job-wasm-v1"


def run(command, environment, capture=False, cwd=None):
    command = [str(part) for part in command]
    if sys.platform == "win32" and not pathlib.Path(command[0]).is_absolute():
        # CreateProcess does not reliably use the supplied child PATH to find
        # its executable. Resolve against the prepared SDK environment first.
        resolved = shutil.which(command[0], path=environment.get("PATH", ""))
        if not resolved:
            raise FileNotFoundError(f"Required native build tool not found: {command[0]}")
        command[0] = resolved
    if pathlib.Path(command[0]).suffix.lower() in (".cmd", ".bat"):
        raise ValueError("Invoke a native executable or JavaScript through Node; no batch shell")
    return subprocess.run(command, cwd=cwd or ROOT, env=environment,
                          check=True, text=True, encoding="utf-8",
                          stdout=subprocess.PIPE if capture else None).stdout


def environment():
    env = dict(os.environ)
    tools = ROOT / "target/windows-toolchain"
    prefixes = []
    if (tools / "cargo/bin/cargo.exe").is_file():
        env.update(CARGO_HOME=str(tools / "cargo"), RUSTUP_HOME=str(tools / "rustup"),
                   RUSTUP_TOOLCHAIN="1.98.0-x86_64-pc-windows-msvc")
        prefixes.append(tools / "cargo/bin")
    for path in [tools / "node-v24.16.0-win-x64", tools / "js-bin", tools / "strawberry-perl/perl/bin"]:
        if path.is_dir():
            prefixes.append(path)
    env["PATH"] = os.pathsep.join([*(str(path) for path in prefixes), env.get("PATH", "")])
    # Capture the native tool environment as data, without putting paths or
    # environment values into shell source or displaying credentials in logs.
    script = r'''
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$ortVsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$ortVsInstall = & $ortVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $ortVsInstall) { throw 'Microsoft C++ Build Tools are required.' }
Import-Module (Join-Path $ortVsInstall 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $ortVsInstall -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
[Environment]::GetEnvironmentVariables() | ConvertTo-Json -Compress
'''
    output = run(["pwsh", "-NoProfile", "-NonInteractive", "-Command", script], env, capture=True)
    env = {name.upper(): value for name, value in json.loads(output).items()}
    env["CARGO_TARGET_DIR"] = str(ROOT / "target")
    for name in ["ORT_CHROME_EXTENSION_ID", "ORT_DEV_EDGE_EXTENSION_ID", "ORT_PARSER_HELPER_CDHASH", "ORT_CODEX_INSTALLER_SHA256",
                 "ORT_PARSER_HELPER_SHA256", "ORT_WINDOWS_PARSER_POLICY"]:
        env.pop(name, None)
    env.update(PNPM_CONFIG_VERIFY_DEPS_BEFORE_RUN="false", PNPM_CONFIG_UPDATE_NOTIFIER="false")
    return env


def pe_subsystem(path, expected, *, maximum_bytes=64 * 1024 * 1024):
    with path.open("rb") as executable:
        size = os.fstat(executable.fileno()).st_size
        prefix = executable.read(64)
        if not 128 <= size <= maximum_bytes or prefix[:2] != b"MZ":
            raise ValueError("Invalid bundled executable")
        offset = struct.unpack_from("<I", prefix, 0x3C)[0]
        if offset > 65536 or offset + 96 > size:
            raise ValueError("Invalid bundled PE header")
        executable.seek(offset)
        header = executable.read(96)
    if len(header) != 96 or header[:4] != b"PE\0\0":
        raise ValueError("Invalid bundled PE header")
    machine, = struct.unpack_from("<H", header, 4)
    magic, = struct.unpack_from("<H", header, 24)
    subsystem, = struct.unpack_from("<H", header, 24 + 68)
    if (machine, magic, subsystem) != (0x8664, 0x20B, expected):
        raise ValueError("Bundled executable has the wrong architecture/subsystem")


def digest(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def retain_installer(source, output):
    """Retain independent verified bytes; future builds reuse Tauri's source path."""
    destination = output / source.name
    expected = digest(source)
    writer = destination.open("xb")  # An existing artifact is never ours to clean up.
    try:
        with writer, source.open("rb") as reader:
            shutil.copyfileobj(reader, writer)
            writer.flush()
            os.fsync(writer.fileno())
    except BaseException:
        destination.unlink(missing_ok=True)
        raise
    if digest(destination) != expected:
        destination.unlink()
        raise ValueError("Installer changed while retaining build output")
    return destination


def nsis_desktop_digest(path):
    # The pinned Tauri bundler patches this marker for the NSIS payload, then
    # restores the loose executable. This development path does not code-sign.
    # Hash the expected packaged bytes, keeping the restored image's hash too.
    data = path.read_bytes()
    original = b"__TAURI_BUNDLE_TYPE_VAR_UNK"
    packaged = b"__TAURI_BUNDLE_TYPE_VAR_NSS"
    if data.count(original) != 1 or packaged in data:
        raise ValueError("Unexpected Tauri bundle marker; review pinned bundler behavior")
    return hashlib.sha256(data.replace(original, packaged, 1)).hexdigest()


def bundle_config(helpers):
    return {"bundle": {"active": True, "targets": ["nsis"], "resources": {
        str(helpers / "ort-parser-helper.exe"): "helpers/ort-parser-helper.exe",
        str(helpers / "ort-native-host.exe"): "helpers/ort-native-host.exe",
        str(helpers / "parser-manifest.json"): "helpers/parser-manifest.json",
        str(helpers / "pdfium-notices"): "helpers/pdfium-notices/",
    }, "windows": {"webviewInstallMode": {"type": "skip"},
                   "nsis": {"installMode": "currentUser", "startMenuFolder": "Open Resume Toolkit Dev"}}}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--download-parser-guests", action="store_true")
    args = parser.parse_args()
    if sys.platform != "win32" or platform.machine().lower() not in ("amd64", "x86_64"):
        raise SystemExit("Windows x64 is required")
    if sys.getwindowsversion().build < 22000:
        raise SystemExit("Windows 11 is required")
    env = environment()
    node = pathlib.Path(shutil.which("node", path=env["PATH"]) or "")
    cli = ROOT / "target/windows-toolchain/pnpm/bin/pnpm.cjs"
    if not node.is_file() or not cli.is_file():
        raise SystemExit("Run tools/setup-windows-js.py before building")
    run([node, "tools/bootstrap.mjs"], env)
    if run(["perl", "-e", "print $^O"], env, capture=True).strip() != "MSWin32":
        raise SystemExit("Native Windows Perl is required")
    extension_id = run([node, "tools/dev-browser-bridge.mjs", "id"], env, capture=True).strip()
    if len(extension_id) != 32 or any(letter not in "abcdefghijklmnop" for letter in extension_id):
        raise SystemExit("Invalid development Chrome identity")
    guests = [sys.executable, "tools/build-parser-guests.py"]
    if args.download_parser_guests:
        guests.append("--download")
    run(guests, env)
    env.update(ORT_PARSER_GUEST_DIRECTORY=str(ROOT / "target/parser-guests"),
               ORT_DEV_CHROME_EXTENSION_ID=extension_id)
    for package, extra in [("ort-parser-helper", []), ("ort-native-host", ["--features", "dev-browser-bridge"])]:
        run(["cargo", "build", "--locked", "--release", "--target", TARGET, "-p", package, *extra], env)
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output = ROOT / "target" / f"windows-development-{stamp}"
    helpers = output / "helpers"
    helpers.mkdir(parents=True, exist_ok=False)
    for name, subsystem in [("ort-parser-helper.exe", 2), ("ort-native-host.exe", 3)]:
        source = ROOT / "target" / TARGET / "release" / name
        pe_subsystem(source, subsystem)
        shutil.copyfile(source, helpers / name)  # independent package file, never a Cargo hard link
    guest_manifest = ROOT / "target/parser-guests/manifest.json"
    shutil.copyfile(guest_manifest, helpers / "parser-manifest.json")
    shutil.copytree(ROOT / "target/parser-guests/pdfium-notices", helpers / "pdfium-notices")
    parser_hash = digest(helpers / "ort-parser-helper.exe")
    host_hash = digest(helpers / "ort-native-host.exe")
    env.update(ORT_PARSER_HELPER_SHA256=parser_hash, ORT_WINDOWS_PARSER_POLICY=POLICY)
    config = output / "tauri.windows.dev.json"
    config.write_text(json.dumps(bundle_config(helpers), indent=2) + "\n", encoding="utf-8")
    started = time.time()
    tauri_cli = ROOT / "apps/desktop/node_modules/@tauri-apps/cli/tauri.js"
    run([node, tauri_cli, "build", "--target", TARGET, "--features", "dev-browser-bridge",
         "--config", config, "--bundles", "nsis"], env, cwd=ROOT / "apps/desktop")
    desktop = ROOT / "target" / TARGET / "release/ort-desktop.exe"
    # The desktop embeds fonts and frontend assets; the smaller helper bound
    # remains aligned with the parser supervisor's executable-copy policy.
    pe_subsystem(desktop, 2, maximum_bytes=256 * 1024 * 1024)
    if digest(helpers / "ort-parser-helper.exe") != parser_hash or digest(helpers / "ort-native-host.exe") != host_hash:
        raise SystemExit("Bundled helpers changed during packaging")
    candidates = [path for path in (desktop.parent / "bundle/nsis").glob("*.exe")
                  if path.stat().st_mtime >= started - 2]
    if len(candidates) != 1:
        raise SystemExit("Cannot identify exactly one newly produced development installer")
    installer = retain_installer(candidates[0], output)
    manifest = {"schemaVersion": 1, "developmentOnly": True, "unsigned": True,
        "platform": TARGET, "installerPath": str(installer), "installerSha256": digest(installer),
        "desktopSha256": nsis_desktop_digest(desktop), "unbundledDesktopSha256": digest(desktop),
        "desktopBundleType": "nsis", "parser": {"sha256": parser_hash, "policy": POLICY},
        "nativeHost": {"sha256": host_hash, "chromeDevelopmentExtensionId": extension_id},
        "guests": json.loads(guest_manifest.read_text(encoding="utf-8")), "webview2Required": True,
        "codexAvailable": False, "appLaunched": False, "installed": False, "browserRegistered": False}
    (output / "build-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (ROOT / "target/latest-windows-development.txt").write_text(str(output) + "\n", encoding="utf-8")
    print(f"Unsigned Windows development installer built without installation or launch: {installer}")


if __name__ == "__main__":
    main()
