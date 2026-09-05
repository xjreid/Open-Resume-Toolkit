// Local M0 qualification. No key export, trust changes, account manipulation,
// automatic installation, GUI-success claims, or network-denial claims.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  designatedRequirement,
  developmentId,
  verifyConfiguration,
  verifyLocalAssetReference,
  verifySignature,
} from "./lib/macos-qualification.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outputRoot = join(root, "target/m0-qualification");
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const json = (path) => JSON.parse(readFileSync(path, "utf8"));

function run(command, args, options = {}) {
  const { rawOutput = false, ...spawnOptions } = options;
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
    env: { ...process.env, CI: "true" },
    ...spawnOptions,
  });
  assert.ifError(result.error);
  assert.equal(
    result.status,
    0,
    `${command} failed: ${result.stderr || "see command output"}`,
  );
  return rawOutput ? result.stdout || "" : (result.stdout || "").trim();
}

function report(name, value) {
  mkdirSync(outputRoot, { recursive: true });
  writeFileSync(join(outputRoot, name), `${JSON.stringify(value, null, 2)}\n`);
  console.log(`Evidence: ${join(outputRoot, name)}`);
}

function configuration() {
  const base = join(root, "apps/desktop/src-tauri");
  const config = json(join(base, "tauri.conf.json"));
  const files = readdirSync(join(base, "capabilities")).sort();
  assert.deepEqual(files, ["main.json", "overlay.json"]);
  const capabilities = files.map((file) =>
    json(join(base, "capabilities", file)),
  );
  verifyConfiguration(config, capabilities);
  return { config, capabilities };
}

function sourceIdentity() {
  // Covers tracked and new implementation inputs, without reading user data or
  // ignored build artifacts. Documentation can be updated after recording tests.
  const paths = run("git", [
    "ls-files",
    "-z",
    "--cached",
    "--others",
    "--exclude-standard",
  ])
    .split("\0")
    .filter(Boolean)
    .filter(
      (path) =>
        !path.endsWith(".md") &&
        !path.startsWith("evidence/") &&
        !path.startsWith("Product Plans/") &&
        !path.startsWith("Implementation Plans/") &&
        !path.startsWith("Aesthetic/"),
    )
    .sort();
  const hash = createHash("sha256");
  for (const path of paths) {
    hash.update(path).update("\0");
    if (!existsSync(join(root, path))) {
      hash.update("deleted\0");
      continue;
    }
    assert.ok(
      lstatSync(join(root, path)).isFile(),
      `non-regular source: ${path}`,
    );
    hash.update(readFileSync(join(root, path))).update("\0");
  }
  return {
    baselineCommit: run("git", ["rev-parse", "HEAD"]),
    dirty: run("git", ["status", "--porcelain"]).length > 0,
    implementationSha256: hash.digest("hex"),
  };
}

function preflight() {
  assert.equal(process.platform, "darwin", "qualification requires macOS");
  assert.equal(
    process.arch,
    "arm64",
    "qualification requires native Apple Silicon Node",
  );
  for (const key of [
    "TAURI_CONFIG",
    "CARGO_TARGET_DIR",
    "APPLE_SIGNING_IDENTITY",
    "APPLE_CERTIFICATE",
    "APPLE_CERTIFICATE_PASSWORD",
    "TAURI_SIGNING_PRIVATE_KEY",
  ]) {
    assert.ok(
      !process.env[key],
      `unset ${key} for reproducible local qualification`,
    );
  }
  run("node", ["tools/bootstrap.mjs"], { stdio: "inherit" });
  run("/usr/bin/xcode-select", ["-p"]);
  configuration();
  const value = {
    scope: "m0-macos-arm64-development",
    date: new Date().toISOString(),
    source: sourceIdentity(),
    macos: run("/usr/bin/sw_vers", []),
    architecture: run("/usr/bin/uname", ["-m"]),
    configuration: "passed-exact-development-policy",
    nativeGui: "pending",
    nativeVaultM1: "not-tested",
  };
  report("preflight.json", value);
  return value;
}

function frontendInventory() {
  const dist = join(root, "apps/desktop/dist");
  const files = [];
  function visit(directory, prefix = "") {
    for (const entry of readdirSync(directory).sort()) {
      const path = join(directory, entry),
        relative = `${prefix}${entry}`;
      assert.ok(!lstatSync(path).isSymbolicLink());
      if (lstatSync(path).isDirectory()) visit(path, `${relative}/`);
      else files.push({ path: relative, sha256: sha256(readFileSync(path)) });
    }
  }
  visit(dist);
  for (const name of ["index.html", "overlay.html"]) {
    const html = readFileSync(join(dist, name), "utf8");
    for (const match of html.matchAll(/\b(?:src|href)=["']([^"']+)["']/g)) {
      verifyLocalAssetReference(match[1]);
      assert.ok(
        existsSync(join(dist, match[1].replace(/^\//, ""))),
        "missing asset",
      );
    }
  }
  return files;
}

function inspectApp(app, expectedCertificate) {
  const bundle = resolve(app);
  assert.ok(bundle.endsWith(".app"));
  assert.ok(!lstatSync(bundle).isSymbolicLink());
  const info = JSON.parse(
    run("/usr/bin/plutil", [
      "-convert",
      "json",
      "-o",
      "-",
      join(bundle, "Contents/Info.plist"),
    ]),
  );
  assert.equal(info.CFBundleIdentifier, developmentId);
  assert.equal(info.CFBundleShortVersionString, "0.0.0-dev");
  assert.match(info.CFBundleExecutable, /^[\w-]+$/);
  const executable = join(bundle, "Contents/MacOS", info.CFBundleExecutable);
  assert.equal(run("/usr/bin/lipo", ["-archs", executable]), "arm64");
  run("/usr/bin/codesign", ["--verify", "--deep", "--strict", bundle]);
  const detailsResult = spawnSync(
    "/usr/bin/codesign",
    ["-d", "--verbose=4", bundle],
    { encoding: "utf8" },
  );
  assert.equal(detailsResult.status, 0);
  const details = detailsResult.stderr;
  const entitlementText = run("/usr/bin/codesign", [
    "-d",
    "--entitlements",
    ":-",
    bundle,
  ]);
  const entitlements = entitlementText
    ? JSON.parse(
        run("/usr/bin/plutil", ["-convert", "json", "-o", "-", "-"], {
          input: entitlementText,
        }),
      )
    : {};
  const certDir = mkdtempSync(join(tmpdir(), "ort-m0-public-cert-"));
  const certPrefix = join(certDir, "certificate-");
  run("/usr/bin/codesign", [
    "-d",
    `--extract-certificates=${certPrefix}`,
    bundle,
  ]);
  const certificateSha256 = sha256(readFileSync(`${certPrefix}0`));
  verifySignature(
    details,
    entitlements,
    certificateSha256,
    expectedCertificate,
  );
  const requirementResult = spawnSync(
    "/usr/bin/codesign",
    ["-d", "-r-", bundle],
    { encoding: "utf8" },
  );
  assert.equal(requirementResult.status, 0);
  return {
    bundle,
    executableSha256: sha256(readFileSync(executable)),
    bundleIdentifier: info.CFBundleIdentifier,
    architecture: "arm64",
    certificateSha256,
    designatedRequirement: designatedRequirement(
      `${requirementResult.stdout}\n${requirementResult.stderr}`,
    ),
    hardenedRuntime: true,
    entitlements,
    signatureVerification: "deep-strict-passed",
  };
}

function build(identity) {
  assert.ok(
    identity && identity !== "-",
    "a local certificate identity is required",
  );
  const initial = preflight();
  const identities = run("/usr/bin/security", [
    "find-identity",
    "-v",
    "-p",
    "codesigning",
  ]);
  const matches = [
    ...identities.matchAll(/\b([A-Fa-f\d]{40}) "([^"\n]+)"/g),
  ].filter((match) => match[2] === identity || match[1] === identity);
  assert.equal(
    matches.length,
    1,
    "exactly one valid local signing identity required",
  );
  const fingerprint = matches[0][1];
  // Read only the public certificate, never the private key.
  const pem = run("/usr/bin/security", [
    "find-certificate",
    "-c",
    matches[0][2],
    "-p",
  ]);
  const der = Buffer.from(
    pem.replace(/-----[^-]+-----/g, "").replace(/\s/g, ""),
    "base64",
  );
  assert.equal(
    createHash("sha1").update(der).digest("hex").toUpperCase(),
    fingerprint,
  );
  const certificateSha256 = sha256(der);
  const override = JSON.stringify({
    bundle: {
      active: true,
      targets: ["app"],
      macOS: {
        signingIdentity: fingerprint,
        hardenedRuntime: true,
      },
    },
  });
  run(
    "pnpm",
    [
      "--filter",
      "@ort/desktop",
      "tauri",
      "build",
      "--config",
      override,
      "--bundles",
      "app",
    ],
    { stdio: "inherit" },
  );
  assert.equal(
    sourceIdentity().implementationSha256,
    initial.source.implementationSha256,
    "source changed during build",
  );
  const assets = frontendInventory();
  const artifact = inspectApp(
    join(root, "target/release/bundle/macos/Open Resume Toolkit Dev.app"),
    certificateSha256,
  );
  report("build.json", {
    ...initial,
    artifact,
    frontendAssets: assets,
    packaging: "release-profile-local-self-signed-app-no-notarization",
    bundlePolicy:
      "source-policy-and-sealed-release-build; not arbitrary-binary source recovery",
    guiHealth: { main: "pending", overlay: "pending" },
    hostedCi: "pending-containing-commit",
  });
}

function verify(app) {
  const buildReport = json(join(outputRoot, "build.json"));
  assert.equal(
    sourceIdentity().implementationSha256,
    buildReport.source.implementationSha256,
    "build evidence is stale; rebuild",
  );
  const artifact = inspectApp(app, buildReport.artifact.certificateSha256);
  assert.equal(
    artifact.executableSha256,
    buildReport.artifact.executableSha256,
    "installed binary differs from tested build",
  );
  assert.equal(
    artifact.designatedRequirement,
    buildReport.artifact.designatedRequirement,
  );
  report("verified-app.json", {
    date: new Date().toISOString(),
    source: buildReport.source,
    artifact,
    guiHealth: "requires-observation-in-both-installed-windows",
    milestoneComplete: false,
  });
}

function cleanCheckout(snapshot = false) {
  preflight();
  if (!snapshot)
    assert.equal(
      run("git", ["status", "--porcelain"]),
      "",
      "commit changes or use clean-snapshot to test a temporary source commit",
    );
  run("node", ["tools/check-secrets.mjs"], { stdio: "inherit" });
  const originalSource = sourceIdentity();
  const commit = run("git", ["rev-parse", "HEAD"]);
  const checkout = join(
    mkdtempSync(join(tmpdir(), "ort-m0-checkout-")),
    "source",
  );
  run(
    "git",
    ["clone", "--no-local", "--no-hardlinks", "--no-checkout", root, checkout],
    { stdio: "inherit" },
  );
  run("git", ["checkout", "--detach", commit], {
    cwd: checkout,
    stdio: "inherit",
  });
  if (snapshot) {
    const patch = run("git", ["diff", "--binary", "HEAD"], { rawOutput: true });
    if (patch)
      run("git", ["apply", "--binary", "-"], {
        cwd: checkout,
        input: patch,
      });
    const added = run("git", [
      "ls-files",
      "--others",
      "--exclude-standard",
      "-z",
    ])
      .split("\0")
      .filter(Boolean);
    for (const path of added) {
      assert.ok(
        lstatSync(join(root, path)).isFile(),
        "snapshot refuses symlinks and special files",
      );
      mkdirSync(dirname(join(checkout, path)), { recursive: true });
      copyFileSync(join(root, path), join(checkout, path));
    }
    run("git", ["add", "--all"], { cwd: checkout });
    run(
      "git",
      [
        "-c",
        "user.name=ORT Local Qualification",
        "-c",
        "user.email=qualification@example.invalid",
        "-c",
        "commit.gpgSign=false",
        "commit",
        "--allow-empty",
        "-m",
        "Temporary M0 qualification source snapshot",
      ],
      { cwd: checkout },
    );
    assert.equal(
      sourceIdentity().implementationSha256,
      originalSource.implementationSha256,
      "source changed during snapshot",
    );
  }
  const testedCommit = run("git", ["rev-parse", "HEAD"], { cwd: checkout });
  assert.equal(run("git", ["status", "--porcelain"], { cwd: checkout }), "");
  for (const recipe of ["bootstrap", "check", "test-platform"]) {
    run("just", [recipe], { cwd: checkout, stdio: "inherit" });
  }
  assert.equal(run("git", ["status", "--porcelain"], { cwd: checkout }), "");
  assert.equal(
    sourceIdentity().implementationSha256,
    originalSource.implementationSha256,
    "source changed during qualification; rerun with the final source",
  );
  report("clean-checkout.json", {
    date: new Date().toISOString(),
    commit,
    testedCommit,
    originalSource,
    temporarySourceSnapshot: snapshot,
    checkout,
    result: "passed-bootstrap-check-platform",
    source: "fresh-local-clone-no-copied-node_modules-or-target",
    note: "system toolchains and package download caches may be shared; checkout retained for inspection",
  });
}

try {
  const [mode, argument, ...extra] = process.argv.slice(2);
  assert.equal(extra.length, 0);
  if (mode === "preflight" && !argument) preflight();
  else if (mode === "build" && argument) build(argument);
  else if (mode === "verify" && argument) verify(argument);
  else if (mode === "clean-checkout" && !argument) cleanCheckout();
  else if (mode === "clean-snapshot" && !argument) cleanCheckout(true);
  else
    throw new Error(
      "Use: preflight | build IDENTITY | verify APP | clean-checkout | clean-snapshot",
    );
} catch (error) {
  console.error(`M0 qualification failed: ${error.message}`);
  process.exitCode = 1;
}
