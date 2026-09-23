import assert from "node:assert/strict";
import test from "node:test";
import { normalizeSelection, sanitizeUrl } from "../dist/chrome/selection.js";

test("selection is normalized and bounded", () => {
  assert.equal(normalizeSelection("  One\r\nTwo  "), "One\nTwo");
  assert.throws(() => normalizeSelection("  "));
  assert.throws(() => normalizeSelection("a".repeat(128 * 1024 + 1)));
});

test("URL credentials, fragments and trackers are removed", () => {
  assert.equal(
    sanitizeUrl(
      "https://user:pass@example.test/job?x=1&utm_source=test&token=secret#part",
    ),
    "https://example.test/job?x=1",
  );
  assert.throws(() => sanitizeUrl("file:///private/test"));
});
