import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  licenseExceptionAllowed,
  licenseExpressionAllowed,
  platformFamilyLicense,
  pnpmPackageKeys,
  splitPackageKey,
} from "../lib/license-policy.mjs";

const allowed = new Set(["Apache-2.0", "GPL-3.0-only", "MIT", "Unicode-3.0"]);
const exceptions = new Set(["LLVM-exception"]);

test("SPDX policy honors OR, AND, WITH, parentheses, and legacy slash choices", () => {
  assert(licenseExpressionAllowed("MIT OR AGPL-3.0-only", allowed, exceptions));
  assert(licenseExpressionAllowed("MIT/Apache-2.0", allowed, exceptions));
  assert(
    licenseExpressionAllowed(
      "Apache-2.0 WITH LLVM-exception OR MIT",
      allowed,
      exceptions,
    ),
  );
  assert(
    licenseExpressionAllowed(
      "(MIT OR Apache-2.0) AND Unicode-3.0",
      allowed,
      exceptions,
    ),
  );
  assert(
    !licenseExpressionAllowed("MIT AND AGPL-3.0-only", allowed, exceptions),
  );
  assert(!licenseExpressionAllowed("AGPL-3.0-only", allowed, exceptions));
  assert(
    !licenseExpressionAllowed(
      "MIT WITH unknown-exception",
      allowed,
      exceptions,
    ),
  );
  assert.throws(() => licenseExpressionAllowed("MIT +", allowed, exceptions));
});

test("pnpm package parsing includes only top-level locked package keys", () => {
  const lock = `lockfileVersion: '9.0'\npackages:\n\n  '@scope/a@1.2.3':\n    resolution: {integrity: fixed}\n    peerDependencies:\n      nested: 1\n\n  plain@2.0.0:\n    engines: {node: '>=24'}\n\nsnapshots:\n\n  '@scope/a@1.2.3': {}\n`;
  assert.deepEqual(pnpmPackageKeys(lock), ["@scope/a@1.2.3", "plain@2.0.0"]);
  assert.deepEqual(splitPackageKey("@scope/a@1.2.3"), {
    name: "@scope/a",
    version: "1.2.3",
  });
});

test("uninstalled platform package policy is family-and-version exact", () => {
  const families = [
    { prefix: "@vendor/tool-", version: "1.2.3", license: "MIT" },
  ];
  assert.equal(
    platformFamilyLicense("@vendor/tool-win32@1.2.3", families),
    "MIT",
  );
  assert.equal(
    platformFamilyLicense("@vendor/tool-win32@1.2.4", families),
    null,
  );
  assert.equal(
    platformFamilyLicense("@other/tool-win32@1.2.3", families),
    null,
  );
});

test("package exceptions are exact, explained, and unexpired", () => {
  const base = {
    license: "LicenseRef-reviewed",
    reason:
      "A concrete review reason that is deliberately longer than forty characters.",
    expiresOn: "2026-12-04",
  };
  assert(licenseExceptionAllowed(base, "LicenseRef-reviewed", "2026-09-04"));
  assert(!licenseExceptionAllowed(base, "LicenseRef-other", "2026-09-04"));
  assert(!licenseExceptionAllowed(base, "LicenseRef-reviewed", "2026-12-05"));
  assert(
    !licenseExceptionAllowed(
      { ...base, reason: "too short" },
      "LicenseRef-reviewed",
      "2026-09-04",
    ),
  );
});

test("canonical local and CI gates execute license and contract drift checks", async () => {
  const [packageJson, justfile, workflow, policy] = await Promise.all([
    readFile(new URL("../../package.json", import.meta.url), "utf8"),
    readFile(new URL("../../justfile", import.meta.url), "utf8"),
    readFile(
      new URL("../../.github/workflows/ci.yml", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../../config/dependency-license-policy.json", import.meta.url),
      "utf8",
    ),
  ]);
  assert.match(
    packageJson,
    /"check:licenses": "node tools\/check-licenses\.mjs"/,
  );
  assert.match(
    justfile,
    /^verify-contracts:\n\s+cargo run --locked -p ort-contract-generator\n\s+git diff --exit-code -- packages\/contracts\/generated/m,
  );
  assert.match(justfile, /^check:\n\s+pnpm check\n\s+just verify-contracts/m);
  assert.match(workflow, /name: Verify dependency licenses/);
  assert.match(workflow, /run: pnpm check:licenses/);
  const parsedPolicy = JSON.parse(policy);
  assert.equal(parsedPolicy.schemaVersion, 1);
  assert.deepEqual(parsedPolicy.packageExceptions, []);
  assert(
    !parsedPolicy.allowedLicenses.some((license) => license.includes("AGPL")),
  );
});
