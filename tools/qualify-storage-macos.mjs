// Opt-in native tests only. This harness never changes the default Keychain,
// exports keys, or modifies either account's installed application/profile.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { resolve, join } from "node:path";

if (process.platform !== "darwin" || process.arch !== "arm64") {
  throw new Error("This qualification requires macOS arm64.");
}
const identity = process.argv[2] ?? "ORT Local Test Signing";
const root = resolve(import.meta.dirname, "..");
const evidence = join(root, "target/m1-qualification");
mkdirSync(evidence, { recursive: true });
const run = mkdtempSync(join(evidence, "native-storage-"));
const report = {
  date: new Date().toISOString(),
  scope:
    "signed native test helper; disposable synthetic SQLCipher profiles and real platform-test Keychain items; not installed-app UI or cross-account proof",
  tests: [],
};
const execute = (file, args, options = {}) =>
  execFileSync(file, args, {
    cwd: root,
    encoding: "utf8",
    timeout: 120_000,
    maxBuffer: 8 * 1024 * 1024,
    ...options,
  });
let attached = false;
const mount = join(run, "volume");
try {
  const build = execute("cargo", [
    "test",
    "--locked",
    "-p",
    "ort-storage",
    "--lib",
    "--no-run",
    "--message-format=json",
  ]);
  const artifact = build
    .trim()
    .split("\n")
    .map((line) => JSON.parse(line))
    .find(
      (item) =>
        item.reason === "compiler-artifact" &&
        item.target.name === "ort_storage" &&
        item.profile.test &&
        item.executable,
    );
  if (!artifact) throw new Error("No native test executable was built.");
  const binary = join(run, "ort-storage-native-tests");
  copyFileSync(artifact.executable, binary);
  execute("/usr/bin/codesign", [
    "--force",
    "--sign",
    identity,
    "--identifier",
    "com.openresumetoolkit.qualification.storage",
    "--options",
    "runtime",
    "--timestamp=none",
    binary,
  ]);
  execute("/usr/bin/codesign", ["--verify", "--strict", "--verbose=2", binary]);
  report.helperSha256 = createHash("sha256")
    .update(readFileSync(binary))
    .digest("hex");
  report.signingIdentity = identity;
  execute("/usr/bin/codesign", ["-d", "-r-", binary], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  const test = (name, extraEnvironment = {}) => {
    const output = execute(
      binary,
      ["--exact", `native_qualification::${name}`, "--ignored", "--nocapture"],
      {
        env: {
          ...process.env,
          ORT_RUN_OS_VAULT_TESTS: "1",
          ...extraEnvironment,
        },
      },
    );
    writeFileSync(join(run, `${name}.log`), output);
    report.tests.push({ name, status: "passed" });
    process.stdout.write(output);
  };
  if (!process.argv.includes("--low-disk-only")) {
    test("native_storage_failure_matrix");
    test("native_m2_recovery_crashes");
  }
  if (process.argv.includes("--installed-key-probe")) {
    test("native_untrusted_process_cannot_load_installed_key");
  }
  // Fixed-size, private disk image only. The Rust fill loop has an independent
  // 80 MiB cap and requires the image's harness marker before writing.
  const image = join(run, "bounded-storage.dmg");
  execute("/usr/bin/hdiutil", [
    "create",
    "-size",
    "64m",
    "-fs",
    process.argv.includes("--apfs") ? "APFS" : "HFS+",
    "-volname",
    "ORT-M1-disposable",
    "-type",
    "UDIF",
    image,
  ]);
  mkdirSync(mount);
  execute("/usr/bin/hdiutil", [
    "attach",
    "-nobrowse",
    "-mountpoint",
    mount,
    image,
  ]);
  attached = true;
  writeFileSync(
    join(mount, "ort-m1-bounded-volume"),
    "64-MiB-disposable-image",
  );
  test("native_low_disk", { ORT_M1_BOUNDED_VOLUME: mount });
} catch (error) {
  report.failure =
    "Native qualification failed; see failure.log. No milestone signoff.";
  writeFileSync(
    join(run, "failure.log"),
    `${error.stack}\n${error.stdout ?? ""}\n${error.stderr ?? ""}`,
  );
  process.exitCode = 1;
} finally {
  if (attached) {
    try {
      execute("/usr/bin/hdiutil", ["detach", mount]);
      report.diskImageDetached = true;
    } catch {
      report.diskImageDetached = false;
      process.exitCode = 1;
    }
  }
  writeFileSync(
    join(run, "report.json"),
    `${JSON.stringify(report, null, 2)}\n`,
  );
  console.log(`Native evidence: ${run}`);
}
