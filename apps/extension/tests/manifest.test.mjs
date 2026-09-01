import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

const root = resolve(import.meta.dirname, "..");
const load = (name) =>
  JSON.parse(readFileSync(resolve(root, `manifest/${name}.json`), "utf8"));

test("M0 base manifest has no browsing or native authority", () => {
  const manifest = load("base");
  assert.deepEqual(manifest.permissions, []);
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
