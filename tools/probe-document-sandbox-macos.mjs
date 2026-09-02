// Builds a separate, ad-hoc signed synthetic XPC probe. Never packages it in ORT.
import {
  chmodSync,
  copyFileSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { interpretProbe } from "./lib/document-sandbox-report.mjs";

if (process.platform !== "darwin") {
  throw new Error("This optional native probe runs only on macOS.");
}
if (process.argv.slice(2).some((arg) => arg !== "--build-only")) {
  throw new Error(
    "Only --build-only is supported; probe targets are not configurable.",
  );
}
const root = realpathSync(fileURLToPath(new URL("..", import.meta.url)));
const source = join(root, "tools/native/macos-document-probe");
const builds = join(root, "target/native-probes");
mkdirSync(builds, { recursive: true });
const build = mkdtempSync(join(realpathSync(builds), "macos-document-"));
const app = join(build, "ORT Document Sandbox Probe.app");
const service = join(app, "Contents/XPCServices/DocumentProbe.xpc");
const hostBinary = join(app, "Contents/MacOS/ort-document-probe");
const workerBinary = join(service, "Contents/MacOS/ort-document-probe-worker");
mkdirSync(dirname(hostBinary), { recursive: true });
mkdirSync(dirname(workerBinary), { recursive: true });
copyFileSync(join(source, "Host-Info.plist"), join(app, "Contents/Info.plist"));
copyFileSync(
  join(source, "Worker-Info.plist"),
  join(service, "Contents/Info.plist"),
);

function run(executable, args, extra = {}) {
  return execFileSync(executable, args, {
    cwd: root,
    encoding: "utf8",
    timeout: 60_000,
    maxBuffer: 64 * 1024,
    ...extra,
  });
}
const sdk = run("/usr/bin/xcrun", ["--show-sdk-path"]).trim();
for (const [file, binary] of [
  ["harness.c", hostBinary],
  ["worker.c", workerBinary],
]) {
  run("/usr/bin/xcrun", [
    "clang",
    "-std=c11",
    "-fblocks",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-O1",
    "-isysroot",
    sdk,
    "-framework",
    "CoreFoundation",
    "-framework",
    "Security",
    join(source, file),
    "-o",
    binary,
  ]);
}
run("/usr/bin/codesign", [
  "--force",
  "--sign",
  "-",
  "--options",
  "runtime",
  "--entitlements",
  join(source, "Worker.entitlements"),
  service,
]);
run("/usr/bin/codesign", [
  "--force",
  "--sign",
  "-",
  "--options",
  "runtime",
  app,
]);
run("/usr/bin/codesign", ["--verify", "--deep", "--strict", app]);

if (process.argv.includes("--build-only")) {
  console.log(`Built and verified synthetic probe: ${app}`);
  process.exit(0);
}

// No user-supplied paths, document data or credentials are accepted. Retain
// binaries/report for inspection; remove only this fresh, validated fixture root.
const tempParent = realpathSync("/private/tmp");
const fixtures = mkdtempSync(join(tempParent, "ort-document-sandbox-"));
chmodSync(fixtures, 0o700);
try {
  writeFileSync(join(fixtures, "input.txt"), "ORT_SYNTHETIC_DESCRIPTOR_V1\n", {
    flag: "wx",
    mode: 0o600,
  });
  writeFileSync(join(fixtures, "sibling.txt"), "ORT_SYNTHETIC_FORBIDDEN_V1\n", {
    flag: "wx",
    mode: 0o600,
  });
  symlinkSync(join(fixtures, "sibling.txt"), join(fixtures, "sibling-link"));
  const output = run(hostBinary, [fixtures], {
    timeout: 20_000,
    maxBuffer: 8 * 1024,
    // The native harness gets no tokens or user app configuration from this shell.
    env: { PATH: "/usr/bin:/bin", TMPDIR: "/private/tmp" },
  });
  const measurement = JSON.parse(output);
  const conclusions = interpretProbe(measurement);
  const report = {
    schemaVersion: 1,
    createdAt: new Date().toISOString(),
    platform: "macos",
    architecture: process.arch,
    osVersion: run("/usr/bin/sw_vers", ["-productVersion"]).trim(),
    signing: "ad-hoc-hardened-runtime-separate-app-sandbox-helper",
    signatureVerified: true,
    helperSha256: createHash("sha256")
      .update(readFileSync(workerBinary))
      .digest("hex"),
    hostSha256: createHash("sha256")
      .update(readFileSync(hostBinary))
      .digest("hex"),
    measurement,
    conclusions,
  };
  const reportPath = join(build, "report.json");
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, {
    flag: "wx",
    mode: 0o600,
  });
  console.log(JSON.stringify(conclusions, null, 2));
  console.log(`Synthetic measurements saved: ${reportPath}`);
  console.log(
    "Probe completion is not full containment proof. Document import remains disabled.",
  );
  if (
    !conclusions.filesystemIsolationPassed ||
    !conclusions.loopbackConnectDenied
  ) {
    throw new Error(
      "The measured filesystem/loopback subset failed; inspect the saved report.",
    );
  }
} finally {
  if (
    dirname(fixtures) !== tempParent ||
    !basename(fixtures).startsWith("ort-document-sandbox-") ||
    lstatSync(fixtures).isSymbolicLink() ||
    realpathSync(fixtures) !== fixtures
  ) {
    throw new Error("Refusing cleanup of an unexpected fixture directory.");
  }
  rmSync(fixtures, { recursive: true });
}
