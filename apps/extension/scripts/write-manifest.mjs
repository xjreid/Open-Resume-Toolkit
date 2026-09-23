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
if (manifest.permissions.length !== 0 || manifest.action.default_popup) {
  throw new Error(
    "Unsigned development manifests must remain capture-disabled",
  );
}

const output = resolve(packageRoot, `dist/${target}`);
mkdirSync(output, { recursive: true });
writeFileSync(
  resolve(output, "manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
);
writeFileSync(
  resolve(output, "popup.html"),
  readFileSync(resolve(packageRoot, "src/popup.html")),
);
console.log(`Generated capture-disabled ${target} development manifest.`);
