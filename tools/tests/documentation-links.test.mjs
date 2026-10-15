import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { localLinks, missingLocalLinks } from "../lib/documentation-links.mjs";

test("documentation links support spaces, images, reference links and fragments without executing code examples", () => {
  assert.deepEqual(
    localLinks(
      [
        "[plan](<Product Plans/Plan.md#scope>) ![image](assets/preview.png)",
        '[reference]: docs/guide.md "Guide"',
        "[web](https://example.test/a) [heading](#scope)",
        "`[example](missing-inline.md)`",
        "```md\n[example](missing-code.md)\n```",
      ].join("\n"),
    ),
    ["Product Plans/Plan.md#scope", "assets/preview.png", "docs/guide.md"],
  );
});

test("moved guides resolve relative and repository-root destinations, reject missing and escaped paths", (t) => {
  const root = mkdtempSync(join(tmpdir(), "ort-doc-links-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  mkdirSync(join(root, "Product Plans"));
  writeFileSync(join(root, "README.md"), "# Readme");
  writeFileSync(join(root, "Product Plans", "Plan.md"), "# Plan");
  const file = join(root, "docs", "guide.md");
  const missing = missingLocalLinks(
    root,
    file,
    [
      "[root](/README.md) [relative](../README.md#heading)",
      "[encoded](../Product%20Plans/Plan.md)",
      "[missing](missing.md) [escape](../../outside.md)",
      "[bad](%ZZ.md)",
    ].join("\n"),
  );
  assert.deepEqual(missing, ["missing.md", "../../outside.md", "%ZZ.md"]);
});
