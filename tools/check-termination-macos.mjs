// Synthetic AppKit lifecycle only: no ORT profile, Keychain or system logout.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { arch, release } from "node:os";
import { join } from "node:path";

if (process.platform !== "darwin" || process.getuid() === 0)
  throw new Error("This check requires a non-root macOS session.");
mkdirSync("target/native-probes", { recursive: true });
const output = mkdtempSync("target/native-probes/termination-");
const sources = [
  "crates/ort-macos-lifecycle/native/termination.m",
  "tools/native/macos-termination-probe/probe.m",
];
const flags = ["-fobjc-arc", "-Wall", "-Wextra", "-Werror"];
const executable = join(output, "termination-probe");
execFileSync(
  "xcrun",
  ["clang", ...flags, "-framework", "AppKit", ...sources, "-o", executable],
  { timeout: 60000, stdio: "pipe" },
);
for (const [index, source] of sources.entries()) {
  const plist = join(output, `analysis-${index}.plist`);
  const diagnostics = execFileSync(
    "xcrun",
    ["clang", ...flags, "--analyze", source, "-o", plist],
    { timeout: 60000, encoding: "utf8", stdio: "pipe" },
  );
  // clang's analyzer can exit zero when it reports a defect.
  execFileSync(
    "/usr/bin/python3",
    [
      "-c",
      "import plistlib,sys; p=plistlib.load(open(sys.argv[1],'rb')); assert not p['diagnostics'], 'Native analyzer findings'",
      plist,
    ],
    { timeout: 10000, stdio: "pipe" },
  );
  if (diagnostics.trim()) throw new Error("Unexpected native analyzer output.");
}
const result = execFileSync(executable, [], {
  timeout: 15000,
  encoding: "utf8",
  stdio: "pipe",
});
if (!result.startsWith("PASS:"))
  throw new Error("Native termination check did not report success.");
writeFileSync(
  join(output, "report.json"),
  JSON.stringify(
    {
      architecture: arch(),
      osRelease: release(),
      sourceHashes: Object.fromEntries(
        sources.map((path) => [
          path,
          createHash("sha256").update(readFileSync(path)).digest("hex"),
        ]),
      ),
      appKitSyntheticTerminationPassed: true,
      staticAnalysisFindings: 0,
      installedEditorQualified: false,
      systemLogoutTested: false,
    },
    null,
    2,
  ) + "\n",
);
console.log(result.trim());
console.log(`Content-free report: ${output}/report.json`);
