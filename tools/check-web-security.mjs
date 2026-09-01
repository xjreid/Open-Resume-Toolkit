import { readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join, relative } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const scanRoots = ["apps/desktop/src", "apps/extension/src"];
const sourceExtensions = new Set([
  ".css",
  ".html",
  ".js",
  ".jsx",
  ".ts",
  ".tsx",
]);
const forbidden = [
  [/dangerouslySetInnerHTML/g, "raw React HTML injection"],
  [/\beval\s*\(/g, "dynamic code evaluation"],
  [/new\s+Function\s*\(/g, "dynamic Function construction"],
  [/https?:\/\//g, "remote web asset or endpoint"],
];
const failures = [];

function visit(path) {
  for (const entry of readdirSync(path)) {
    const candidate = join(path, entry);
    if (statSync(candidate).isDirectory()) {
      visit(candidate);
      continue;
    }
    if (!sourceExtensions.has(extname(candidate))) continue;
    const source = readFileSync(candidate, "utf8");
    for (const [pattern, reason] of forbidden) {
      if (pattern.test(source))
        failures.push(`${relative(root, candidate)}: ${reason}`);
      pattern.lastIndex = 0;
    }
  }
}

for (const directory of scanRoots) visit(join(root, directory));

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log(
  "Web sources contain no forbidden remote assets or dynamic HTML/code APIs.",
);
