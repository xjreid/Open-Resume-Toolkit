import assert from "node:assert/strict";
import test from "node:test";
import {
  createBridgeRuntime,
  RECONNECT_ALARM,
} from "../dist/chrome/bridge-runtime.js";

function harness(poll) {
  const listeners = {},
    alarms = [],
    timers = new Map();
  let closed = 0,
    sequence = 0;
  const api = {
    alarms: {
      create: async (name, info) => alarms.push({ name, info }),
      onAlarm: { addListener: (fn) => (listeners.alarm = fn) },
    },
    runtime: {
      onStartup: { addListener: (fn) => (listeners.startup = fn) },
      onInstalled: { addListener: (fn) => (listeners.installed = fn) },
    },
  };
  const runtime = createBridgeRuntime(api, poll, () => closed++, {
    setTimeout: (fn, delay) => {
      timers.set(++sequence, { fn, delay });
      return sequence;
    },
    clearTimeout: (id) => timers.delete(id),
  });
  return { runtime, listeners, alarms, timers, closed: () => closed };
}

test("absent desktop releases the port and uses browser-owned recovery, never a fast retry loop", async () => {
  let polls = 0;
  const h = harness(async () => {
    polls++;
    return false;
  });
  await h.runtime.wake();
  assert.equal(polls, 1);
  assert.equal(h.closed(), 1);
  assert.equal(h.timers.size, 0);
  assert.deepEqual(h.alarms, [
    { name: RECONNECT_ALARM, info: { periodInMinutes: 0.5 } },
  ]);
  h.listeners.alarm({ name: "unrelated-alarm" });
  assert.equal(polls, 1);
  h.listeners.alarm({ name: RECONNECT_ALARM });
  await h.runtime.wake();
  assert.equal(polls, 2);
});

test("live connection uses one heartbeat and concurrent wake events cannot duplicate polls", async () => {
  let finish,
    polls = 0;
  const h = harness(() => {
    polls++;
    return new Promise((resolve) => (finish = resolve));
  });
  const one = h.runtime.wake(),
    two = h.runtime.wake();
  assert.equal(one, two);
  h.listeners.startup();
  h.listeners.installed();
  assert.equal(polls, 1);
  finish(true);
  await one;
  assert.equal(h.timers.size, 1);
  assert.equal([...h.timers.values()][0].delay, 500);
  const next = h.runtime.wake();
  assert.equal(h.timers.size, 0);
  finish(false);
  await next;
  assert.equal(h.closed(), 1);
  assert.equal(h.timers.size, 0);
});

test("controller failure and a fresh worker can recover through the same alarm", async () => {
  const h = harness(async () => {
    throw new Error("failure");
  });
  await h.runtime.wake();
  assert.equal(h.closed(), 1);
  assert.equal(h.timers.size, 0);
  const restarted = harness(async () => true);
  restarted.listeners.alarm({ name: RECONNECT_ALARM });
  await restarted.runtime.wake();
  assert.equal(restarted.timers.size, 1);
});
