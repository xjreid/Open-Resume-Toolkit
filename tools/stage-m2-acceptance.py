#!/usr/bin/env python3
"""Copy the signed M2 candidate and synthetic fixtures for standard-account QA.

Creates a new directory under /Users/Shared. Does not install or launch an app,
change an account, modify Keychain items, or touch any existing transfer folder.
"""
import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=pathlib.Path, default=ROOT / "target/m2-candidate")
    parser.add_argument("--checklist", type=pathlib.Path, default=ROOT / "evidence/0.0.0-dev/m2-final-native-acceptance.md")
    args = parser.parse_args()
    candidate = args.candidate.resolve()
    manifest = json.loads((candidate / "manifest.json").read_text())
    app = candidate / "Open Resume Toolkit Dev.app"
    executable = app / "Contents/MacOS/ort-desktop"
    assert hashlib.sha256(executable.read_bytes()).hexdigest() == manifest["desktopSha256"]
    subprocess.run(["/usr/bin/codesign", "--verify", "--deep", "--strict", str(app)], check=True)
    transfer = pathlib.Path(tempfile.mkdtemp(prefix="ORT-M2-acceptance-", dir="/Users/Shared"))
    transfer.chmod(0o755)
    copied = transfer / app.name
    shutil.copytree(app, copied)
    assert hashlib.sha256((copied / "Contents/MacOS/ort-desktop").read_bytes()).hexdigest() == manifest["desktopSha256"]
    for suffix in ["pdf", "docx"]:
        source = ROOT / f"target/m2-style-parity-repeat-v2/technical/{suffix}/standard.{suffix}"
        shutil.copyfile(source, transfer / f"Synthetic resume.{suffix}")
    shutil.copyfile(candidate / "manifest.json", transfer / "manifest.json")
    launcher = transfer / "Open M2 Test.command"
    launcher.write_text('''#!/bin/zsh
set -eu
cd -- "${0:A:h}"
if [[ "$(/usr/bin/id -un)" != "orttest" || "$(/usr/bin/stat -f %Su /dev/console)" != "orttest" ]]; then
  print "Switch to the real orttest login before opening this candidate."
  exit 1
fi
if /usr/bin/pgrep -x ort-desktop >/dev/null; then
  print "Quit other ORT app instances before opening the test candidate."
  exit 1
fi
/usr/bin/codesign --verify --deep --strict "Open Resume Toolkit Dev.app"
/usr/bin/open -n "Open Resume Toolkit Dev.app"
''')
    launcher.chmod(0o755)
    shutil.copyfile(args.checklist, transfer / "Test checklist.md")
    (candidate / "transfer.json").write_text(json.dumps({"directory": str(transfer)}, indent=2) + "\n")
    print(transfer)


if __name__ == "__main__":
    main()
