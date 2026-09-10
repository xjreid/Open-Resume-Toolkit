#!/usr/bin/env python3
"""Read-only orttest profile/Keychain-metadata receipts for final native QA.

Never retrieves a key/password, locks a Keychain, changes permissions, or mutates
profile data. Run after ORT quits. Receipts contain hashes and opaque identities.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess

ROOT_NAMES = {"default", ".ort-restore-staged", ".ort-restore-safety", ".ort-safety-delete-pending"}


def read(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_size > 64 * 1024 * 1024:
            raise RuntimeError("Unexpected profile file type or size")
        with os.fdopen(os.dup(fd), "rb") as stream:
            return stream.read(64 * 1024 * 1024 + 1), stat.S_IMODE(info.st_mode)
    finally:
        os.close(fd)


def key_present(account):
    result = subprocess.run(
        ["/usr/bin/security", "find-generic-password", "-s",
         "com.openresumetoolkit.dev.database", "-a", account],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10, check=False)
    if result.returncode not in (0, 44):
        raise RuntimeError("Keychain metadata check inconclusive")
    return result.returncode == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["snapshot", "compare-locked", "compare-deleted"])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    args = parser.parse_args()
    if (Path.home() != Path("/Users/orttest") or os.getuid() != os.geteuid()
            or os.getuid() == 0 or os.stat("/dev/console").st_uid != os.getuid()):
        raise SystemExit("Run only in the real orttest console login")
    profiles = Path.home() / "Library/Application Support/com.openresumetoolkit.dev/profiles"
    if profiles.is_symlink():
        raise SystemExit("Unexpected profile parent")
    files, references = {}, {}
    entries = sorted(profiles.iterdir())
    if len(entries) > 16:
        raise SystemExit("Unexpected profile inventory")
    for entry in entries:
        if entry.is_symlink():
            raise SystemExit("Unexpected profile symlink")
        if entry.is_dir():
            if entry.name not in ROOT_NAMES:
                raise SystemExit("Unexpected profile directory")
            paths = sorted(entry.iterdir())
            if len(paths) > 16:
                raise SystemExit("Unexpected profile directory inventory")
        else:
            paths = [entry]
        for path in paths:
            data, mode = read(path)
            relative = str(path.relative_to(profiles))
            files[relative] = {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "mode": oct(mode)}
            if path.name == "profile.json":
                manifest = json.loads(data)
                if manifest["channel"] != "dev":
                    raise SystemExit("Unexpected profile channel")
                account = f'install-{manifest["installId"]}.profile-{manifest["profileId"]}'
                references[entry.name] = account
    receipt = {"files": files, "references": references}
    if args.mode != "snapshot":
        if not args.baseline:
            raise SystemExit("Baseline required")
        before = json.loads(read(args.baseline)[0])
        if args.mode == "compare-locked":
            receipt["unchangedAfterDenial"] = files == before["files"] and references == before["references"]
            if not receipt["unchangedAfterDenial"]:
                raise SystemExit("FAIL: profile files changed after denied startup")
        else:
            old = set(before["references"].values())
            receipt["oldKeysAbsent"] = all(not key_present(value) for value in old)
            receipt["freshActiveIdentity"] = references.get("default") is not None and references["default"] not in old
            receipt["freshActiveKeyPresent"] = key_present(references["default"]) if references.get("default") else False
            receipt["recoveryRemoved"] = set(references) == {"default"} and all(name.startswith("default/") for name in files)
            if not all(receipt[key] for key in ["oldKeysAbsent", "freshActiveIdentity", "freshActiveKeyPresent", "recoveryRemoved"]):
                raise SystemExit("FAIL: deletion/key/recovery verification incomplete")
    fd = os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(receipt, stream, indent=2)
        stream.write("\n")
    print(f"PASS {args.mode}: {args.output}")


if __name__ == "__main__":
    main()
