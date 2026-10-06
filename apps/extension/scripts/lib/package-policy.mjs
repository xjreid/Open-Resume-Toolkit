import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const modules = [
  "bridge-config.js",
  "bridge-runtime.js",
  "capture.js",
  "chrome-api.js",
  "native-client.js",
  "page-capture.js",
  "selection.js",
  "service-worker.js",
];
const expectedFiles = [
  ...modules,
  "manifest.json",
  "LICENSE",
  "NOTICE",
  "ADDITIONAL_TERMS.md",
  "TRADEMARKS.md",
  ...[16, 32, 48, 128].map((size) => `icons/${size}.png`),
].sort();

export function inspectChromePackage(directory, development = false) {
  if (
    !lstatSync(directory).isDirectory() ||
    lstatSync(directory).isSymbolicLink()
  )
    throw new Error("Package root must be a real directory.");
  const files = [];
  function walk(path, prefix = "") {
    for (const name of readdirSync(path).sort()) {
      const full = join(path, name),
        relative = `${prefix}${name}`;
      const metadata = lstatSync(full);
      if (metadata.isSymbolicLink())
        throw new Error(`Package symlink: ${relative}`);
      if (metadata.isDirectory()) walk(full, `${relative}/`);
      else if (metadata.isFile()) files.push(relative);
      else throw new Error(`Unsupported package entry: ${relative}`);
    }
  }
  walk(directory);
  if (JSON.stringify(files.sort()) !== JSON.stringify(expectedFiles))
    throw new Error(
      "Unexpected or missing package files; rebuild before packaging.",
    );
  const manifest = JSON.parse(
    readFileSync(join(directory, "manifest.json"), "utf8"),
  );
  if (
    manifest.manifest_version !== 3 ||
    manifest.minimum_chrome_version !== "120" ||
    manifest.name !==
      (development ? "Open Resume Toolkit BETA" : "Open Resume Toolkit") ||
    manifest.background?.service_worker !== "service-worker.js" ||
    manifest.background?.type !== "module" ||
    JSON.stringify(manifest.permissions) !==
      JSON.stringify(["scripting", "nativeMessaging", "alarms"]) ||
    JSON.stringify(manifest.host_permissions) !==
      JSON.stringify(["http://*/*", "https://*/*"]) ||
    manifest.content_scripts ||
    manifest.externally_connectable ||
    manifest.web_accessible_resources ||
    manifest.action ||
    manifest.commands ||
    manifest.optional_permissions ||
    (!development && manifest.key) ||
    manifest.content_security_policy?.extension_pages !==
      "script-src 'self'; object-src 'none'; connect-src 'none'; base-uri 'none'"
  )
    throw new Error(
      "Manifest does not match the Chrome package security boundary.",
    );
  const parts = manifest.version?.split(".") ?? [];
  if (
    parts.length < 1 ||
    parts.length > 4 ||
    parts.some(
      (part) => !/^(0|[1-9]\d*)$/.test(part) || Number(part) > 65535,
    ) ||
    parts.every((part) => Number(part) === 0)
  )
    throw new Error("Invalid Chrome package version.");
  const hostName = development
    ? "com.openresumetoolkit.dev"
    : "com.openresumetoolkit";
  const configuration = readFileSync(
    join(directory, "bridge-config.js"),
    "utf8",
  );
  if (
    configuration !==
    `export const NATIVE_HOST = ${JSON.stringify(hostName)};\nexport const BROWSER = "chrome";\n`
  )
    throw new Error("Incorrect native host or browser configuration.");
  for (const name of modules) {
    const source = readFileSync(join(directory, name), "utf8");
    if (
      /chrome\.storage|localStorage|sessionStorage|console\.(log|error)|\bfetch\s*\(|\beval\s*\(|new\s+Function\s*\(/.test(
        source,
      )
    )
      throw new Error(
        `Prohibited storage, logging or remote/dynamic code in ${name}.`,
      );
    for (const [, dependency] of source.matchAll(
      /(?:from\s+|import\s*)["']([^"']+)["']/g,
    ))
      if (
        !dependency.startsWith("./") ||
        !modules.includes(dependency.slice(2))
      )
        throw new Error(`Non-local or missing module in ${name}.`);
  }
  for (const size of [16, 32, 48, 128]) {
    if (manifest.icons?.[size] !== `icons/${size}.png`)
      throw new Error("Incorrect icon path.");
    const bytes = readFileSync(join(directory, `icons/${size}.png`));
    if (
      bytes.length < 24 ||
      bytes.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a" ||
      bytes.readUInt32BE(16) !== size ||
      bytes.readUInt32BE(20) !== size
    )
      throw new Error(`Invalid ${size}px PNG icon.`);
  }
  return { files, manifest, hostName };
}
