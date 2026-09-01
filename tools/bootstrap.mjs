import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

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
  just: "1",
};

const failures = [];

for (const [command, expected] of Object.entries(required)) {
  try {
    const output = execFileSync(command, ["--version"], {
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
