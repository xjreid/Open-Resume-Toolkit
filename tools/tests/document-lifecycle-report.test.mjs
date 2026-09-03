import assert from "node:assert/strict";
import test from "node:test";
import { interpretLifecycle } from "../lib/document-lifecycle-report.mjs";

function fixture() {
  return [0, 1, 2, 3, 3, 4, 5, 2, 2].map((reason, index) => ({
    schemaVersion: 1,
    case: index,
    reason,
    readyObserved: true,
    childReaped: true,
    killSent: (index >= 1 && index <= 4) || index >= 7,
    stdoutEof: true,
    stderrEof: true,
    accepted: index === 0,
    exitCode:
      (index >= 1 && index <= 4) || index >= 7 ? -1 : index === 5 ? 65 : 0,
    signal: (index >= 1 && index <= 4) || index >= 7 ? 9 : 0,
    elapsedMs: [2, 7, 8].includes(index) ? 1100 : 100,
    stdoutBytes: [27, 13, 13, 4097, 13, 27, 21, 27, 27][index],
    stderrBytes: index === 4 ? 4097 : 0,
  }));
}

test("native lifecycle subset never enables import or proves full containment", () => {
  const report = interpretLifecycle(fixture());
  assert.equal(report.cancellationAndTimeoutKillReapPassed, true);
  assert.equal(report.importEnabled, false);
  assert.equal(report.fullContainmentProven, false);
});

test("valid bytes, connection closure or sent kill without reaping are insufficient", () => {
  for (const key of [
    "readyObserved",
    "childReaped",
    "stdoutEof",
    "stderrEof",
    "killSent",
  ]) {
    const value = fixture();
    value[1][key] = false;
    assert.throws(() => interpretLifecycle(value));
  }
  for (const signal of [0, 14, 15]) {
    const value = fixture();
    value[1].signal = signal;
    assert.throws(() => interpretLifecycle(value));
  }
});

test("cancel, timeout, flood, nonzero and malformed output cannot become success", () => {
  for (let index = 1; index < 9; index++) {
    const value = fixture();
    value[index].accepted = true;
    assert.throws(() => interpretLifecycle(value));
  }
});

test("byte limits, case identity and measured deadlines are checked", () => {
  for (const change of [
    (v) => {
      v[0].stdoutBytes++;
    },
    (v) => {
      v[3].stdoutBytes--;
    },
    (v) => {
      v[4].stderrBytes--;
    },
    (v) => {
      v[0].stderrBytes++;
    },
    (v) => {
      v[2].elapsedMs = 999;
    },
    (v) => {
      v[1].elapsedMs = 4000;
    },
    (v) => {
      v[0].case = 1;
    },
    (v) => {
      v[5].exitCode = 0;
    },
  ]) {
    const value = fixture();
    change(value);
    assert.throws(() => interpretLifecycle(value));
  }
});

test("missing, extra, invalid and replayed case records are rejected", () => {
  for (const value of [
    null,
    {},
    [],
    fixture().slice(1),
    [...fixture(), fixture()[0]],
  ])
    assert.throws(() => interpretLifecycle(value));
  for (const key of Object.keys(fixture()[0])) {
    const missing = fixture();
    delete missing[0][key];
    assert.throws(() => interpretLifecycle(missing));
    const invalid = fixture();
    invalid[0][key] = "1";
    assert.throws(() => interpretLifecycle(invalid));
  }
  const extra = fixture();
  extra[0].unexpected = true;
  assert.throws(() => interpretLifecycle(extra));
  const repeated = fixture();
  repeated[1] = repeated[0];
  assert.throws(() => interpretLifecycle(repeated));
  const inherited = fixture();
  Object.setPrototypeOf(inherited[0], { childReaped: true });
  delete inherited[0].childReaped;
  inherited[0].unexpected = true;
  assert.throws(() => interpretLifecycle(inherited));
});
