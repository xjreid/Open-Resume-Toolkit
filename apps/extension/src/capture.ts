import type { CaptureTarget, ChromeApi, Sender } from "./chrome-api.js";
import { BROWSER } from "./bridge-config.js";
import { normalizeSelection, sanitizeUrl } from "./selection.js";
import { startPageCapture } from "./page-capture.js";

type Active = {
  id: string;
  target: CaptureTarget;
  expiresAt: number;
  tabId: number;
  windowId?: number;
  url: string;
  documentId: string;
  phase: "waiting" | "selecting" | "sending";
};
function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
function uuid(value: unknown): value is string {
  return (
    typeof value === "string" &&
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(
      value,
    )
  );
}
const safeCodes = new Set([
  "PAGE_UNAVAILABLE",
  "PAGE_CHANGED",
  "EMPTY_SELECTION",
  "CAPTURE_TOO_LARGE",
  "CAPTURE_EXPIRED",
  "CAPTURE_INVALID",
  "BRIDGE_UNAVAILABLE",
  "DELIVERY_UNCONFIRMED",
]);
export function createCaptureController(
  api: ChromeApi,
  native: (message: unknown) => Promise<unknown>,
) {
  const clientId = crypto.randomUUID();
  let active: Active | null = null;
  let starting = false,
    polling = false;
  const finished = new Set<string>();
  function remember(id: string) {
    finished.add(id);
    if (finished.size > 64) finished.delete(finished.values().next().value!);
  }
  async function event(id: string, phase: string, code?: string) {
    return native({
      protocolVersion: 1,
      kind: "capture.event",
      clientId,
      sessionId: id,
      phase,
      ...(code ? { code: safeCodes.has(code) ? code : "CAPTURE_INVALID" } : {}),
    });
  }
  async function remove(session: Active) {
    await api.tabs
      .sendMessage(
        session.tabId,
        { kind: "capture.cancel", sessionId: session.id },
        { documentId: session.documentId },
      )
      .catch(() => {});
  }
  async function fail(code: string) {
    const session = active;
    active = null;
    if (!session) return;
    remember(session.id);
    await remove(session);
    await event(session.id, "failed", code).catch(() => {});
  }
  function source(sender: Sender, session: Active) {
    return (
      sender.id === api.runtime.id &&
      sender.tab?.id === session.tabId &&
      sender.frameId === 0 &&
      sender.documentId === session.documentId &&
      sender.url === session.url
    );
  }
  async function start(command: Record<string, unknown>) {
    if (
      !uuid(command.sessionId) ||
      !["job", "question"].includes(String(command.target)) ||
      typeof command.expiresAt !== "number" ||
      !Number.isSafeInteger(command.expiresAt) ||
      command.expiresAt <= Date.now() ||
      command.expiresAt > Date.now() + 120_000
    )
      return;
    const id = command.sessionId;
    if (active?.id === id || finished.has(id) || starting) return;
    starting = true;
    try {
      if (active) {
        const old = active;
        active = null;
        remember(old.id);
        await remove(old);
      }
      const [tab] = await api.tabs.query({
        active: true,
        lastFocusedWindow: true,
      });
      if (typeof tab?.id !== "number" || !tab.url || !/^https?:/.test(tab.url))
        throw new Error("PAGE_UNAVAILABLE");
      const documents = await api.scripting.executeScript({
        target: { tabId: tab.id, frameIds: [0] },
        world: "ISOLATED",
        func: () => location.href,
      });
      const frame = documents.find((frame) => frame.frameId === 0);
      if (!frame?.documentId || frame.result !== tab.url)
        throw new Error("PAGE_UNAVAILABLE");
      // Establish sender identity before exposing the box, so a fast first click
      // cannot race the injection response. Inject only into this exact document.
      const session: Active = {
        id,
        target: command.target as CaptureTarget,
        expiresAt: command.expiresAt,
        tabId: tab.id,
        windowId: tab.windowId,
        url: tab.url,
        documentId: frame.documentId,
        phase: "waiting",
      };
      active = session;
      const injected = await api.scripting.executeScript({
        target: { tabId: tab.id, documentIds: [frame.documentId] },
        world: "ISOLATED",
        func: startPageCapture,
        args: [id, command.expiresAt],
      });
      if (active !== session) {
        await remove(session);
        return;
      }
      if (
        !injected.some(
          (result) =>
            result.documentId === session.documentId &&
            record(result.result) &&
            result.result.ready === true,
        )
      )
        throw new Error("PAGE_UNAVAILABLE");
      const current = await api.tabs.get(tab.id);
      if (current.url !== tab.url) {
        await fail("PAGE_CHANGED");
        return;
      }
      const response = await event(id, "started");
      if (
        !record(response) ||
        response.protocolVersion !== 1 ||
        response.ok !== true
      )
        await fail("CAPTURE_EXPIRED");
    } catch {
      if (active?.id === id) await fail("PAGE_UNAVAILABLE");
      else {
        remember(id);
        await event(id, "failed", "PAGE_UNAVAILABLE").catch(() => {});
      }
    } finally {
      starting = false;
    }
  }
  async function cancel(id: unknown) {
    if (!uuid(id)) return;
    if (active?.id === id) {
      const session = active;
      active = null;
      remember(id);
      await remove(session);
    }
    await event(id, "cancelled").catch(() => {});
  }
  async function poll() {
    if (polling) return false;
    polling = true;
    try {
      const response = await native({
        protocolVersion: 1,
        kind: "bridge.poll",
        clientId,
      });
      if (
        !record(response) ||
        response.protocolVersion !== 1 ||
        response.ok !== true ||
        !record(response.value) ||
        response.value.ready !== true
      ) {
        if (active) await fail("BRIDGE_UNAVAILABLE");
        return false;
      }
      const commands = response.value.commands;
      if (!Array.isArray(commands) || commands.length > 2) {
        await fail("CAPTURE_INVALID");
        return false;
      }
      for (const command of commands) {
        if (!record(command)) continue;
        if (command.kind === "capture.cancel") await cancel(command.sessionId);
        else if (command.kind === "capture.start") await start(command);
      }
      if (active && active.expiresAt <= Date.now())
        await fail("CAPTURE_EXPIRED");
      return true;
    } catch {
      await fail("BRIDGE_UNAVAILABLE");
      return false;
    } finally {
      polling = false;
    }
  }
  async function handle(
    request: unknown,
    sender: Sender,
  ): Promise<{ alive: boolean }> {
    const session = active;
    if (
      !record(request) ||
      !session ||
      request.sessionId !== session.id ||
      !source(sender, session)
    )
      return { alive: false };
    if (request.kind === "capture.ping")
      return { alive: session.expiresAt > Date.now() };
    if (request.kind !== "capture.page") return { alive: false };
    if (request.phase === "selecting" && session.phase === "waiting") {
      session.phase = "selecting";
      try {
        const response = await event(session.id, "selecting");
        if (
          !record(response) ||
          response.protocolVersion !== 1 ||
          response.ok !== true
        ) {
          await fail("CAPTURE_EXPIRED");
          return { alive: false };
        }
      } catch {
        await fail("BRIDGE_UNAVAILABLE");
        return { alive: false };
      }
      return { alive: active === session };
    }
    if (request.phase === "failed") {
      await fail(
        typeof request.code === "string" ? request.code : "CAPTURE_INVALID",
      );
      return { alive: false };
    }
    if (request.phase === "cancelled") {
      await cancel(session.id);
      return { alive: false };
    }
    if (request.phase !== "completed" || session.phase !== "selecting")
      return { alive: false };
    session.phase = "sending";
    remember(session.id);
    let sent = false;
    try {
      if (
        typeof request.text !== "string" ||
        typeof request.url !== "string" ||
        typeof request.title !== "string" ||
        request.url !== session.url ||
        request.title.length > 500 ||
        /[\uD800-\uDFFF]/u.test(request.title) ||
        session.expiresAt <= Date.now()
      )
        throw new Error("CAPTURE_INVALID");
      const text = normalizeSelection(request.text);
      const url = sanitizeUrl(request.url);
      const tab = await api.tabs.get(session.tabId);
      const [current] = await api.tabs.query({
        active: true,
        lastFocusedWindow: true,
      });
      if (
        tab.url !== session.url ||
        current?.id !== session.tabId ||
        active !== session
      )
        throw new Error("PAGE_CHANGED");
      const live = await api.scripting.executeScript({
        target: { tabId: session.tabId, documentIds: [session.documentId] },
        world: "ISOLATED",
        func: () => location.href,
      });
      if (
        !live.some(
          (frame) =>
            frame.documentId === session.documentId &&
            frame.result === session.url,
        ) ||
        active !== session ||
        session.expiresAt <= Date.now()
      )
        throw new Error("PAGE_CHANGED");
      const envelope = {
        protocolVersion: 1,
        kind: "capture.selection",
        requestId: session.id,
        sentAt: new Date().toISOString(),
        payload: {
          text,
          url,
          title: request.title.normalize("NFC"),
          browser: BROWSER,
          target: session.target,
        },
      };
      if (
        new TextEncoder().encode(JSON.stringify(envelope)).length >
        256 * 1024
      )
        throw new Error("CAPTURE_TOO_LARGE");
      sent = true;
      const response = await native(envelope);
      if (
        !record(response) ||
        response.protocolVersion !== 1 ||
        response.ok !== true ||
        !record(response.value) ||
        response.value.requestId !== session.id
      )
        throw new Error("DELIVERY_UNCONFIRMED");
      if (active === session) active = null;
      return { alive: false };
    } catch (error) {
      await fail(
        sent
          ? "DELIVERY_UNCONFIRMED"
          : error instanceof Error && safeCodes.has(error.message)
            ? error.message
            : "CAPTURE_INVALID",
      );
      return { alive: false };
    }
  }
  api.tabs.onActivated.addListener((info) => {
    if (
      active &&
      active.windowId === info.windowId &&
      active.tabId !== info.tabId
    )
      void fail("PAGE_CHANGED");
  });
  api.tabs.onRemoved.addListener((id) => {
    if (active?.tabId === id) void fail("PAGE_CHANGED");
  });
  api.tabs.onUpdated.addListener((id, change) => {
    if (
      active?.tabId === id &&
      (change.status === "loading" || (change.url && change.url !== active.url))
    )
      void fail("PAGE_CHANGED");
  });
  api.windows.onFocusChanged.addListener((id) => {
    if (
      active &&
      id >= 0 &&
      active.windowId !== undefined &&
      id !== active.windowId
    )
      void fail("PAGE_CHANGED");
  });
  return { poll, handle };
}
