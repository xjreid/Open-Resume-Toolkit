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
