import assert from "node:assert/strict";
import test from "node:test";
import { interpretProbe } from "../lib/document-sandbox-report.mjs";

function fixture() {
  return {
    schemaVersion: 2,
    control: {
      descriptorRead: true,
      descriptorReadOnly: true,
      sandboxEntitlement: false,
      siblingRead: 0,
      siblingWrite: 0,
      symlinkRead: 0,
      loopbackConnect: 0,
      childCreation: 0,
      childFork: 0,
    },
    sandboxed: {
      descriptorRead: true,
      descriptorReadOnly: true,
      sandboxEntitlement: true,
      siblingRead: 1,
      siblingWrite: 1,
      symlinkRead: 1,
      loopbackConnect: 1,
      childCreation: 0,
      childFork: 0,
    },
    hardened: {
      descriptorRead: true,
      descriptorReadOnly: true,
      sandboxEntitlement: true,
      siblingRead: 1,
      siblingWrite: 1,
      symlinkRead: 1,
      loopbackConnect: 1,
      childCreation: 1,
      childFork: 1,
    },
    hardLimits: {
      nprocSoft: 0,
      nprocHard: 0,
      nofileSoft: 64,
      nofileHard: 64,
      coreSoft: 0,
      coreHard: 0,
      raiseDenied: true,
      descriptorCeilingDenied: true,
      descriptorRecovery: true,
    },
    parentUnaffected: true,
    cooperativeDisconnectObserved: true,
  };
}

test("passing every measured denial still cannot enable import or claim full containment", () => {
  const report = interpretProbe(fixture());
  assert.equal(report.filesystemIsolationPassed, true);
  assert.equal(report.baselineChildCreationDenied, false);
  assert.equal(report.directChildCreationDenied, true);
  assert.equal(report.descriptorCeilingEnforced, true);
  assert.equal(report.hardLimitRaiseDenied, true);
  assert.equal(report.parentUnaffected, true);
  assert.equal(report.fullContainmentProven, false);
  assert.equal(report.importEnabled, false);
  assert.ok(report.untested.length > 0);
});

test("observed child/filesystem/network authority remains a visible limitation", () => {
  for (const [key, result] of [
    ["childCreation", "directChildCreationDenied"],
    ["childFork", "directChildCreationDenied"],
    ["siblingRead", "filesystemIsolationPassed"],
    ["loopbackConnect", "loopbackConnectDenied"],
  ]) {
    const value = fixture();
    value.hardened[key] = 0;
    assert.equal(interpretProbe(value)[result], false);
    assert.equal(interpretProbe(value).fullContainmentProven, false);
  }
});

test("hardened results cannot hide a filesystem or loopback baseline regression", () => {
  for (const [key, result] of [
    ["siblingRead", "filesystemIsolationPassed"],
    ["siblingWrite", "filesystemIsolationPassed"],
    ["symlinkRead", "filesystemIsolationPassed"],
    ["loopbackConnect", "loopbackConnectDenied"],
  ]) {
    const value = fixture();
    value.sandboxed[key] = 0;
    assert.equal(interpretProbe(value)[result], false);
  }
});

test("missing-target and other OS errors are inconclusive, not access denials", () => {
  for (const side of ["control", "sandboxed", "hardened"]) {
    for (const key of [
      "siblingRead",
      "siblingWrite",
      "symlinkRead",
      "loopbackConnect",
      "childCreation",
      "childFork",
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
      value.schemaVersion = 1;
    },
    (value) => {
      value.hardened.sandboxEntitlement = false;
    },
    (value) => {
      value.parentUnaffected = false;
    },
  ]) {
    const value = fixture();
    change(value);
    assert.throws(() => interpretProbe(value));
  }
});

test("same-helper spawn and fork controls are required before hard-limit evidence", () => {
  for (const key of ["childCreation", "childFork"]) {
    const value = fixture();
    value.sandboxed[key] = 1;
    assert.throws(() => interpretProbe(value), /positive control/);
  }
});

test("soft-only or changed hard limits cannot be accepted as the required policy", () => {
  for (const key of [
    "nprocSoft",
    "nprocHard",
    "nofileSoft",
    "nofileHard",
    "coreSoft",
    "coreHard",
  ]) {
    for (const invalid of ["0", null, -1, true, 1024]) {
      const value = fixture();
      value.hardLimits[key] = invalid;
      assert.throws(() => interpretProbe(value), /hard-limit/);
    }
  }
});

test("limit escalation, missing descriptor denial and failed recovery stay visible", () => {
  for (const [key, conclusion] of [
    ["raiseDenied", "hardLimitRaiseDenied"],
    ["descriptorCeilingDenied", "descriptorCeilingEnforced"],
    ["descriptorRecovery", "descriptorCeilingEnforced"],
  ]) {
    const value = fixture();
    value.hardLimits[key] = false;
    const report = interpretProbe(value);
    assert.equal(report[conclusion], false);
    assert.equal(report.importEnabled, false);
    value.hardLimits[key] = 1;
    assert.throws(() => interpretProbe(value), /hard-limit/);
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
  for (const field of ["control", "sandboxed", "hardened", "hardLimits"]) {
    for (const key of Object.keys(fixture()[field])) {
      const value = fixture();
      delete value[field][key];
      assert.throws(() => interpretProbe(value), /shape/);
    }
    const extra = fixture();
    extra[field].unexpectedAuthority = true;
    assert.throws(() => interpretProbe(extra), /shape/);
  }
});
