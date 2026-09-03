// Synthetic signed supervisor/direct-child experiment. No product integration.
import {
  chmodSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  interpretLifecycle,
  lifecycleCases,
} from "./lib/document-lifecycle-report.mjs";

if (process.platform !== "darwin" || process.getuid() === 0)
  throw new Error("This probe requires non-root macOS.");
if (process.argv.slice(2).some((arg) => arg !== "--build-only"))
  throw new Error("Only --build-only is accepted.");
const root = realpathSync(fileURLToPath(new URL("..", import.meta.url)));
const builds = join(root, "target/native-probes");
mkdirSync(builds, { recursive: true });
const build = mkdtempSync(join(realpathSync(builds), "macos-lifecycle-"));
const app = join(build, "ORT Lifecycle Probe.app");
const service = join(app, "Contents/XPCServices/LifecycleProbe.xpc");
const binaries = {
  HOST: join(app, "Contents/MacOS/ort-lifecycle-host"),
  SUPERVISOR: join(service, "Contents/MacOS/ort-lifecycle-supervisor"),
  CHILD: join(service, "Contents/MacOS/ort-lifecycle-child"),
};
for (const path of Object.values(binaries))
  mkdirSync(dirname(path), { recursive: true });
const plist = (content) =>
  `<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict>${content}</dict></plist>\n`;
const bundle = (identifier, executable, type, extra) =>
  plist(
    `<key>CFBundleIdentifier</key><string>${identifier}</string><key>CFBundleExecutable</key><string>${executable}</string><key>CFBundlePackageType</key><string>${type}</string><key>CFBundleVersion</key><string>1</string>${extra}`,
  );
writeFileSync(
  join(app, "Contents/Info.plist"),
  bundle(
    "com.openresumetoolkit.lifecycle-probe",
    "ort-lifecycle-host",
    "APPL",
    "<key>LSUIElement</key><true/>",
  ),
  { flag: "wx" },
);
writeFileSync(
  join(service, "Contents/Info.plist"),
  bundle(
    "com.openresumetoolkit.lifecycle-probe.supervisor",
    "ort-lifecycle-supervisor",
    "XPC!",
    "<key>XPCService</key><dict><key>ServiceType</key><string>Application</string></dict>",
  ),
  { flag: "wx" },
);
const supervisorEntitlements = join(build, "supervisor.entitlements");
const childEntitlements = join(build, "child.entitlements");
writeFileSync(
  supervisorEntitlements,
  plist("<key>com.apple.security.app-sandbox</key><true/>"),
  { flag: "wx" },
);
writeFileSync(
  childEntitlements,
  plist(
    "<key>com.apple.security.app-sandbox</key><true/><key>com.apple.security.inherit</key><true/>",
  ),
  { flag: "wx" },
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
for (const [role, binary] of Object.entries(binaries)) {
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
    `-DPROBE_${role}`,
    join(root, "tools/native/macos-lifecycle-probe/probe.c"),
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
  "--identifier",
  "com.openresumetoolkit.lifecycle-probe.child",
  "--entitlements",
  childEntitlements,
  binaries.CHILD,
]);
run("/usr/bin/codesign", [
  "--force",
  "--sign",
  "-",
  "--options",
  "runtime",
  "--entitlements",
  supervisorEntitlements,
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
  console.log(`Built and verified synthetic lifecycle probe: ${app}`);
  process.exit(0);
}
const fixtures = mkdtempSync("/private/tmp/ort-lifecycle-");
chmodSync(fixtures, 0o700);
try {
  const input = join(fixtures, "input.txt");
  writeFileSync(input, "ORT_SYNTHETIC_DESCRIPTOR_V1\n", {
    mode: 0o600,
    flag: "wx",
  });
  const measurements = lifecycleCases.map((name, index) => {
    console.log(`lifecycle-probe: ${name}`);
    return JSON.parse(
      run(binaries.HOST, [input, String(index)], {
        timeout: 10_000,
        maxBuffer: 8192,
        env: { PATH: "/usr/bin:/bin", TMPDIR: "/private/tmp" },
      }),
    );
  });
  const report = {
    schemaVersion: 1,
    createdAt: new Date().toISOString(),
    architecture: process.arch,
    osVersion: run("/usr/bin/sw_vers", ["-productVersion"]).trim(),
    signing: "adhoc-hardened-runtime-sandboxed-supervisor-inherited-child",
    signatureVerified: true,
    hashes: Object.fromEntries(
      Object.entries(binaries).map(([role, path]) => [
        role,
        createHash("sha256").update(readFileSync(path)).digest("hex"),
      ]),
    ),
    measurements,
  };
  // Retain failing synthetic measurements too; interpretation never weakens a gate.
  const reportPath = join(build, "report.json");
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, {
    flag: "wx",
    mode: 0o600,
  });
  console.log(JSON.stringify(report, null, 2));
  console.log(`Synthetic lifecycle evidence saved: ${reportPath}`);
  console.log(JSON.stringify(interpretLifecycle(measurements), null, 2));
} finally {
  if (
    dirname(fixtures) !== "/private/tmp" ||
    !basename(fixtures).startsWith("ort-lifecycle-") ||
    lstatSync(fixtures).isSymbolicLink() ||
    realpathSync(fixtures) !== fixtures
  )
    throw new Error("Refusing unexpected fixture cleanup.");
  rmSync(fixtures, { recursive: true });
}
