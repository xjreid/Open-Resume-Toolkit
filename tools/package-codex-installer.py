#!/usr/bin/env python3
"""Package the development-only offline Codex copy helper. Never installs or runs it elevated."""
import hashlib
import json
import pathlib
import plistlib
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    subprocess.run(["cargo", "build", "--locked", "--offline", "--release", "-p", "ort-codex-install"], cwd=ROOT, check=True)
    bundle = ROOT / "target/ORT Codex Installer.app"
    contents = bundle / "Contents"
    binary = contents / "MacOS/ort-codex-install"
    binary.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / "target/release/ort-codex-install", binary)
    binary.chmod(0o755)
    (contents / "Info.plist").write_bytes(plistlib.dumps({
        "CFBundleExecutable": "ort-codex-install",
        "CFBundleIdentifier": "com.openresumetoolkit.dev.codexinstaller",
        "CFBundleName": "ORT Codex Installer", "CFBundlePackageType": "APPL",
        "CFBundleVersion": "1", "LSUIElement": True,
    }))
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-", "--options", "runtime", str(bundle)], check=True)
    subprocess.run(["/usr/bin/codesign", "--verify", "--strict", str(bundle)], check=True)
    linked = subprocess.run(["/usr/bin/otool", "-L", str(binary)], capture_output=True, text=True, check=True).stdout
    for line in linked.splitlines()[1:]:
        dependency = line.strip().split(" (", 1)[0]
        if not dependency.startswith(("/usr/lib/", "/System/Library/")):
            raise SystemExit("Installer helper has an unapproved dynamic dependency")
    result = {"schemaVersion": 1, "developmentOnly": True,
              "executableSha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "runtimeVersion": "0.162.0", "network": False,
              "destination": "/Library/Application Support/Open Resume Toolkit/Codex/codex"}
    (ROOT / "target/codex-installer-package.json").write_text(json.dumps(result, indent=2) + "\n")
    print("Verified development Codex installer helper packaged; no runtime installed")


if __name__ == "__main__":
    main()
