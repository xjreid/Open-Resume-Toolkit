"""Source-only packaging checks. No compiler, installer or application is run."""
import importlib.util
import hashlib
import json
import pathlib
import struct
import tempfile
import unittest
from unittest import mock
from types import SimpleNamespace

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("windows_build", ROOT / "tools/build-development-windows.py")
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)


class WindowsBuildTests(unittest.TestCase):
    def test_rebuilding_preserves_each_manifest_installer_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "shared-installer.exe"
            builds = [root / "first", root / "second"]
            manifests = []
            for index, build in enumerate(builds):
                build.mkdir()
                source.write_bytes(f"SYNTHETIC BUILD {index}".encode())
                retained = BUILD.retain_installer(source, build)
                manifests.append({"installerPath": retained, "installerSha256": BUILD.digest(retained)})
            source.write_bytes(b"LATER BUILD")
            self.assertNotEqual(manifests[0]["installerSha256"], manifests[1]["installerSha256"])
            for manifest in manifests:
                self.assertEqual(BUILD.digest(manifest["installerPath"]), manifest["installerSha256"])
            with self.assertRaises(FileExistsError):
                BUILD.retain_installer(source, builds[0])
            self.assertEqual(BUILD.digest(manifests[0]["installerPath"]), manifests[0]["installerSha256"])

    def test_changed_installer_is_not_retained_as_a_verified_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "shared-installer.exe"
            output = root / "retained"
            output.mkdir()
            source.write_bytes(b"SYNTHETIC")
            with mock.patch.object(BUILD, "digest", side_effect=["before", "changed"]):
                with self.assertRaisesRegex(ValueError, "Installer changed"):
                    BUILD.retain_installer(source, output)
            self.assertFalse((output / source.name).exists())

    def test_nsis_payload_hash_accounts_for_the_pinned_bundle_marker_and_refuses_ambiguity(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "synthetic-bundle-marker.bin"
            original = b"prefix__TAURI_BUNDLE_TYPE_VAR_UNKsuffix"
            path.write_bytes(original)
            expected = hashlib.sha256(b"prefix__TAURI_BUNDLE_TYPE_VAR_NSSsuffix").hexdigest()
            self.assertEqual(BUILD.nsis_desktop_digest(path), expected)
            self.assertEqual(path.read_bytes(), original)
            for ambiguous in [b"no marker", original + original,
                              original + b"__TAURI_BUNDLE_TYPE_VAR_NSS"]:
                path.write_bytes(ambiguous)
                with self.assertRaises(ValueError):
                    BUILD.nsis_desktop_digest(path)

    def test_native_program_resolution_uses_the_prepared_sdk_path(self):
        environment = {"PATH":"C:/Pinned tools"}
        with mock.patch.object(BUILD.sys, "platform", "win32"), \
             mock.patch.object(BUILD.shutil, "which", return_value="C:/Pinned tools/cargo.exe") as resolve, \
             mock.patch.object(BUILD.subprocess, "run", return_value=SimpleNamespace(stdout="ok")) as execute:
            self.assertEqual(BUILD.run(["cargo", "--version"], environment, capture=True), "ok")
        resolve.assert_called_once_with("cargo", path=environment["PATH"])
        self.assertEqual(execute.call_args.args[0], ["C:/Pinned tools/cargo.exe", "--version"])

    def test_batch_wrappers_are_not_executed_as_native_programs(self):
        with self.assertRaises(ValueError):
            BUILD.run([str(ROOT / "synthetic.cmd")], {})

    def test_windows_environment_is_case_insensitive_and_clears_unrelated_channel_pins(self):
        snapshot = {"Path": "C:/Synthetic tool path", "Ort_Chrome_Extension_Id": "production",
                    "Ort_Parser_Helper_Cdhash": "macos", "ORT_CODEX_INSTALLER_SHA256": "macos",
                    "ORT_PARSER_HELPER_SHA256": "previous", "Ort_Windows_Parser_Policy": "previous"}
        with mock.patch.object(BUILD, "run", return_value=json.dumps(snapshot)):
            environment = BUILD.environment()
        self.assertEqual(environment["PATH"], snapshot["Path"])
        self.assertEqual(environment["CARGO_TARGET_DIR"], str(ROOT / "target"))
        self.assertNotIn("ORT_CHROME_EXTENSION_ID", environment)
        self.assertNotIn("ORT_PARSER_HELPER_CDHASH", environment)
        self.assertNotIn("ORT_CODEX_INSTALLER_SHA256", environment)
        self.assertNotIn("ORT_PARSER_HELPER_SHA256", environment)
        self.assertNotIn("ORT_WINDOWS_PARSER_POLICY", environment)

    def test_installer_stays_per_user_and_preserves_fixed_helper_paths(self):
        helpers = pathlib.Path("C:/Synthetic space/工具/helpers")
        bundle = BUILD.bundle_config(helpers)["bundle"]
        self.assertEqual(bundle["targets"], ["nsis"])
        self.assertEqual(bundle["windows"]["nsis"]["installMode"], "currentUser")
        self.assertEqual(bundle["windows"]["webviewInstallMode"], {"type": "skip"})
        self.assertEqual(bundle["resources"][str(helpers / "ort-parser-helper.exe")], "helpers/ort-parser-helper.exe")
        self.assertNotIn("externalBin", bundle)  # no alternate unsupervised sidecar path

    def test_headless_parser_and_console_native_host_have_distinct_pe_requirements(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "synthetic-pe.bin"
            data = bytearray(256)
            data[:2] = b"MZ"
            struct.pack_into("<I", data, 0x3C, 128)
            data[128:132] = b"PE\0\0"
            struct.pack_into("<H", data, 132, 0x8664)
            struct.pack_into("<H", data, 152, 0x20B)
            struct.pack_into("<H", data, 220, 2)
            path.write_bytes(data)
            BUILD.pe_subsystem(path, 2)
            with self.assertRaises(ValueError):
                BUILD.pe_subsystem(path, 3)
            struct.pack_into("<H", data, 132, 0xAA64)
            path.write_bytes(data)
            with self.assertRaises(ValueError):
                BUILD.pe_subsystem(path, 2)
            struct.pack_into("<I", data, 0x3C, 0xFFFFFFF0)
            path.write_bytes(data)
            with self.assertRaises(ValueError):
                BUILD.pe_subsystem(path, 2)

    def test_desktop_assets_can_exceed_helper_limit_without_relaxing_helper_policy(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "synthetic-large-desktop.bin"
            data = bytearray(256)
            data[:2] = b"MZ"
            struct.pack_into("<I", data, 0x3C, 128)
            data[128:132] = b"PE\0\0"
            struct.pack_into("<H", data, 132, 0x8664)
            struct.pack_into("<H", data, 152, 0x20B)
            struct.pack_into("<H", data, 220, 2)
            with path.open("wb") as executable:
                executable.write(data)
                executable.truncate(65 * 1024 * 1024)
            with self.assertRaises(ValueError):
                BUILD.pe_subsystem(path, 2)
            BUILD.pe_subsystem(path, 2, maximum_bytes=256 * 1024 * 1024)
            with path.open("r+b") as executable:
                executable.truncate(256 * 1024 * 1024 + 1)
            with self.assertRaises(ValueError):
                BUILD.pe_subsystem(path, 2, maximum_bytes=256 * 1024 * 1024)


if __name__ == "__main__":
    unittest.main()
