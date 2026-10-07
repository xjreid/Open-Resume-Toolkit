import { createHash } from "node:crypto";
import {
  mkdirSync,
  readFileSync,
  renameSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { inspectChromePackage } from "./lib/package-policy.mjs";

const root = resolve(import.meta.dirname, "..");
const dev = process.argv[2] === "dev-bridge";
if (process.argv.length > 3 || (process.argv[2] !== undefined && !dev))
  throw new Error("Use package-chrome.mjs [dev-bridge].");
const build = resolve(root, dev ? "dist/chrome-dev-bridge" : "dist/chrome");
const { manifest, files, hostName } = inspectChromePackage(build, dev);
const artifactRoot = resolve(root, "../../artifacts/extension/chrome");
mkdirSync(artifactRoot, { recursive: true });
const archive = resolve(
  artifactRoot,
  `open-resume-toolkit-chrome-${dev ? "dev-" : ""}${manifest.version}.zip`,
);
const temporary = `${archive}.${process.pid}.tmp`;
// ZIP at the package root (not a wrapping directory), with deterministic dates.
const script = `import pathlib, sys, zipfile
root = pathlib.Path(sys.argv[1])
with zipfile.ZipFile(sys.argv[2], 'w', compression=zipfile.ZIP_DEFLATED) as archive:
    for name in sys.argv[3:]:
        info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o100644 << 16
        archive.writestr(info, (root / name).read_bytes())
with zipfile.ZipFile(sys.argv[2]) as archive:
    assert archive.testzip() is None
    assert archive.namelist() == sys.argv[3:]
`;
const packed = spawnSync(
  "python3",
  ["-c", script, build, temporary, ...files],
  {
    encoding: "utf8",
  },
);
if (packed.error || packed.status !== 0)
  throw new Error("Could not create Chrome ZIP. Python 3 is required.");
renameSync(temporary, archive);
const sha256 = (path) =>
  createHash("sha256").update(readFileSync(path)).digest("hex");
const revision = spawnSync("git", ["rev-parse", "HEAD"], {
  cwd: root,
  encoding: "utf8",
});
const changes = spawnSync("git", ["status", "--porcelain"], {
  cwd: root,
  encoding: "utf8",
});
const inputs = [
  "manifest/chrome-store.json",
  "tsconfig.json",
  ...readdirSync(resolve(root, "src"))
    .filter((name) => name.endsWith(".ts"))
    .map((name) => `src/${name}`),
].sort();
const sourceFiles = Object.fromEntries(
  inputs.map((name) => [`apps/extension/${name}`, sha256(resolve(root, name))]),
);
const sourceSha256 = createHash("sha256")
  .update(JSON.stringify(sourceFiles))
  .digest("hex");
writeFileSync(
  `${archive}.json`,
  `${JSON.stringify({ schemaVersion: 1, browser: "chrome", version: manifest.version, nativeHost: hostName, protocolVersion: 1, archiveSha256: sha256(archive), sourceSha256, sourceFiles, sourceCommit: revision.status === 0 ? revision.stdout.trim() : null, sourceDirty: changes.status === 0 ? changes.stdout.trim().length > 0 : null, desktopDelivery: dev ? "Explicit macOS current-user development bridge" : "Production host; explicit development registration can test the same extension contract. Signed production bridge qualification remains M8.", files: Object.fromEntries(files.map((name) => [name, sha256(resolve(build, name))])) }, null, 2)}\n`,
);
console.log(`Chrome package: ${archive}`);
console.log(
  dev
    ? "Development beta package; configure exact extension ID before testing."
    : "Chrome Store upload ZIP ready. Upload as a draft to obtain its final ID. Signed native production bridge qualification remains M8.",
);
