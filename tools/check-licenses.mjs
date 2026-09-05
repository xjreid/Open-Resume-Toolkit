import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  licenseExceptionAllowed,
  licenseExpressionAllowed,
  platformFamilyLicense,
  platformPackageLicense,
  pnpmPackageKeys,
  splitPackageKey,
} from "./lib/license-policy.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const policy = JSON.parse(
  readFileSync(join(root, "config/dependency-license-policy.json"), "utf8"),
);
const allowedLicenses = new Set(policy.allowedLicenses);
const allowedExceptions = new Set(policy.allowedLicenseExceptions);
const exceptions = new Map(
  policy.packageExceptions.map((entry) => [
    `${entry.ecosystem}:${entry.package}`,
    entry,
  ]),
);
const usedExceptions = new Set();
const failures = [];
const inventory = [];
const javascriptOnly = process.argv.includes("--javascript-only");
const rustOnly = process.argv.includes("--rust-only");
const today = new Date().toISOString().slice(0, 10);

if (javascriptOnly && rustOnly) {
  throw new Error("--javascript-only and --rust-only are mutually exclusive");
}
if (exceptions.size !== policy.packageExceptions.length) {
  failures.push(
    "dependency license policy contains duplicate package exceptions",
  );
}

function verify(ecosystem, name, version, license, source) {
  const packageKey = `${name}@${version}`;
  let allowed = false;
  let parseFailure = null;
  try {
    allowed = licenseExpressionAllowed(
      license,
      allowedLicenses,
      allowedExceptions,
    );
  } catch (error) {
    parseFailure = error.message;
  }
  const exceptionKey = `${ecosystem}:${packageKey}`;
  const exception = exceptions.get(exceptionKey);
  if (!allowed && licenseExceptionAllowed(exception, license, today)) {
    allowed = true;
    usedExceptions.add(exceptionKey);
  }
  if (!allowed) {
    failures.push(
      `${ecosystem} ${packageKey}: ${parseFailure ?? `license ${JSON.stringify(license)} is not allowed`}`,
    );
  }
  inventory.push({ ecosystem, license, name, source, version });
}

function packageManifest(path) {
  const value = JSON.parse(readFileSync(path, "utf8"));
  if (!value.name || !value.version || typeof value.license !== "string") {
    throw new Error(`${path}: name, version, and string license are required`);
  }
  return value;
}

function workspaceManifests() {
  const paths = [join(root, "package.json")];
  for (const parent of ["apps", "packages"]) {
    for (const entry of readdirSync(join(root, parent), {
      withFileTypes: true,
    })) {
      const path = join(root, parent, entry.name, "package.json");
      if (entry.isDirectory() && existsSync(path)) paths.push(path);
    }
  }
  return paths.map(packageManifest);
}

function installedPnpmPackages() {
  const packages = new Map();
  const virtualStore = join(root, "node_modules/.pnpm");
  if (!existsSync(virtualStore)) {
    throw new Error(
      "node_modules is missing; run pnpm install --frozen-lockfile",
    );
  }
  for (const virtualEntry of readdirSync(virtualStore, {
    withFileTypes: true,
  })) {
    if (!virtualEntry.isDirectory()) continue;
    const modules = join(virtualStore, virtualEntry.name, "node_modules");
    if (!existsSync(modules)) continue;
    for (const entry of readdirSync(modules, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const base = join(modules, entry.name);
      const candidates = entry.name.startsWith("@")
        ? readdirSync(base, { withFileTypes: true })
            .filter((child) => child.isDirectory())
            .map((child) => join(base, child.name, "package.json"))
        : [join(base, "package.json")];
      for (const path of candidates) {
        if (!existsSync(path)) continue;
        const value = packageManifest(path);
        const key = `${value.name}@${value.version}`;
        const previous = packages.get(key);
        if (previous && previous.license !== value.license) {
          throw new Error(`${key}: installed copies disagree on license`);
        }
        packages.set(key, value);
      }
    }
  }
  return packages;
}

function verifyJavascript() {
  for (const value of workspaceManifests()) {
    verify(
      "javascript-workspace",
      value.name,
      value.version,
      value.license,
      "workspace",
    );
  }
  const installed = installedPnpmPackages();
  const lockfile = readFileSync(join(root, "pnpm-lock.yaml"), "utf8");
  const lockKeys = pnpmPackageKeys(lockfile);
  for (const record of policy.javascriptPlatformPackages ?? []) {
    if (!lockKeys.includes(record.package))
      failures.push(`unused platform-package policy: ${record.package}`);
  }
  if (new Set(lockKeys).size !== lockKeys.length) {
    failures.push("javascript: pnpm lockfile contains duplicate package keys");
  }
  for (const family of policy.javascriptPlatformFamilies) {
    const representatives = [...installed.entries()].filter(([key]) => {
      const value = splitPackageKey(key);
      return (
        value.name.startsWith(family.prefix) && value.version === family.version
      );
    });
    if (representatives.length === 0) {
      failures.push(
        `javascript: no installed representative verifies ${family.prefix}*@${family.version}`,
      );
    }
    for (const [key, value] of representatives) {
      if (value.license !== family.license) {
        failures.push(
          `javascript ${key}: declared ${JSON.stringify(value.license)} but its platform-family policy declares ${JSON.stringify(family.license)}`,
        );
      }
    }
  }
  for (const key of lockKeys) {
    const { name, version } = splitPackageKey(key);
    const installedPackage = installed.get(key);
    const reviewedLicense = platformPackageLicense(
      key,
      policy.javascriptPlatformPackages ?? [],
      lockfile,
      process.platform,
      installedPackage,
    );
    const license =
      installedPackage?.license ??
      reviewedLicense ??
      platformFamilyLicense(key, policy.javascriptPlatformFamilies);
    if (!license) {
      failures.push(
        `javascript ${key}: package is not installed on this platform and has no exact family policy`,
      );
      inventory.push({
        ecosystem: "javascript",
        license: null,
        name,
        source: "unresolved-platform-package",
        version,
      });
      continue;
    }
    verify(
      "javascript",
      name,
      version,
      license,
      installedPackage
        ? "installed-lock-package"
        : reviewedLicense
          ? "reviewed-exact-platform-package"
          : "reviewed-platform-family",
    );
  }
}

function verifyRust() {
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--locked", "--format-version", "1"], {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 100 * 1024 * 1024,
      stdio: ["ignore", "pipe", "inherit"],
    }),
  );
  for (const value of metadata.packages) {
    if (!value.license) {
      failures.push(
        `rust ${value.name}@${value.version}: SPDX license is missing${
          value.license_file ? ` (license-file: ${value.license_file})` : ""
        }`,
      );
      inventory.push({
        ecosystem: "rust",
        license: null,
        name: value.name,
        source: value.source ?? "workspace",
        version: value.version,
      });
      continue;
    }
    verify(
      "rust",
      value.name,
      value.version,
      value.license,
      value.source ?? "workspace",
    );
  }
}

if (!rustOnly) verifyJavascript();
if (!javascriptOnly) verifyRust();

for (const key of exceptions.keys()) {
  if (!usedExceptions.has(key))
    failures.push(`${key}: unused license exception`);
}

inventory.sort((left, right) =>
  `${left.ecosystem}:${left.name}@${left.version}`.localeCompare(
    `${right.ecosystem}:${right.name}@${right.version}`,
  ),
);
const output = join(root, "target/licenses/dependency-inventory.json");
mkdirSync(dirname(output), { recursive: true });
writeFileSync(
  output,
  `${JSON.stringify(
    {
      schemaVersion: 1,
      policy: "config/dependency-license-policy.json",
      packages: inventory,
    },
    null,
    2,
  )}\n`,
);

if (failures.length > 0) {
  console.error("Dependency license policy failed:\n");
  for (const failure of failures) console.error(`- ${failure}`);
  console.error(`\nPartial inventory: ${output}`);
  process.exit(1);
}

const rustCount = inventory.filter(
  (entry) => entry.ecosystem === "rust",
).length;
const javascriptCount = inventory.length - rustCount;
console.log(
  `Dependency licenses passed: ${rustCount} Rust and ${javascriptCount} JavaScript packages.`,
);
console.log(`Inventory: ${output}`);
