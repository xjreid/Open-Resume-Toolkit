import { readdirSync, readFileSync, statSync } from "node:fs";
import { basename, extname, join, relative } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const ignoredDirectories = new Set([".git", "node_modules", "target", "dist"]);
const ignoredExtensions = new Set([".png", ".jpg", ".jpeg", ".gif", ".pdf"]);
const forbiddenNames = new Set([".env", ".env.local", "id_rsa", "id_ed25519"]);
const secretMarkers = [
  `-----BEGIN ${"PRIVATE KEY"}-----`,
  `-----BEGIN ${"RSA PRIVATE KEY"}-----`,
  `-----BEGIN ${"OPENSSH PRIVATE KEY"}-----`,
];
const failures = [];

function visit(path) {
  for (const entry of readdirSync(path)) {
    const candidate = join(path, entry);
    if (statSync(candidate).isDirectory()) {
      if (!ignoredDirectories.has(entry)) visit(candidate);
      continue;
    }
    const relativePath = relative(root, candidate);
    if (forbiddenNames.has(basename(candidate)))
      failures.push(`${relativePath}: forbidden secret file`);
    if (ignoredExtensions.has(extname(candidate).toLowerCase())) continue;
    const contents = readFileSync(candidate, "utf8");
    if (secretMarkers.some((marker) => contents.includes(marker))) {
      failures.push(`${relativePath}: private-key marker`);
    }
  }
}

visit(root);
if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log(
  "Repository contains no forbidden secret files or private-key markers.",
);
