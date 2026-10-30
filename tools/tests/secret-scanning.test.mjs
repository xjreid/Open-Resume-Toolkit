import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { test } from "node:test";
import {
  forbiddenSecretPath,
  inspectScanDirectory,
  scanSecrets,
} from "../lib/secret-scanning.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
const binary =
  process.env.GITLEAKS_BIN ||
  join(
    root,
    "target/security-tools",
    process.platform === "win32" ? "gitleaks.exe" : "gitleaks",
  );
const config = join(root, ".gitleaks.toml");
const token = () =>
  ["ghp", randomBytes(27).toString("hex").slice(0, 36)].join("_");
function temporary(t) {
  const path = mkdtempSync(join(tmpdir(), "ort-secret-test-"));
  t.after(() => rmSync(path, { recursive: true, force: true }));
  return path;
}
function put(directory, name, value) {
  const path = join(directory, name);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, value);
  return path;
}
function git(directory, ...args) {
  return execFileSync(
    "git",
    [
      "-c",
      "core.hooksPath=/dev/null",
      "-c",
      "user.name=Secret scan fixture",
      "-c",
      "user.email=fixture@example.invalid",
      ...args,
    ],
    { cwd: directory, stdio: ["ignore", "pipe", "pipe"] },
  );
}
function repository(t) {
  const path = temporary(t);
  for (const name of [
    "tools/check-secrets.mjs",
    "tools/lib/secret-scanning.mjs",
    ".gitleaks.toml",
    "target/security-tools/gitleaks-rules.toml",
  ])
    put(path, name, readFileSync(join(root, name)));
  put(path, ".gitignore", "target/\n");
  git(path, "init", "--initial-branch=main");
  return path;
}
function run(directory, ...args) {
  return spawnSync(
    process.execPath,
    [join(directory, "tools/check-secrets.mjs"), ...args],
    {
      cwd: directory,
      env: { ...process.env, GITLEAKS_BIN: binary },
      encoding: "utf8",
    },
  );
}

test("credential filenames are rejected at any depth without banning public keys or example templates", () => {
  for (const path of [
    ".env.production.local",
    "nested/.env.test",
    "nested/id_ed25519",
    "nested/signing.p12",
    ".aws/config",
    "nested/auth.json",
    "profile.sqlite3",
    ".ort-test-data/profile.json",
  ])
    assert.equal(forbiddenSecretPath(path), true, path);
  for (const path of [
    ".env.example",
    "docs/example.md",
    "packages/catalog/direct-v1.pub",
    "apps/extension/manifest/chrome-dev-key.json",
  ])
    assert.equal(forbiddenSecretPath(path), false, path);
});

test("provider tokens are rejected and never copied to failure messages, even with an inline bypass", (t) => {
  const directory = temporary(t),
    report = temporary(t),
    value = token();
  put(
    directory,
    "client.js",
    `export const credential = "${value}"; // gitleaks:allow\n`,
  );
  assert.throws(
    () => scanSecrets(binary, config, directory, report),
    (error) => {
      assert.match(error.message, /Potential secrets/);
      assert.equal(error.message.includes(value), false);
      return true;
    },
  );
});

test("reviewed Chrome public key passes but the same file cannot hide a different secret", (t) => {
  const directory = temporary(t),
    report = temporary(t);
  const name = "apps/extension/manifest/chrome-dev-key.json";
  put(directory, name, readFileSync(join(root, name)));
  scanSecrets(binary, config, directory, report);
  put(directory, name, JSON.stringify({ key: token() }));
  assert.throws(
    () => scanSecrets(binary, config, directory, report),
    /Potential secrets/,
  );
});

test("archive members are scanned", (t) => {
  const directory = temporary(t),
    report = temporary(t);
  const fixture = put(report, "credential.txt", token());
  execFileSync(process.platform === "win32" ? "python" : "python3", [
    "-c",
    "import sys,zipfile; z=zipfile.ZipFile(sys.argv[1],'w',compression=zipfile.ZIP_DEFLATED); z.write(sys.argv[2],'client.js'); z.close()",
    join(directory, "package.zip"),
    fixture,
  ]);
  assert.throws(
    () => scanSecrets(binary, config, directory, report),
    /Potential secrets/,
  );
});

test("SVG, lockfile, and scanner-config names cannot bypass credential detection", (t) => {
  for (const name of ["image.svg", "pnpm-lock.yaml", ".gitleaks.toml"]) {
    const directory = temporary(t);
    put(directory, name, token());
    assert.throws(
      () => scanSecrets(binary, config, directory, temporary(t)),
      /Potential secrets/,
    );
  }
});

test("a weak hardcoded client password is rejected without relying on entropy", (t) => {
  const directory = temporary(t),
    report = temporary(t);
  put(directory, "apps/desktop/src/client.ts", 'const password = "hunter2";');
  assert.throws(
    () => scanSecrets(binary, config, directory, report),
    /ort-client-credential-literal/,
  );
});

test("missing scanners fail closed", (t) => {
  const directory = temporary(t);
  put(directory, "safe.js", "export const safe = true;");
  assert.throws(
    () =>
      scanSecrets(join(directory, "missing"), config, directory, temporary(t)),
    /Gitleaks 8.30.1 is required/,
  );
});

test("artifact traversal rejects links instead of reading an external credential", (t) => {
  const directory = temporary(t),
    outside = temporary(t);
  const path = put(outside, "private.txt", token());
  if (process.platform === "win32")
    symlinkSync(outside, join(directory, "linked"), "junction");
  else symlinkSync(path, join(directory, "linked.js"));
  assert.throws(() => inspectScanDirectory(directory), /symlinks/);
});

test("staged scan reads the index rather than a cleaned working copy", (t) => {
  const directory = repository(t),
    value = token();
  put(directory, "client.js", `export const credential = "${value}";\n`);
  git(directory, "add", ".");
  put(directory, "client.js", "export const safe = true;\n");
  const result = run(directory, "--staged");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Potential secrets/);
  assert.equal(result.stderr.includes(value), false);
  assert.equal(run(directory, "--source").status, 0);
});

test("full-history scan finds a secret deleted from the current tree", (t) => {
  const directory = repository(t),
    value = token();
  put(directory, "client.js", `export const credential = "${value}";\n`);
  git(directory, "add", ".");
  git(directory, "commit", "-m", "Synthetic credential fixture");
  put(directory, "client.js", "export const safe = true;\n");
  git(directory, "add", ".");
  git(directory, "commit", "-m", "Remove fixture");
  const result = run(directory, "--history");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Potential secrets/);
  assert.equal(result.stderr.includes(value), false);
});

test("shallow history and missing build directories cannot report a pass", (t) => {
  const directory = repository(t);
  put(directory, "safe.js", "export const safe = true;\n");
  git(directory, "add", ".");
  git(directory, "commit", "-m", "Synthetic fixture");
  const head = git(directory, "rev-parse", "HEAD");
  put(directory, ".git/shallow", head);
  const history = run(directory, "--history");
  assert.equal(history.status, 1);
  assert.match(history.stderr, /complete clone/);
  const artifacts = run(directory, "--artifacts", "missing-dist");
  assert.equal(artifacts.status, 1);
  assert.match(artifacts.stderr, /Missing or unsafe/);
});

test("renderer build excludes environment canaries and unimported public files", async (t) => {
  const directory = temporary(t),
    value = randomBytes(24).toString("hex");
  const vitePath = join(
    root,
    "apps/desktop/node_modules/vite/dist/node/index.js",
  );
  assert.ok(existsSync(vitePath), "Install locked dependencies first");
  const { build, loadConfigFromFile } = await import(
    pathToFileURL(vitePath).href
  );
  const { config: desktop } = await loadConfigFromFile(
    { command: "build", mode: "production" },
    join(root, "apps/desktop/vite.config.ts"),
  );
  const previous = process.env.VITE_ORT_AUDIT_CANARY;
  process.env.VITE_ORT_AUDIT_CANARY = value;
  t.after(() => {
    if (previous === undefined) delete process.env.VITE_ORT_AUDIT_CANARY;
    else process.env.VITE_ORT_AUDIT_CANARY = previous;
  });
  put(directory, ".env.production", `VITE_DOTENV_CANARY=${value}\n`);
  put(directory, "public/unreviewed.txt", value);
  const input = put(
    directory,
    "index.html",
    '<script type="module" src="/main.js"></script>',
  );
  put(
    directory,
    "main.js",
    "document.body.textContent = JSON.stringify(import.meta.env);",
  );
  const result = await build({
    ...desktop,
    configFile: false,
    root: directory,
    logLevel: "silent",
    build: { write: false, rollupOptions: { input } },
  });
  const output = (Array.isArray(result) ? result : [result]).flatMap(
    (bundle) => bundle.output,
  );
  assert.ok(output.length);
  assert.equal(JSON.stringify(output).includes(value), false);
  assert.deepEqual(desktop.envPrefix, []);
  assert.equal(desktop.envDir, false);
  assert.equal(desktop.publicDir, false);
});
