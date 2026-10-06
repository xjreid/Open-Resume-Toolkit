import assert from "node:assert/strict";
import { createHash, generateKeyPairSync } from "node:crypto";
import {
  cpSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { inspectChromePackage } from "../scripts/lib/package-policy.mjs";
import {
  chromeIdentity,
  readChromeIdentity,
} from "../../../tools/lib/chrome-extension-identity.mjs";

test("Store packaging rejects leaked files, symlinks, development routing and permission expansion", () => {
  const temporary = mkdtempSync(join(tmpdir(), "ort-package-qa-"));
  const folder = join(temporary, "chrome");
  const source = resolve(import.meta.dirname, "../dist/chrome");
  const reset = () => {
    rmSync(folder, { recursive: true, force: true });
    cpSync(source, folder, { recursive: true });
  };
  try {
    reset();
    assert.equal(
      inspectChromePackage(folder).hostName,
      "com.openresumetoolkit",
    );
    writeFileSync(join(folder, "local-profile.json"), "synthetic private data");
    assert.throws(() => inspectChromePackage(folder), /Unexpected/);
    reset();
    rmSync(join(folder, "NOTICE"));
    symlinkSync(join(source, "NOTICE"), join(folder, "NOTICE"));
    assert.throws(() => inspectChromePackage(folder), /symlink/);
    reset();
    writeFileSync(
      join(folder, "bridge-config.js"),
      'export const NATIVE_HOST = "com.openresumetoolkit.dev";\nexport const BROWSER = "chrome";\n',
    );
    assert.throws(() => inspectChromePackage(folder), /native host/);
    reset();
    const path = join(folder, "manifest.json"),
      manifest = JSON.parse(readFileSync(path, "utf8"));
    manifest.permissions.push("storage");
    writeFileSync(path, JSON.stringify(manifest));
    assert.throws(() => inspectChromePackage(folder), /security boundary/);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});

test("a dashboard identity configures matching unpacked testing without changing upload identity or dev key", () => {
  const temporary = mkdtempSync(join(tmpdir(), "ort-store-identity-qa-"));
  try {
    mkdirSync(join(temporary, "tools/lib"), { recursive: true });
    mkdirSync(join(temporary, "apps/extension/manifest"), { recursive: true });
    for (const name of [
      "configure-chrome-store.mjs",
      "dev-browser-bridge.mjs",
      "lib/chrome-extension-identity.mjs",
    ])
      cpSync(
        resolve(import.meta.dirname, `../../../tools/${name}`),
        join(temporary, "tools", name),
      );
    const configuration = join(
      temporary,
      "apps/extension/manifest/chrome-store-key.json",
    );
    writeFileSync(
      configuration,
      JSON.stringify({ extensionId: null, key: null }),
    );
    const dev = join(temporary, "apps/extension/manifest/chrome-dev-key.json");
    cpSync(
      resolve(import.meta.dirname, "../manifest/chrome-dev-key.json"),
      dev,
    );
    const originalDev = readFileSync(dev, "utf8");
    const { publicKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
    const pem = publicKey.export({ type: "spki", format: "pem" });
    const der = publicKey.export({ type: "spki", format: "der" });
    const id = [...createHash("sha256").update(der).digest().subarray(0, 16)]
      .map((byte) => String.fromCharCode(97 + (byte >> 4), 97 + (byte & 15)))
      .join("");
    const publicFile = join(temporary, "public-key.txt");
    writeFileSync(publicFile, pem);
    const run = (script, ...args) =>
      spawnSync(process.execPath, [join(temporary, "tools", script), ...args], {
        encoding: "utf8",
      });
    const configure = (extensionId) =>
      run(
        "configure-chrome-store.mjs",
        "--public-key-file",
        publicFile,
        "--extension-id",
        extensionId,
      );
    assert.notEqual(configure("a".repeat(32)).status, 0);
    assert.throws(() => readChromeIdentity(configuration), /Upload/);
    assert.equal(configure(id).status, 0);
    assert.deepEqual(readChromeIdentity(configuration), {
      key: der.toString("base64"),
      extensionId: id,
    });
    assert.equal(
      run("dev-browser-bridge.mjs", "id", "--store-test").stdout.trim(),
      id,
    );
    assert.notEqual(
      run(
        "dev-browser-bridge.mjs",
        "install",
        "--store-test",
        "--extension-id",
        "a".repeat(32),
      ).status,
      0,
    );
    assert.equal(readFileSync(dev, "utf8"), originalDev);
    assert.throws(() => chromeIdentity("invalid base64"));
    assert.throws(
      () => chromeIdentity("-----BEGIN " + "PRIVATE KEY-----"),
      /PUBLIC/,
    );
    const { publicKey: weakKey } = generateKeyPairSync("rsa", {
      modulusLength: 1024,
    });
    assert.throws(
      () => chromeIdentity(weakKey.export({ type: "spki", format: "pem" })),
      /2048/,
    );
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});
