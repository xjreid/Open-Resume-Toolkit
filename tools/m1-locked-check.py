#!/usr/bin/python3
"""Read-only before/after comparison for a user-controlled locked-Keychain test."""
import hashlib
import json
import os
from pathlib import Path
import runpy
import sys


def main():
    assert len(sys.argv) == 2 and sys.argv[1] in ("before", "after")
    assert os.getuid() == os.geteuid() == 502
    assert Path.home() == Path("/Users/orttest")
    assert os.stat("/dev/console").st_uid == os.getuid()
    helper = runpy.run_path(str(Path(__file__).with_name("m1-account-check.py")))
    read = helper["read_bounded"]
    profiles = Path.home() / "Library/Application Support/com.openresumetoolkit.dev/profiles"
    snapshot = {}
    for entry in sorted(profiles.iterdir()):
        assert not entry.is_symlink()
        paths = sorted(entry.iterdir()) if entry.is_dir() else [entry]
        for path in paths:
            snapshot[str(path.relative_to(profiles))] = hashlib.sha256(read(path, 64 * 1024 * 1024)).hexdigest()
    receipt = Path.home() / "ORT-M1-locked-before.json"
    if sys.argv[1] == "before":
        # Take this after normal app quit, so WAL checkpointing is complete.
        fd = os.open(receipt, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "w") as stream:
            json.dump(snapshot, stream, indent=2)
        print("BASELINE SAVED: now perform the instructed Keychain lock/deny test")
    else:
        before = json.loads(read(receipt, 16384))
        assert snapshot == before, "STOP: profile file inventory or bytes changed"
        print("ALL CHECKS PASSED: encrypted profile files unchanged after Keychain denial")


if __name__ == "__main__":
    main()
