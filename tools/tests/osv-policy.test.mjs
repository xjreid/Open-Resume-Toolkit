import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const expected = new Set([
  "RUSTSEC-2024-0320",
  "RUSTSEC-2024-0370",
  "RUSTSEC-2024-0411",
  "RUSTSEC-2024-0412",
  "RUSTSEC-2024-0413",
  "RUSTSEC-2024-0415",
  "RUSTSEC-2024-0416",
  "RUSTSEC-2024-0418",
  "RUSTSEC-2024-0419",
  "RUSTSEC-2024-0420",
  "RUSTSEC-2024-0429",
  "RUSTSEC-2024-0436",
  "RUSTSEC-2025-0075",
  "RUSTSEC-2025-0080",
  "RUSTSEC-2025-0081",
  "RUSTSEC-2025-0098",
  "RUSTSEC-2025-0100",
  "RUSTSEC-2025-0141",
  "RUSTSEC-2026-0192",
  "RUSTSEC-2026-0194",
  "RUSTSEC-2026-0195",
  "RUSTSEC-2026-0206",
]);

test("OSV exceptions are exact, explained, expiring, and wired into both lockfile scans", async () => {
  const [policy, workflow] = await Promise.all([
    readFile(new URL("../../osv-scanner.toml", import.meta.url), "utf8"),
    readFile(
      new URL("../../.github/workflows/dependency-scan.yml", import.meta.url),
      "utf8",
    ),
  ]);
  assert.doesNotMatch(policy, /\[\[PackageOverrides\]\]/);
  const entries = policy.split("[[IgnoredVulns]]").slice(1);
  assert.equal(entries.length, expected.size);
  const actual = new Set();
  for (const entry of entries) {
    const id = entry.match(/^\s*id = "(RUSTSEC-\d{4}-\d{4})"/m)?.[1];
    const expiry = entry.match(/^\s*ignoreUntil = (\d{4}-\d{2}-\d{2})/m)?.[1];
    const reason = entry.match(/^\s*reason = "([^"]+)"/m)?.[1];
    assert.ok(
      id && expected.has(id),
      `unexpected or malformed exception ${id}`,
    );
    assert.ok(!actual.has(id), `duplicate exception ${id}`);
    assert.equal(expiry, "2026-12-04", `${id} must keep the review deadline`);
    assert.ok(reason && reason.length >= 40, `${id} needs a concrete reason`);
    actual.add(id);
  }
  assert.deepEqual(actual, expected);
  assert.match(workflow, /^\s*--config=osv-scanner\.toml$/m);
  assert.match(workflow, /^\s*--lockfile=pnpm-lock\.yaml$/m);
  assert.match(workflow, /^\s*--lockfile=Cargo\.lock$/m);
  assert.doesNotMatch(workflow, /fail-on-vuln:\s*false/);
});
