import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  designatedRequirement,
  verifyConfiguration,
  verifyLocalAssetReference,
  verifySignature,
} from "../lib/macos-qualification.mjs";

const base = new URL("../../apps/desktop/src-tauri/", import.meta.url);
const read = (path) => JSON.parse(readFileSync(new URL(path, base), "utf8"));
const config = read("tauri.conf.json");
const capabilities = [
  read("capabilities/main.json"),
  read("capabilities/overlay.json"),
];

test("designated requirement comparison ignores executable location and stream order", () => {
  const requirement =
    'designated => identifier "com.openresumetoolkit.dev" and certificate root = H"abcd"';
  assert.equal(
    designatedRequirement(`${requirement}\nExecutable=/build/app`),
    requirement,
  );
  assert.equal(
    designatedRequirement(`Executable=/Applications/app\n${requirement}\n`),
    requirement,
  );
  assert.throws(() => designatedRequirement("Executable=/app"));
  assert.throws(() => designatedRequirement(`${requirement}\n${requirement}`));
});

test("M0 policy rejects remote windows, CSP weakening and broader capabilities", () => {
  verifyConfiguration(config, capabilities);
  for (const mutate of [
    (value) => {
      value.identifier = "com.openresumetoolkit.preview";
    },
    (value) => {
      value.app.windows[0].url = "https://example.invalid";
    },
    (value) => {
      value.app.security.csp += "; script-src *";
    },
    (value) => {
      value.app.security.capabilities.push("shell");
    },
    (value) => {
      value.app.withGlobalTauri = true;
    },
    (value) => {
      value.bundle.externalBin = ["unexpected"];
    },
  ]) {
    const changed = structuredClone(config);
    mutate(changed);
    assert.throws(() => verifyConfiguration(changed, capabilities));
  }
  for (const mutate of [
    (value) => {
      value[0].permissions.push("shell:allow-execute");
    },
    (value) => {
      value[0].windows = ["*"];
    },
    (value) => {
      value[0].remote = { urls: ["https://example.invalid"] };
    },
  ]) {
    const changed = structuredClone(capabilities);
    mutate(changed);
    assert.throws(() => verifyConfiguration(config, changed));
  }
});

test("production entrypoints accept local assets and reject remote or escaping references", () => {
  for (const path of ["/assets/app.js", "assets/app.css"])
    verifyLocalAssetReference(path);
  for (const path of [
    "https://example.invalid/a",
    "//example.invalid/a",
    "data:text/javascript,x",
    "../a",
    "/%2e%2e/a",
    "a\\b",
  ])
    assert.throws(() => verifyLocalAssetReference(path));
});

test("artifact gate requires pinned certificate, hardened runtime and no added entitlements", () => {
  const details =
    "Identifier=com.openresumetoolkit.dev\nCodeDirectory flags=0x10000(runtime)";
  verifySignature(details, {}, "a".repeat(64), "a".repeat(64));
  assert.throws(() =>
    verifySignature(details, {}, "b".repeat(64), "a".repeat(64)),
  );
  assert.throws(() =>
    verifySignature(
      details,
      { "com.apple.security.get-task-allow": true },
      "a",
      "a",
    ),
  );
  assert.throws(() =>
    verifySignature(details + "\nSignature=adhoc", {}, "a", "a"),
  );
  assert.throws(() =>
    verifySignature(details.replace("(runtime)", ""), {}, "a", "a"),
  );
});
