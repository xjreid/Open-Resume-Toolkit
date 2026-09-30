import type { ChromeApi, Port } from "./chrome-api.js";
import { NATIVE_HOST } from "./bridge-config.js";

// One persistent native port keeps MV3's worker alive. Each request has exactly
// one response, in order. Content is never retried after an uncertain delivery.
export function createNativeClient(api: ChromeApi, timeoutMs = 5_000) {
  let port: Port | null = null;
  let waiting: {
    resolve: (value: unknown) => void;
    reject: (error: Error) => void;
    timer: ReturnType<typeof setTimeout>;
  } | null = null;
  let queue: Promise<unknown> = Promise.resolve();
  function disconnected(active: Port) {
    if (port !== active) return;
    port = null;
    const request = waiting;
    waiting = null;
    if (request) {
      clearTimeout(request.timer);
      request.reject(new Error("BRIDGE_UNAVAILABLE"));
    }
  }
  function connection() {
    if (port) return port;
    const active = api.runtime.connectNative(NATIVE_HOST);
    port = active;
    active.onMessage.addListener((value) => {
      if (port !== active || !waiting) return;
      const request = waiting;
      waiting = null;
      clearTimeout(request.timer);
      request.resolve(value);
    });
    active.onDisconnect.addListener(() => {
      void api.runtime.lastError;
      disconnected(active);
    });
    return active;
  }
  function request(message: unknown): Promise<unknown> {
    const operation = queue.then(
      () =>
        new Promise<unknown>((resolve, reject) => {
          try {
            const active = connection();
            const timer = setTimeout(() => {
              disconnected(active);
              active.disconnect();
              reject(new Error("DELIVERY_UNCONFIRMED"));
            }, timeoutMs);
            waiting = { resolve, reject, timer };
            active.postMessage(message);
          } catch {
            if (port) {
              const active = port;
              disconnected(active);
              active.disconnect();
            }
            reject(new Error("BRIDGE_UNAVAILABLE"));
          }
        }),
    );
    queue = operation.catch(() => {});
    return operation;
  }
  return { request };
}
