import assert from "node:assert/strict";
import { createHash, generateKeyPairSync } from "node:crypto";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  copyFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

test("dashboard key synchronization rejects mismatched IDs and increments only on changes", () => {
  const root = mkdtempSync(join(tmpdir(), "ort-identity-qa-"));
  try {
    mkdirSync(join(root, "tools"));
    mkdirSync(join(root, "tools/lib"));
    copyFileSync(
      resolve(
        import.meta.dirname,
        "../../../tools/lib/chrome-extension-identity.mjs",
      ),
      join(root, "tools/lib/chrome-extension-identity.mjs"),
    );
    mkdirSync(join(root, "apps/extension/manifest"), { recursive: true });
    const script = join(root, "tools/dev-browser-bridge.mjs");
    copyFileSync(
      resolve(import.meta.dirname, "../../../tools/dev-browser-bridge.mjs"),
      script,
    );
    const configuration = join(
      root,
      "apps/extension/manifest/chrome-dev-key.json",
    );
    copyFileSync(
      resolve(import.meta.dirname, "../manifest/chrome-dev-key.json"),
      configuration,
    );
    const original = readFileSync(configuration, "utf8");
    const { publicKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
    const der = publicKey.export({ type: "spki", format: "der" });
    const publicFile = join(root, "public-key.txt");
    writeFileSync(
      publicFile,
      publicKey.export({ type: "spki", format: "pem" }),
    );
    const id = [...createHash("sha256").update(der).digest().subarray(0, 16)]
      .map((byte) => String.fromCharCode(97 + (byte >> 4), 97 + (byte & 15)))
      .join("");
    const run = (...args) =>
      spawnSync(process.execPath, [script, ...args], { encoding: "utf8" });
    assert.notEqual(
      run(
        "configure-key",
        "--public-key-file",
        publicFile,
        "--extension-id",
        "a".repeat(32),
      ).status,
      0,
    );
    assert.equal(readFileSync(configuration, "utf8"), original);
    assert.equal(
      run(
        "configure-key",
        "--public-key-file",
        publicFile,
        "--extension-id",
        id,
      ).status,
      0,
    );
    const updated = JSON.parse(readFileSync(configuration, "utf8"));
    const parts = (JSON.parse(original).version ?? "0.1.0")
      .split(".")
      .map(Number);
    parts[parts.length - 1] += 1;
    assert.equal(updated.version, parts.join("."));
    assert.equal(updated.key, der.toString("base64"));
    assert.equal(run("id").stdout.trim(), id);
    assert.equal(
      run(
        "configure-key",
        "--public-key-file",
        publicFile,
        "--extension-id",
        id,
      ).status,
      0,
    );
    assert.deepEqual(JSON.parse(readFileSync(configuration, "utf8")), updated);
    assert.notEqual(run("install", "--extension-id", "a".repeat(32)).status, 0);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
