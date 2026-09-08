// Measurements only. Successful execution never qualifies native containment.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { arch, release } from "node:os";

if (process.platform !== "darwin" || process.getuid() === 0)
  throw new Error("Run this measurement as an ordinary macOS user.");
mkdirSync("target/native-probes", { recursive: true });
const output = mkdtempSync("target/native-probes/worker-resources-");
const measurements = {};
const sourceHashes = {};
for (const name of ["memory_limit", "cpu_limit"]) {
  const source = `tools/native/macos-document-probe/${name}.c`;
  const executable = join(output, name);
  // Sanitizers reserve virtual memory and would contaminate this measurement.
  execFileSync(
    "xcrun",
    [
      "clang",
      "-std=c11",
      "-Wall",
      "-Wextra",
      "-Werror",
      "-pedantic",
      source,
      "-o",
      executable,
    ],
    { timeout: 60000, stdio: "pipe" },
  );
  measurements[name] = JSON.parse(
    execFileSync(executable, [], {
      timeout: 20000,
      encoding: "utf8",
      stdio: "pipe",
    }),
  );
  sourceHashes[source] = createHash("sha256")
    .update(readFileSync(source))
    .digest("hex");
}
const report = {
  platform: "macos",
  architecture: arch(),
  osRelease: release(),
  sourceHashes,
  measurements,
  fullContainmentProven: false,
  importEnabled: false,
};
writeFileSync(
  join(output, "report.json"),
  JSON.stringify(report, null, 2) + "\n",
);
console.log(JSON.stringify(report, null, 2));
console.log(`Measurement report: ${output}/report.json`);
console.log(
  "Measurement complete. This is not a memory or full containment pass.",
);
