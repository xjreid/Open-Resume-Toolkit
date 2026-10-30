#!/usr/bin/env python3
"""Provision checksum-pinned Windows x64 Node/pnpm inside ignored target/ only.

No global PATH, package installation, application build, or registration changes.
Requires network for missing official archives; existing matching files are reused.
"""
import base64
import hashlib
import io
import json
import pathlib
import stat
import subprocess
import sys
import tarfile
import urllib.request
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
TOOLS = ROOT / "target/windows-toolchain"
NODE = "24.16.0"
PNPM = "11.19.0"
NODE_SHA = "edaca9bd58ec8e92037dac4e877d52f6b8f430b81c18b57e264b4e2fb111cd56"
PNPM_SHA512 = "eIHz7VkNRyxKlV4riLISF5ERYGbcyIy8o4SeybYPG7qm0syyIfqR2k4cZb7yvL43k2Wup6xTnHv4be3DobItzg=="


def archive(name, url, expected, algorithm):
    path = TOOLS / name
    if path.exists():
        data = path.read_bytes()
    else:
        with urllib.request.urlopen(url, timeout=60) as response:
            data = response.read(100 * 1024 * 1024 + 1)
    if len(data) > 100 * 1024 * 1024 or hashlib.new(algorithm, data).digest() != expected:
        raise SystemExit(f"Archive checksum/size refused: {name}")
    if not path.exists():
        with path.open("xb") as file:
            file.write(data)
    return data


def write_member(root, name, data):
    parts = pathlib.PurePosixPath(name).parts
    if not parts or any(part in ("..", "/") or ":" in part or "\\" in part for part in parts):
        raise SystemExit("Unsafe archive path")
    target = root.joinpath(*parts)
    if not target.resolve().is_relative_to(TOOLS.resolve()) or len(data) > 128 * 1024 * 1024:
        raise SystemExit("Unsafe archive destination/size")
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        if target.read_bytes() != data:
            raise SystemExit(f"Existing tool file differs; refusing overwrite: {target}")
    else:
        with target.open("xb") as file:
            file.write(data)


def main():
    if sys.platform != "win32":
        raise SystemExit("Windows x64 only")
    package = json.loads((ROOT / "package.json").read_text())
    if (ROOT / ".nvmrc").read_text().strip() != NODE or package["engines"]["pnpm"] != PNPM:
        raise SystemExit("Review and update archive pins with repository tool versions")
    if not TOOLS.resolve().is_relative_to(ROOT):
        raise SystemExit("Tool destination leaves workspace")
    TOOLS.mkdir(parents=True, exist_ok=True)
    node_archive = f"node-v{NODE}-win-x64.zip"
    data = archive(node_archive, f"https://nodejs.org/dist/v{NODE}/{node_archive}", bytes.fromhex(NODE_SHA), "sha256")
    with zipfile.ZipFile(io.BytesIO(data)) as bundle:
        if len(bundle.infolist()) > 10_000:
            raise SystemExit("Unexpected Node archive")
        for member in bundle.infolist():
            if stat.S_ISLNK(member.external_attr >> 16) or member.file_size > 128 * 1024 * 1024:
                raise SystemExit("Unexpected Node member")
            if not member.is_dir():
                write_member(TOOLS, member.filename, bundle.read(member))
    data = archive(f"pnpm-{PNPM}.tgz", f"https://registry.npmjs.org/pnpm/-/pnpm-{PNPM}.tgz",
                   base64.b64decode(PNPM_SHA512, validate=True), "sha512")
    with tarfile.open(fileobj=io.BytesIO(data)) as bundle:
        members = bundle.getmembers()
        if len(members) > 10_000:
            raise SystemExit("Unexpected pnpm archive")
        for member in members:
            parts = pathlib.PurePosixPath(member.name).parts
            if not parts or parts[0] != "package" or not (member.isfile() or member.isdir()):
                raise SystemExit("Unexpected pnpm member")
            if member.isfile():
                if member.size > 64 * 1024 * 1024:
                    raise SystemExit("Oversized pnpm member")
                write_member(TOOLS / "pnpm", "/".join(parts[1:]), bundle.extractfile(member).read())
    node = TOOLS / f"node-v{NODE}-win-x64/node.exe"
    cli = TOOLS / "pnpm/bin/pnpm.cjs"
    subprocess.run([str(node), "--version"], check=True)
    subprocess.run([str(node), str(cli), "--version"], check=True)
    shim = (f'@echo off\r\n"%~dp0..\\node-v{NODE}-win-x64\\node.exe" '
            '"%~dp0..\\pnpm\\bin\\pnpm.cjs" %*\r\n').encode()
    write_member(TOOLS, "js-bin/pnpm.cmd", shim)
    print("Pinned Node/pnpm ready inside target/windows-toolchain; system PATH unchanged.")


if __name__ == "__main__":
    main()
