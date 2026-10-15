import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { missingLocalLinks } from "./lib/documentation-links.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
function markdownFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (
      entry.isDirectory() &&
      ["node_modules", "target", "dist", ".git"].includes(entry.name)
    )
      return [];
    return entry.isDirectory()
      ? markdownFiles(path)
      : entry.name.endsWith(".md")
        ? [path]
        : [];
  });
}
const files = [
  ...readdirSync(root)
    .filter((name) => name.endsWith(".md"))
    .map((name) => join(root, name)),
  ...[
    ".github",
    "Product Plans",
    "Implementation Plans",
    "docs",
    "apps",
    "crates",
    "packages",
    "packaging",
    "fixtures",
    "templates",
  ].flatMap((name) => markdownFiles(join(root, name))),
];
const missing = files.flatMap((file) =>
  missingLocalLinks(root, file, readFileSync(file, "utf8")).map(
    (target) => `${relative(root, file)}: ${target}`,
  ),
);
if (missing.length) {
  console.error(
    "Broken repository documentation links:\n" + missing.join("\n"),
  );
  process.exit(1);
}
console.log(
  `Repository links passed in ${files.length} entry, plan, component, and documentation files. External links and heading anchors are not verified.`,
);
