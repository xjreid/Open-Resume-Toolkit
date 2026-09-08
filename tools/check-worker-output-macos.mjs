// Synthetic native I/O regression only; does not launch a parser or an XPC service.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { arch, release } from "node:os";

if (process.platform !== "darwin")
  throw new Error("This native check requires macOS.");
mkdirSync("target/native-probes", { recursive: true });
const output = mkdtempSync("target/native-probes/worker-output-");
const sources = ["worker_output.c", "worker_output_test.c"].map((name) =>
  join("crates/ort-platform/native/macos", name),
);
const executable = join(output, "worker-output-test");
execFileSync(
  "xcrun",
  [
    "clang",
    "-std=c11",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-pedantic",
    "-fsanitize=address,undefined",
    "-g",
    ...sources,
    "-o",
    executable,
  ],
  { timeout: 60000, stdio: "pipe" },
);
const result = execFileSync(executable, [], {
  timeout: 10000,
  encoding: "utf8",
  stdio: "pipe",
});
if (!result.startsWith("PASS:"))
  throw new Error("Native output-reader check did not report success.");
const hashes = Object.fromEntries(
  [...sources, join("crates/ort-platform/native/macos", "worker_output.h")].map(
    (path) => [
      path,
      createHash("sha256").update(readFileSync(path)).digest("hex"),
    ],
  ),
);
writeFileSync(
  join(output, "report.json"),
  JSON.stringify(
    {
      platform: "macos",
      architecture: arch(),
      osRelease: release(),
      sourceHashes: hashes,
      sanitizers: ["address", "undefined"],
      ioSubsetPassed: true,
      fullContainmentProven: false,
      importEnabled: false,
    },
    null,
    2,
  ) + "\n",
);
console.log(result.trim());
console.log(`Content-free report: ${output}/report.json`);
