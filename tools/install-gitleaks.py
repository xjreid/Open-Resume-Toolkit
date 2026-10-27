#!/usr/bin/env python3
"""Install the pinned, SHA-256 verified scanner without package scripts."""
import hashlib
import io
import os
import pathlib
import platform
import re
import tarfile
import urllib.request
import zipfile

VERSION = "8.30.1"
RULES_SHA256 = "e163e53b9e7e8a8511e77271e2b323ed057759542a6d988258afe3a1fa329caf"
CHECKSUMS = {
    "darwin_arm64": "b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5",
    "darwin_x64": "dfe101a4db2255fc85120ac7f3d25e4342c3c20cf749f2c20a18081af1952709",
    "linux_arm64": "e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080",
    "linux_x64": "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb",
    "windows_arm64": "b95f5e4f5c425cedca7ee203d9afd29597e692c4924a12ed42f970537c72cc0f",
    "windows_x64": "d29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e",
}


def main():
    system = platform.system().lower()
    architecture = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "x64", "amd64": "x64"}.get(platform.machine().lower())
    target = f"{system}_{architecture}"
    if target not in CHECKSUMS:
        raise SystemExit("Unsupported Gitleaks platform")
    extension = "zip" if system == "windows" else "tar.gz"
    url = f"https://github.com/gitleaks/gitleaks/releases/download/v{VERSION}/gitleaks_{VERSION}_{target}.{extension}"
    with urllib.request.urlopen(url, timeout=60) as response:
        archive = response.read(64 * 1024 * 1024)
    if hashlib.sha256(archive).hexdigest() != CHECKSUMS[target]:
        raise SystemExit("Gitleaks archive checksum mismatch")
    rules_url = f"https://raw.githubusercontent.com/gitleaks/gitleaks/v{VERSION}/config/gitleaks.toml"
    with urllib.request.urlopen(rules_url, timeout=60) as response:
        rules = response.read(2 * 1024 * 1024)
    if hashlib.sha256(rules).hexdigest() != RULES_SHA256:
        raise SystemExit("Gitleaks rules checksum mismatch")
    # Upstream skips SVG, lockfiles, scanner config, and many document/binary
    # extensions by filename. Retain every detector and value-based exception,
    # but remove that global path exemption so these files are inspected too.
    hardened_rules, replacements = re.subn(
        r"(?ms)^paths = \[.*?^\]\n", "paths = []\n", rules.decode("utf-8"), count=1
    )
    if replacements != 1:
        raise SystemExit("Unexpected Gitleaks global path-exclusion format")
    executable = "gitleaks.exe" if system == "windows" else "gitleaks"
    # Read only the expected binary; never extract archive-supplied paths.
    if extension == "zip":
        with zipfile.ZipFile(io.BytesIO(archive)) as package:
            data = package.read(executable)
    else:
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as package:
            member = package.getmember(executable)
            if not member.isfile():
                raise SystemExit("Scanner is not a regular file")
            data = package.extractfile(member).read()
    destination = pathlib.Path(__file__).resolve().parents[1] / "target/security-tools" / executable
    destination.parent.mkdir(parents=True, exist_ok=True)
    (destination.parent / "gitleaks-rules.toml").write_bytes(hardened_rules.encode("utf-8"))
    temporary = destination.with_suffix(f".download-{os.getpid()}")
    temporary.write_bytes(data)
    temporary.chmod(0o755)
    temporary.replace(destination)
    print(f"Installed checksum-verified Gitleaks {VERSION}")


if __name__ == "__main__":
    main()
