import assert from "node:assert/strict";
import test from "node:test";
import { createCaptureController } from "../dist/chrome/capture.js";
const url = "https://example.test/job?jobId=42&token=secret&utm_source=qa#part";
function harness(options = {}) {
  const id = crypto.randomUUID(),
    calls = [],
    scripts = [],
    removed = [];
  let commands = [
    {
      kind: "capture.start",
      sessionId: id,
      target: options.target ?? "job",
      expiresAt: Date.now() + 60_000,
    },
  ];
  const listeners = {};
  const event = (name) => ({
    addListener: (fn) => {
      listeners[name] = fn;
    },
  });
  const api = {
    runtime: { id: "ort-test" },
    tabs: {
      query: async () => [
        { id: options.activeId ?? 7, url: options.url ?? url, windowId: 2 },
      ],
      get: async () => ({
        id: 7,
        url: options.changed ? "https://example.test/new" : url,
        windowId: 2,
      }),
      sendMessage: async (id, message) => {
        removed.push({ id, message });
        return { ok: true };
      },
      onActivated: event("activated"),
      onRemoved: event("removed"),
      onUpdated: event("updated"),
    },
    windows: { onFocusChanged: event("focus") },
    scripting: {
      executeScript: async (value) => {
        scripts.push(value);
        if (options.scriptError) throw new Error("private system details");
        if (!value.args) {
          if (value.target.documentIds && options.reloaded)
            throw new Error("new document");
          return [{ frameId: 0, documentId: "document-one", result: url }];
        }
        if (options.duringInjection) options.duringInjection();
        return [
          { frameId: 0, documentId: "document-one", result: { ready: true } },
        ];
      },
    },
  };
  const native = async (message) => {
    calls.push(message);
    if (options.native) return options.native(message);
    if (message.kind === "bridge.poll")
      return { ok: true, protocolVersion: 1, value: { ready: true, commands } };
    if (message.kind === "capture.selection")
      return (
        options.ack ?? {
          ok: true,
          protocolVersion: 1,
          value: { requestId: message.requestId },
        }
      );
    return { ok: true, protocolVersion: 1 };
  };
  const controller = createCaptureController(api, native);
  const sender = {
    id: "ort-test",
    tab: { id: 7 },
    frameId: 0,
    documentId: "document-one",
    url,
  };
  const page = (phase, extra = {}) => ({
    kind: "capture.page",
    sessionId: id,
    phase,
    ...extra,
  });
  const select = () => controller.handle(page("selecting"), sender);
  const complete = (extra = {}) =>
    controller.handle(
      page("completed", {
        text: "Synthetic job description\r\nRe\u0301sume\u0301 experience required.",
        url,
        title: "Synthetic job",
        ...extra,
      }),
      sender,
    );
  return {
    controller,
    id,
    calls,
    scripts,
    removed,
    listeners,
    sender,
    page,
    select,
    complete,
    setCommands: (value) => {
      commands = value;
    },
  };
}
test("desktop command arms the active document; page content is not sent until completion", async () => {
  const h = harness();
  await h.controller.poll();
  assert.equal(h.scripts.length, 2);
  assert.deepEqual(h.scripts[0].target, { tabId: 7, frameIds: [0] });
  assert.deepEqual(h.scripts[1].target, {
    tabId: 7,
    documentIds: ["document-one"],
  });
  assert.equal(
    h.calls.some((call) => call.kind === "capture.selection"),
    false,
  );
  assert.equal((await h.complete()).alive, false);
  await h.select();
  await h.complete();
  const capture = h.calls.find((call) => call.kind === "capture.selection");
  assert.equal(capture.requestId, h.id);
  assert.equal(capture.payload.url, "https://example.test/job?jobId=42");
  assert.equal(
    capture.payload.text,
    "Synthetic job description\nRésumé experience required.",
  );
  assert.equal(capture.payload.target, "job");
  await h.controller.poll();
  assert.equal(h.scripts.filter((script) => script.args).length, 1);
});
test("a first corner during injection already has exact document authority", async () => {
  let selected;
  const h = harness({
    duringInjection: () => {
      selected = h.select();
    },
  });
  await h.controller.poll();
  assert.equal((await selected).alive, true);
  await h.complete();
  assert.equal(
    h.calls.filter((call) => call.kind === "capture.selection").length,
    1,
  );
});
test("question destination is controlled by the desktop, not page messages", async () => {
  const h = harness({ target: "question" });
  await h.controller.poll();
  await h.select();
  await h.complete({ target: "job" });
  assert.equal(
    h.calls.find((call) => call.kind === "capture.selection").payload.target,
    "question",
  );
});
test("wrong extension, document, frame, tab and URL cannot forward content or progress", async () => {
  const h = harness();
  await h.controller.poll();
  for (const sender of [
    { ...h.sender, id: "other" },
    { ...h.sender, documentId: "new" },
    { ...h.sender, frameId: 1 },
    { ...h.sender, tab: { id: 8 } },
    { ...h.sender, url: "https://other.test/" },
    {},
  ])
    assert.equal(
      (await h.controller.handle(h.page("selecting"), sender)).alive,
      false,
    );
  assert.equal(
    h.calls.some((call) => call.phase === "selecting"),
    false,
  );
});
test("cancel clears the page mode and rejects late completion", async () => {
  const h = harness();
  await h.controller.poll();
  await h.select();
  h.setCommands([{ kind: "capture.cancel", sessionId: h.id }]);
  await h.controller.poll();
  await h.complete();
  assert.equal(
    h.calls.some((call) => call.kind === "capture.selection"),
    false,
  );
  assert.equal(h.removed.length, 1);
});
test("navigation, same-URL reload, tab changes, close and window changes fail closed", async () => {
  for (const event of ["updated", "activated", "removed", "focus"]) {
    const h = harness();
    await h.controller.poll();
    await h.select();
    if (event === "updated") h.listeners.updated(7, { status: "loading" });
    if (event === "activated") h.listeners.activated({ tabId: 8, windowId: 2 });
    if (event === "removed") h.listeners.removed(7);
    if (event === "focus") h.listeners.focus(3);
    await h.complete();
    assert.equal(
      h.calls.some((call) => call.kind === "capture.selection"),
      false,
    );
  }
  const h = harness({ reloaded: true });
  await h.controller.poll();
  await h.select();
  await h.complete();
  assert.equal(
    h.calls.some((call) => call.kind === "capture.selection"),
    false,
  );
});
test("oversized, empty, unsafe URL and malformed Unicode content cannot be sent", async () => {
  for (const extra of [
    { text: "" },
    { text: "x".repeat(128 * 1024 + 1) },
    { text: "invalid\ud800" },
    { url: "file:///private/job" },
    { title: "x".repeat(501) },
    { title: "Invalid\ud800" },
  ]) {
    const h = harness();
    await h.controller.poll();
    await h.select();
    await h.complete(extra);
    assert.equal(
      h.calls.some((call) => call.kind === "capture.selection"),
      false,
    );
  }
});

test("incompatible progress replies revoke selection before any content delivery", async () => {
  const h = harness({
    native: async (message) =>
      message.kind === "bridge.poll"
        ? {
            ok: true,
            protocolVersion: 1,
            value: {
              ready: true,
              commands: [
                {
                  kind: "capture.start",
                  sessionId: h.id,
                  target: "job",
                  expiresAt: Date.now() + 60_000,
                },
              ],
            },
          }
        : { ok: true, protocolVersion: 2 },
  });
  await h.controller.poll();
  await h.select();
  await h.complete();
  assert.equal(
    h.calls.some((call) => call.kind === "capture.selection"),
    false,
  );
  assert.equal(h.removed.length, 1);
});
test("missing permission and browser-restricted pages produce a safe failure", async () => {
  for (const options of [{ scriptError: true }, { url: "chrome://settings" }]) {
    const h = harness(options);
    await h.controller.poll();
    assert.equal(
      h.calls.some((call) => call.kind === "capture.selection"),
      false,
    );
    assert.ok(
      h.calls.some(
        (call) => call.phase === "failed" && call.code === "PAGE_UNAVAILABLE",
      ),
    );
  }
});
test("uncorrelated acknowledgements and transport failure never resend a capture", async () => {
  for (const ack of [
    { ok: true, protocolVersion: 1, value: { requestId: "wrong" } },
    null,
    { ok: false, protocolVersion: 1, error: { code: "CAPTURE_EXPIRED" } },
  ]) {
    const h = harness({ ack: ack === null ? { ok: "unknown" } : ack });
    await h.controller.poll();
    await h.select();
    await h.complete();
    await h.controller.poll();
    assert.equal(
      h.calls.filter((call) => call.kind === "capture.selection").length,
      1,
    );
    assert.ok(
      h.calls.some(
        (call) =>
          call.phase === "failed" && call.code === "DELIVERY_UNCONFIRMED",
      ),
    );
  }
});
test("idle heartbeat is content-free and unavailable transport cannot arm a page", async () => {
  const h = harness({
    native: async () => ({
      ok: false,
      protocolVersion: 1,
      error: { code: "BRIDGE_UNAVAILABLE" },
    }),
  });
  await h.controller.poll();
  assert.equal(h.scripts.length, 0);
  assert.deepEqual(Object.keys(h.calls[0]).sort(), [
    "clientId",
    "kind",
    "protocolVersion",
  ]);
});
