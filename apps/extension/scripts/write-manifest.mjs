import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const target = process.argv[2];
if (!new Set(["chrome", "edge"]).has(target)) {
  console.error("Expected browser target: chrome or edge");
  process.exit(2);
}

const packageRoot = resolve(import.meta.dirname, "..");
const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const base = readJson(resolve(packageRoot, "manifest/base.json"));
const targetFields = readJson(resolve(packageRoot, `manifest/${target}.json`));
const manifest = { ...base, ...targetFields };

if (
  manifest.host_permissions ||
  manifest.content_scripts ||
  manifest.externally_connectable
) {
  throw new Error("M0 manifests must not expose page or external origins");
}
if (manifest.permissions.length !== 0) {
  throw new Error("M0 manifests must remain permission-free");
}

const output = resolve(packageRoot, `dist/${target}`);
mkdirSync(output, { recursive: true });
writeFileSync(
  resolve(output, "manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
);
console.log(`Generated permission-free ${target} development manifest.`);
