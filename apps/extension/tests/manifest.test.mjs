import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

const root = resolve(import.meta.dirname, "..");
const load = (name) =>
  JSON.parse(readFileSync(resolve(root, `manifest/${name}.json`), "utf8"));

test("unsigned development manifest has no capture authority", () => {
  const manifest = load("base");
  assert.deepEqual(manifest.permissions, []);
  assert.equal(manifest.action.default_popup, undefined);
  assert.equal(manifest.host_permissions, undefined);
  assert.equal(manifest.content_scripts, undefined);
  assert.equal(manifest.externally_connectable, undefined);
});

test("browser overlays cannot add permissions", () => {
  for (const target of ["chrome", "edge"]) {
    const overlay = load(target);
    assert.equal(overlay.permissions, undefined);
    assert.equal(overlay.host_permissions, undefined);
  }
});

test("Chrome store manifest grants only deliberate capture and native delivery", () => {
  const manifest = load("chrome-store");
  assert.equal(manifest.manifest_version, 3);
  assert.equal(manifest.name, "Open Resume Toolkit");
  assert.deepEqual(manifest.permissions, ["scripting", "nativeMessaging"]);
  assert.deepEqual(manifest.host_permissions, ["http://*/*", "https://*/*"]);
  assert.equal(manifest.optional_host_permissions, undefined);
  assert.equal(manifest.content_scripts, undefined);
  assert.equal(manifest.externally_connectable, undefined);
  assert.equal(manifest.action, undefined);
  assert.equal(manifest.background.type, "module");
  assert.equal(manifest.commands, undefined);
  assert.match(
    manifest.content_security_policy.extension_pages,
    /connect-src 'none'/,
  );
});

test("built store package contains its declared assets and local modules", () => {
  const directory = resolve(root, "dist/chrome");
  const manifest = JSON.parse(
    readFileSync(resolve(directory, "manifest.json"), "utf8"),
  );
  assert.deepEqual(manifest, load("chrome-store"));
  for (const asset of [
    manifest.background.service_worker,
    ...Object.values(manifest.icons),
    "LICENSE",
    "NOTICE",
    "ADDITIONAL_TERMS.md",
    "TRADEMARKS.md",
  ]) {
    assert.ok(readFileSync(resolve(directory, asset)).length > 0);
  }
  for (const name of [
    "service-worker.js",
    "capture.js",
    "selection.js",
    "native-client.js",
    "page-capture.js",
    "bridge-config.js",
  ]) {
    const source = readFileSync(resolve(directory, name), "utf8");
    for (const [, dependency] of source.matchAll(/from "(\.\/[^"\n]+)"/g))
      assert.ok(readFileSync(resolve(directory, dependency)).length > 0);
    assert.doesNotMatch(
      source,
      /chrome\.storage|localStorage|sessionStorage|console\.(log|error)|fetch\(/,
    );
  }
  assert.match(
    readFileSync(resolve(directory, "bridge-config.js"), "utf8"),
    /"com\.openresumetoolkit"/,
  );
});

test("development bridge build has a stable public ID and isolated host", () => {
  const directory = resolve(root, "dist/chrome-dev-bridge");
  const manifest = JSON.parse(
    readFileSync(resolve(directory, "manifest.json"), "utf8"),
  );
  assert.equal(manifest.name, "Open Resume Toolkit BETA");
  assert.match(manifest.description, /THIS EXTENSION IS FOR BETA TESTING/);
  assert.equal(manifest.key, load("chrome-dev-key").key);
  assert.deepEqual(manifest.permissions, load("chrome-store").permissions);
  assert.deepEqual(manifest.host_permissions, ["http://*/*", "https://*/*"]);
  assert.match(
    readFileSync(resolve(directory, "bridge-config.js"), "utf8"),
    /"com\.openresumetoolkit\.dev"/,
  );
  assert.equal(load("chrome-store").key, undefined);
});
