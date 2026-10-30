import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const packageJson = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url)),
);
const pinnedNode = readFileSync(
  new URL("../.nvmrc", import.meta.url),
  "utf8",
).trim();
const required = {
  node: pinnedNode,
  pnpm: packageJson.engines.pnpm,
  rustc: "1.98.0",
  cargo: "1.98.0",
  ...(process.platform === "win32" ? {} : { just: "1" }),
};

const failures = [];

for (const [command, expected] of Object.entries(required)) {
  try {
    const localPnpm = resolve(
      import.meta.dirname,
      "../target/windows-toolchain/pnpm/bin/pnpm.cjs",
    );
    // Execute JavaScript through Node directly; execFile cannot safely use a
    // Windows .cmd shim as a native executable. No shell interpolation is needed.
    const invocation =
      command === "node"
        ? [process.execPath, ["--version"]]
        : process.platform === "win32" &&
            command === "pnpm" &&
            existsSync(localPnpm)
          ? [process.execPath, [localPnpm, "--version"]]
          : [command, ["--version"]];
    const output = execFileSync(...invocation, {
      encoding: "utf8",
    }).trim();
    if (!output.includes(expected)) {
      failures.push(`${command}: expected ${expected}, found ${output}`);
    }
  } catch {
    failures.push(`${command}: not installed (expected ${expected})`);
  }
}

if (failures.length > 0) {
  console.error("Open Resume Toolkit prerequisites are not ready:\n");
  for (const failure of failures) console.error(`- ${failure}`);
  console.error(
    "\nSee DEVELOPMENT.md for platform-specific setup instructions.",
  );
  process.exit(1);
}

console.log("Pinned toolchains are available.");
