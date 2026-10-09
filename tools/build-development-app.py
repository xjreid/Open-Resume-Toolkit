#!/usr/bin/env python3
"""Build a complete ad-hoc signed development app, without installing or launching it."""
import datetime
import hashlib
import json
import os
import pathlib
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


def digest(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def main():
    parser_manifest = ROOT / "target/parser-helper-package.json"
    if not parser_manifest.exists():
        run(["python3", "tools/package-parser-helper.py"])
    parser = json.loads(parser_manifest.read_text())
    parser_bundle = ROOT / "target/ORT Parser Helper.app"
    assert digest(parser_bundle / "Contents/MacOS/ort-parser-helper") == parser["executableSha256"]
    run(["/usr/bin/codesign", "--verify", "--strict", str(parser_bundle)])
    run(["python3", "tools/package-codex-installer.py"])
    installer = json.loads((ROOT / "target/codex-installer-package.json").read_text())
    environment = dict(os.environ,
                       ORT_PARSER_HELPER_SHA256=parser["executableSha256"],
                       ORT_PARSER_HELPER_CDHASH=parser["cdhash"],
                       ORT_CODEX_INSTALLER_SHA256=installer["executableSha256"],
                       pnpm_config_verify_deps_before_run="false")
    config = json.dumps({"bundle": {"active": True, "targets": ["app"],
                                    "macOS": {"signingIdentity": "-", "hardenedRuntime": True}}})
    run(["pnpm", "--filter", "@ort/desktop", "tauri", "build", "--features", "dev-browser-bridge", "--config", config, "--bundles", "app"], env=environment)
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    output = ROOT / "target" / ("codex-installer-update-" + stamp)
    output.mkdir()
    candidate = output / "Open Resume Toolkit Dev.app"
    shutil.copytree(ROOT / "target/release/bundle/macos/Open Resume Toolkit Dev.app", candidate)
    helpers = candidate / "Contents/Helpers"
    shutil.copytree(parser_bundle, helpers / "ORT Parser Helper.app")
    shutil.copytree(ROOT / "target/ORT Codex Installer.app", helpers / "ORT Codex Installer.app")
    run(["/usr/bin/codesign", "--force", "--sign", "-", "--options", "runtime", str(candidate)])
    run(["/usr/bin/codesign", "--verify", "--deep", "--strict", str(candidate)])
    assert digest(helpers / "ORT Parser Helper.app/Contents/MacOS/ort-parser-helper") == parser["executableSha256"]
    assert digest(helpers / "ORT Codex Installer.app/Contents/MacOS/ort-codex-install") == installer["executableSha256"]
    manifest = {"schemaVersion": 1, "developmentOnly": True, "candidatePath": str(candidate),
                "desktopSha256": digest(candidate / "Contents/MacOS/ort-desktop"),
                "parser": parser, "codexInstaller": installer, "signatureVerified": True,
                "appLaunched": False, "runtimeInstalled": False, "liveAccountUsed": False}
    (output / "build-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (ROOT / "target/latest-codex-installer-update.txt").write_text(str(output) + "\n")
    print("Development app built and verified, without installation or launch: " + str(candidate))


if __name__ == "__main__":
    main()
