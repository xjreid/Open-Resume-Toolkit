import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const dev = process.argv[2] === "dev-bridge";
const build = resolve(root, dev ? "dist/chrome-dev-bridge" : "dist/chrome");
const manifest = JSON.parse(
  readFileSync(resolve(build, "manifest.json"), "utf8"),
);
if (
  manifest.name !==
    (dev ? "Open Resume Toolkit BETA" : "Open Resume Toolkit") ||
  manifest.permissions.length !== 2
)
  throw new Error("Build the Chrome store package first.");
const files = readdirSync(build, { recursive: true, withFileTypes: true })
  .filter((entry) => entry.isFile())
  .map((entry) => resolve(entry.parentPath, entry.name).slice(build.length + 1))
  .sort();
const artifactRoot = resolve(root, "../../artifacts/extension/chrome");
mkdirSync(artifactRoot, { recursive: true });
const archive = resolve(
  artifactRoot,
  `open-resume-toolkit-chrome-${dev ? "dev-" : ""}${manifest.version}.zip`,
);
// ZIP at the package root (not a wrapping directory), with deterministic dates.
const script = `import pathlib, sys, zipfile
root = pathlib.Path(sys.argv[1])
with zipfile.ZipFile(sys.argv[2], 'w', compression=zipfile.ZIP_DEFLATED) as archive:
    for name in sys.argv[3:]:
        info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o100644 << 16
        archive.writestr(info, (root / name).read_bytes())
`;
const packed = spawnSync("python3", ["-c", script, build, archive, ...files], {
  encoding: "utf8",
});
if (packed.error || packed.status !== 0)
  throw new Error("Could not create Chrome ZIP. Python 3 is required.");
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
writeFileSync(
  `${archive}.json`,
  `${JSON.stringify({ version: manifest.version, archiveSha256: sha256(archive), sourceCommit: revision.status === 0 ? revision.stdout.trim() : null, sourceDirty: changes.status === 0 ? changes.stdout.trim().length > 0 : null, desktopDelivery: dev ? "Opt-in macOS unsigned development bridge; requires local host setup" : "Requires verified signed production desktop/native-host integration", files: Object.fromEntries(files.map((name) => [name, sha256(resolve(build, name))])) }, null, 2)}\n`,
);
console.log(`Chrome package: ${archive}`);
console.log(
  dev
    ? "Development beta package; configure exact extension ID before testing."
    : "Production draft only; signed production transport is not implemented.",
);
