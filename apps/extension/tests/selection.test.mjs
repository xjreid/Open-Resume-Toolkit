import assert from "node:assert/strict";
import test from "node:test";
import { normalizeSelection, sanitizeUrl } from "../dist/chrome/selection.js";

test("selection is normalized and bounded", () => {
  assert.equal(normalizeSelection("  One\r\nTwo  "), "One\nTwo");
  assert.throws(() => normalizeSelection("  "));
  assert.throws(() => normalizeSelection("a".repeat(128 * 1024 + 1)));
});

test("UTF-8 limits count bytes and accept the exact boundary", () => {
  assert.equal(normalizeSelection("a".repeat(128 * 1024)).length, 128 * 1024);
  assert.throws(() => normalizeSelection("😀".repeat(40_000)));
  assert.throws(() => normalizeSelection(null));
  assert.throws(() => normalizeSelection("Job text \ud800"));
  assert.equal(normalizeSelection("Job text 😀"), "Job text 😀");
  assert.throws(() => sanitizeUrl(null));
  assert.throws(() =>
    sanitizeUrl(`https://example.test/?job=${"a".repeat(4096)}`),
  );
});

test("URL cleanup preserves job identifiers but drops common credentials", () => {
  assert.equal(
    sanitizeUrl(
      "https://example.test/job?jobId=42&API_KEY=secret&SESSIONID=secret&refresh_token=secret&password=secret&ref=jobs",
    ),
    "https://example.test/job?jobId=42&ref=jobs",
  );
  assert.throws(() => sanitizeUrl("javascript:alert(1)"));
  assert.throws(() => sanitizeUrl("not a URL"));
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
