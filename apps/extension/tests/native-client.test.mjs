import test from "node:test";
import assert from "node:assert/strict";
import { createNativeClient } from "../dist/chrome/native-client.js";
function harness(timeout = 50) {
  const ports = [];
  const api = {
    runtime: {
      connectNative: (host) => {
        const listeners = {};
        const port = {
          host,
          sent: [],
          postMessage: (value) => port.sent.push(value),
          disconnect: () => listeners.disconnect?.(),
          onMessage: { addListener: (fn) => (listeners.message = fn) },
          onDisconnect: { addListener: (fn) => (listeners.disconnect = fn) },
          reply: (value) => listeners.message(value),
        };
        ports.push(port);
        return port;
      },
    },
  };
  return { ports, client: createNativeClient(api, timeout) };
}
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
test("one native port serializes requests, correlating responses in order", async () => {
  const h = harness();
  const first = h.client.request({ kind: "bridge.poll" }),
    second = h.client.request({ kind: "capture.event" });
  await tick();
  assert.equal(h.ports.length, 1);
  assert.equal(h.ports[0].sent.length, 1);
  h.ports[0].reply({ one: true });
  assert.deepEqual(await first, { one: true });
  await tick();
  assert.equal(h.ports[0].sent.length, 2);
  h.ports[0].reply({ two: true });
  assert.deepEqual(await second, { two: true });
});
test("disconnect and timeout reject once and content is not retried", async () => {
  const h = harness(5);
  const response = h.client.request({
    kind: "capture.selection",
    payload: { text: "Synthetic" },
  });
  await assert.rejects(response);
  assert.equal(h.ports.length, 1);
  assert.equal(h.ports[0].sent.length, 1);
  const next = h.client.request({ kind: "bridge.poll" });
  await tick();
  h.ports[1].reply({ ok: true });
  assert.deepEqual(await next, { ok: true });
});

test("a timeout invalidates queued captures instead of sending them through a new port", async () => {
  const h = harness(5);
  const first = h.client.request({ kind: "bridge.poll" });
  const capture = h.client.request({
    kind: "capture.selection",
    payload: { text: "Synthetic" },
  });
  await Promise.all([assert.rejects(first), assert.rejects(capture)]);
  assert.equal(h.ports.length, 1);
  assert.deepEqual(h.ports[0].sent, [{ kind: "bridge.poll" }]);
});

test("oversized frames and queue floods are bounded before native delivery", async () => {
  const h = harness(50);
  await assert.rejects(
    h.client.request({ text: "x".repeat(256 * 1024) }),
    /CAPTURE_TOO_LARGE/,
  );
  assert.equal(h.ports.length, 0);
  const requests = Array.from({ length: 8 }, () =>
    h.client.request({ kind: "bridge.poll" }),
  );
  const rejected = requests.map((request) => assert.rejects(request));
  await assert.rejects(
    h.client.request({ kind: "bridge.poll" }),
    /BRIDGE_UNAVAILABLE/,
  );
  await tick();
  h.client.close();
  await Promise.all(rejected);
  assert.equal(h.ports.length, 1);
  assert.equal(h.ports[0].sent.length, 1);
});

test("a stale port or oversized reply cannot satisfy a newer request", async () => {
  const h = harness();
  const old = h.client.request({ kind: "bridge.poll" });
  const rejected = assert.rejects(old);
  await tick();
  h.ports[0].reply({ text: "x".repeat(256 * 1024) });
  await rejected;
  const next = h.client.request({ kind: "bridge.poll" });
  await tick();
  h.ports[0].reply({ stale: true });
  h.ports[1].reply({ ok: true });
  assert.deepEqual(await next, { ok: true });
});

test("closing before the queued request starts prevents native launch", async () => {
  const h = harness();
  const request = h.client.request({
    kind: "capture.selection",
    payload: { text: "Synthetic" },
  });
  h.client.close();
  await assert.rejects(request, /BRIDGE_UNAVAILABLE/);
  assert.equal(h.ports.length, 0);
});
