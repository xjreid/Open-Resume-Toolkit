#!/usr/bin/env python3
"""Assemble a signed local M2 candidate with exact nested parser identity.

Creates target/m2-candidate only. Never installs or launches the app and never
exports a signing key. Use an existing local development signing identity.
"""
import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--identity", required=True)
    args = parser.parse_args()
    if len(args.identity) != 40 or not all(c in "0123456789abcdefABCDEF" for c in args.identity):
        raise SystemExit("Use the exact fingerprint of an existing local signing identity")
    run(["python3", "tools/package-parser-helper.py"])
    helper_manifest = json.loads((ROOT / "target/parser-helper-package.json").read_text())
    environment = dict(os.environ,
                       ORT_PARSER_HELPER_SHA256=helper_manifest["executableSha256"],
                       ORT_PARSER_HELPER_CDHASH=helper_manifest["cdhash"])
    override = json.dumps({"bundle": {"active": True, "targets": ["app"],
                                      "macOS": {"signingIdentity": args.identity, "hardenedRuntime": True}}})
    run(["pnpm", "--filter", "@ort/desktop", "tauri", "build", "--config", override, "--bundles", "app"], env=environment)
    original = ROOT / "target/release/bundle/macos/Open Resume Toolkit Dev.app"
    destination = ROOT / "target/m2-candidate/Open Resume Toolkit Dev.app"
    if destination.exists():
        raise SystemExit("Candidate already exists; preserve it or choose a fresh worktree before rebuilding")
    shutil.copytree(original, destination)
    helper = destination / "Contents/Helpers/ORT Parser Helper.app"
    shutil.copytree(ROOT / "target/ORT Parser Helper.app", helper)
    executable = helper / "Contents/MacOS/ort-parser-helper"
    assert hashlib.sha256(executable.read_bytes()).hexdigest() == helper_manifest["executableSha256"]
    # Sign only the outer bundle: never recursively re-sign the pinned helper.
    run(["/usr/bin/codesign", "--force", "--sign", args.identity, "--options", "runtime", str(destination)])
    run(["/usr/bin/codesign", "--verify", "--deep", "--strict", str(destination)])
    assert hashlib.sha256(executable.read_bytes()).hexdigest() == helper_manifest["executableSha256"]
    desktop = destination / "Contents/MacOS/ort-desktop"
    manifest = {"schemaVersion": 1, "developmentOnly": True,
                "desktopSha256": hashlib.sha256(desktop.read_bytes()).hexdigest(),
                "parser": helper_manifest, "signingIdentityFingerprint": args.identity,
                "nativeAcceptance": "pending-standard-account-and-accessibility-matrix"}
    (destination.parent / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print("M2 candidate assembled and signatures verified; installed application unchanged")


if __name__ == "__main__":
    main()
