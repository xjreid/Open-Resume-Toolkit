import {
  copyFileSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createHash, createPublicKey } from "node:crypto";
import { resolve } from "node:path";

const target = process.argv[2];
const channel = process.argv[3] ?? "dev";
if (!new Set(["chrome", "edge"]).has(target)) {
  console.error("Expected browser target: chrome or edge");
  process.exit(2);
}
if (
  !["dev", "store", "dev-bridge"].includes(channel) ||
  (channel !== "dev" && target !== "chrome")
) {
  throw new Error("Only Chrome supports a store package in this milestone.");
}

const packageRoot = resolve(import.meta.dirname, "..");
const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const base = readJson(resolve(packageRoot, "manifest/base.json"));
const targetFields = readJson(resolve(packageRoot, `manifest/${target}.json`));
const manifest =
  channel !== "dev"
    ? readJson(resolve(packageRoot, "manifest/chrome-store.json"))
    : { ...base, ...targetFields };

if (channel === "dev-bridge") {
  const configuration = readJson(
    resolve(packageRoot, "manifest/chrome-dev-key.json"),
  );
  const { key } = configuration;
  createPublicKey({
    key: Buffer.from(key, "base64"),
    format: "der",
    type: "spki",
  });
  Object.assign(manifest, {
    name: "Open Resume Toolkit BETA",
    version: configuration.version ?? manifest.version,
    description:
      "THIS EXTENSION IS FOR BETA TESTING. Capture page text using the ORT desktop overlay.",
    key,
  });
  const id = [
    ...createHash("sha256")
      .update(Buffer.from(key, "base64"))
      .digest()
      .subarray(0, 16),
  ]
    .map((byte) => String.fromCharCode(97 + (byte >> 4), 97 + (byte & 15)))
    .join("");
  console.log(`Development extension ID: ${id}`);
}

if (
  (channel === "dev" && manifest.host_permissions) ||
  manifest.content_scripts ||
  manifest.externally_connectable
) {
  throw new Error("Manifests must not expose page or external origins");
}
if (
  channel === "dev" &&
  (manifest.permissions.length !== 0 || manifest.action.default_popup)
) {
  throw new Error(
    "Unsigned development manifests must remain capture-disabled",
  );
}
if (
  channel !== "dev" &&
  JSON.stringify(manifest.permissions) !==
    JSON.stringify(["scripting", "nativeMessaging"])
) {
  throw new Error(
    "Chrome package requires exactly the approved capture permissions.",
  );
}

const output = resolve(
  packageRoot,
  `dist/${target}${channel === "dev-bridge" ? "-dev-bridge" : target === "chrome" && channel === "dev" ? "-dev" : ""}`,
);
if (process.argv[4] === "prepare") {
  rmSync(output, { recursive: true, force: true });
  process.exit(0);
}
mkdirSync(output, { recursive: true });
writeFileSync(
  resolve(output, "manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
);
writeFileSync(
  resolve(output, "bridge-config.js"),
  `export const NATIVE_HOST = ${JSON.stringify(channel === "store" ? "com.openresumetoolkit" : "com.openresumetoolkit.dev")};\nexport const BROWSER = ${JSON.stringify(target)};\n`,
);
if (channel !== "dev") {
  mkdirSync(resolve(output, "icons"), { recursive: true });
  for (const size of [16, 32, 48, 128])
    copyFileSync(
      resolve(packageRoot, `../../Aesthetic/Logo/app-${size}.png`),
      resolve(output, `icons/${size}.png`),
    );
  for (const file of [
    "LICENSE",
    "NOTICE",
    "ADDITIONAL_TERMS.md",
    "TRADEMARKS.md",
  ])
    copyFileSync(resolve(packageRoot, `../../${file}`), resolve(output, file));
}
console.log(
  `Generated ${target} ${channel} manifest (${channel !== "dev" ? "capture enabled; matching desktop bridge required" : "capture disabled"}).`,
);
