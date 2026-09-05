#!/usr/bin/python3
"""Read-only Step 4 checks run by the real standard-account user.

Reads non-secret manifests, scans encrypted files for fixed synthetic markers,
and queries only Keychain item metadata (never passwords). Writes a receipt in
the test user's home. Does not lock/unlock Keychain, delete data, or change ACLs.
"""

import errno
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys


def read_bounded(path, limit):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(fd)
        assert stat.S_ISREG(info.st_mode) and info.st_size <= limit
        with os.fdopen(os.dup(fd), "rb") as stream:
            data = stream.read(limit + 1)
        assert len(data) <= limit
        return data
    finally:
        os.close(fd)


def key_present(reference):
    result = subprocess.run(
        ["/usr/bin/security", "find-generic-password", "-s", reference["service"],
         "-a", reference["account"]],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
        check=False,
    )
    assert result.returncode in (0, 44), "Keychain metadata query was inconclusive"
    return result.returncode == 0


def reference(manifest):
    assert manifest["channel"] == "dev"
    return {"service": "com.openresumetoolkit.dev.database",
            "account": f'install-{manifest["installId"]}.profile-{manifest["profileId"]}'}


def denied(path):
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except OSError as error:
        assert error.errno in (errno.EACCES, errno.EPERM), "Path did not fail with access denial"
        return True
    else:
        os.close(fd)
        return False


def main():
    assert len(sys.argv) == 2 and sys.argv[1] in ("before", "after")
    config = json.loads(read_bounded(Path(__file__).with_name("account-check-config.json"), 16384))
    assert os.getuid() == os.geteuid() == config["testUid"]
    assert Path.home() == Path("/Users/orttest"), "Use the real orttest login, never sudo"
    assert os.stat("/dev/console").st_uid == os.getuid(), "orttest must be the active console login"
    receipt = Path.home() / "ORT-M1-account-before.json"
    profiles = Path.home() / "Library/Application Support/com.openresumetoolkit.dev/profiles"
    current = json.loads(read_bounded(profiles / "default/profile.json", 16384))
    current_ref = reference(current)
    checks = {
        "developer_database_access_denied": denied(config["developerDatabase"]),
        "developer_keychain_file_access_denied": denied(config["developerKeychain"]),
        "developer_key_absent_from_test_account_search": not key_present(config["developerReference"]),
        "active_key_metadata_present": key_present(current_ref),
        "active_identity_differs_from_developer": current_ref != config["developerReference"],
        "shared_encrypted_backup_unchanged": hashlib.sha256(read_bounded(Path(config["backupPath"]), 64 * 1024 * 1024)).hexdigest() == config["backupSha256"],
    }
    markers = ["Synthetic M2 save-and-quit check", "Example Tester", "Synthetic test role"]
    for filename in ("profile.db", "profile.db-wal"):
        path = profiles / "default" / filename
        if not path.exists() and filename.endswith("-wal"):
            continue
        data = read_bounded(path, 64 * 1024 * 1024)
        checks[filename + "_private"] = stat.S_IMODE(path.stat().st_mode) == 0o600
        checks[filename + "_markers_absent"] = all(marker.encode(encoding) not in data for marker in markers for encoding in ("utf-8", "utf-16-le", "utf-16-be"))
        checks[filename + "_not_plain_sqlite"] = not data.startswith(b"SQLite format 3")
    if sys.argv[1] == "before":
        old_refs = [current_ref]
        safety = profiles / ".ort-restore-safety/profile.json"
        checks["restore_safety_copy_present"] = safety.is_file()
        if safety.is_file():
            safety_ref = reference(json.loads(read_bounded(safety, 16384)))
            checks["safety_key_metadata_present"] = key_present(safety_ref)
            checks["restored_key_differs_from_previous_test_key"] = safety_ref != current_ref
            old_refs.append(safety_ref)
        payload = {"checks": checks, "oldReferences": old_refs}
        # Refuse to overwrite evidence from an earlier attempt.
        fd = os.open(receipt, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "w") as stream:
            json.dump(payload, stream, indent=2)
    else:
        before = json.loads(read_bounded(receipt, 16384))
        checks["previous_checks_passed"] = all(before["checks"].values())
        checks["old_active_and_safety_keys_removed"] = all(not key_present(item) for item in before["oldReferences"])
        checks["fresh_active_key_identity"] = current_ref not in before["oldReferences"]
        checks["recovery_directories_and_markers_removed"] = not any((profiles / name).exists() for name in (
            ".ort-restore-safety", ".ort-restore-staged", ".ort-safety-delete-pending",
            ".ort-restore-pending.json", ".ort-delete-all-pending.json"))
        destination = Path.home() / "ORT-M1-account-after.json"
        fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "w") as stream:
            json.dump({"checks": checks}, stream, indent=2)
    print(json.dumps(checks, indent=2))
    assert all(checks.values()), "STOP: a qualification check failed; keep the evidence"
    print("ALL CHECKS PASSED (" + sys.argv[1] + ")")


if __name__ == "__main__":
    main()
