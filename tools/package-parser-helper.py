#!/usr/bin/env python3
"""Package a development-only, ad-hoc signed App Sandbox parser helper.

This does not install or launch the desktop app and does not use the Keychain.
Release signing and desktop identity binding are separate packaging steps.
"""
import hashlib
import json
import os
import pathlib
import plistlib
import re
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    subprocess.run(["python3", "tools/build-parser-guests.py"], cwd=ROOT, check=True)
    environment = dict(os.environ, ORT_PARSER_GUEST_DIRECTORY=str(ROOT / "target/parser-guests"))
    subprocess.run(["cargo", "build", "--locked", "-p", "ort-parser-helper"],
                   cwd=ROOT, env=environment, check=True)
    bundle = ROOT / "target/ORT Parser Helper.app"
    contents = bundle / "Contents"
    executable = contents / "MacOS/ort-parser-helper"
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / "target/debug/ort-parser-helper", executable)
    executable.chmod(0o755)
    info = {"CFBundleExecutable": "ort-parser-helper", "CFBundleIdentifier": "com.openresumetoolkit.dev.parser",
            "CFBundleName": "ORT Parser Helper", "CFBundlePackageType": "APPL",
            "CFBundleVersion": "1", "LSUIElement": True}
    (contents / "Info.plist").write_bytes(plistlib.dumps(info))
    notices = contents / "Resources/pdfium-notices"
    shutil.copytree(ROOT / "target/parser-guests/pdfium-notices", notices, dirs_exist_ok=True)
    shutil.copyfile(ROOT / "target/parser-guests/manifest.json", contents / "Resources/parser-manifest.json")
    entitlements = ROOT / "target/parser-helper.entitlements"
    entitlements.write_bytes(plistlib.dumps({"com.apple.security.app-sandbox": True}))
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-", "--entitlements", str(entitlements), str(bundle)], check=True)
    subprocess.run(["/usr/bin/codesign", "--verify", "--strict", str(bundle)], check=True)
    details = subprocess.run(["/usr/bin/codesign", "-d", "--verbose=4", str(bundle)], check=True, capture_output=True, text=True).stderr
    cdhash = re.search(r"^CDHash=([0-9a-f]{40})$", details, re.MULTILINE).group(1)
    manifest = {"cdhash": cdhash,"schemaVersion": 1, "developmentOnly": True, "sandbox": "com.apple.security.app-sandbox",
                "executableSha256": hashlib.sha256(executable.read_bytes()).hexdigest()}
    (ROOT / "target/parser-helper-package.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print("Signed development parser helper in target/ORT Parser Helper.app")


if __name__ == "__main__":
    main()
