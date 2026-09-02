import assert from "node:assert/strict";
import test from "node:test";
import { interpretProbe } from "../lib/document-sandbox-report.mjs";

function fixture() {
  return {
    schemaVersion: 1,
    control: {
      descriptorRead: true,
      descriptorReadOnly: true,
      sandboxEntitlement: false,
      siblingRead: 0,
      siblingWrite: 0,
      symlinkRead: 0,
      loopbackConnect: 0,
      childCreation: 0,
    },
    sandboxed: {
      descriptorRead: true,
      descriptorReadOnly: true,
      sandboxEntitlement: true,
      siblingRead: 1,
      siblingWrite: 1,
      symlinkRead: 1,
      loopbackConnect: 1,
      childCreation: 1,
    },
    cooperativeDisconnectObserved: true,
  };
}

test("passing every measured denial still cannot enable import or claim full containment", () => {
  const report = interpretProbe(fixture());
  assert.equal(report.filesystemIsolationPassed, true);
  assert.equal(report.childCreationDenied, true);
  assert.equal(report.fullContainmentProven, false);
  assert.equal(report.importEnabled, false);
  assert.ok(report.untested.length > 0);
});

test("observed child/filesystem/network authority remains a visible limitation", () => {
  for (const [key, result] of [
    ["childCreation", "childCreationDenied"],
    ["siblingRead", "filesystemIsolationPassed"],
    ["loopbackConnect", "loopbackConnectDenied"],
  ]) {
    const value = fixture();
    value.sandboxed[key] = 0;
    assert.equal(interpretProbe(value)[result], false);
    assert.equal(interpretProbe(value).fullContainmentProven, false);
  }
});

test("missing-target and other OS errors are inconclusive, not access denials", () => {
  for (const side of ["control", "sandboxed"]) {
    for (const key of [
      "siblingRead",
      "siblingWrite",
      "symlinkRead",
      "loopbackConnect",
      "childCreation",
    ]) {
      const value = fixture();
      value[side][key] = 2;
      assert.throws(() => interpretProbe(value), /inconclusive/);
    }
  }
});

test("failed positive controls, entitlement claims and lifecycle observation reject evidence", () => {
  for (const change of [
    (value) => {
      value.control.siblingRead = 1;
    },
    (value) => {
      value.control.sandboxEntitlement = true;
    },
    (value) => {
      value.sandboxed.sandboxEntitlement = false;
    },
    (value) => {
      value.control.descriptorRead = false;
    },
    (value) => {
      value.sandboxed.descriptorReadOnly = false;
    },
    (value) => {
      value.cooperativeDisconnectObserved = false;
    },
    (value) => {
      value.schemaVersion = 2;
    },
  ]) {
    const value = fixture();
    change(value);
    assert.throws(() => interpretProbe(value));
  }
});

test("malformed, extra, missing, or incorrectly typed measurements are rejected", () => {
  for (const value of [
    null,
    [],
    {},
    { ...fixture(), fullContainmentProven: true },
  ]) {
    assert.throws(() => interpretProbe(value));
  }
  for (const corrupt of [true, "1", -1, 3, null, 0.5]) {
    const value = fixture();
    value.sandboxed.childCreation = corrupt;
    assert.throws(() => interpretProbe(value));
  }
  const missing = fixture();
  delete missing.sandboxed.siblingRead;
  assert.throws(() => interpretProbe(missing));
});
