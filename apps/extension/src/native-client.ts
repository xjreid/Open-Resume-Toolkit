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
  let pending = 0;
  let generation = 0;
  const maxFrameBytes = 256 * 1024;
  function disconnected(active: Port) {
    if (port !== active) return;
    port = null;
    generation += 1;
    const request = waiting;
    waiting = null;
    if (request) {
      clearTimeout(request.timer);
      request.reject(new Error("DELIVERY_UNCONFIRMED"));
    }
  }
  function connection() {
    if (port) return port;
    const active = api.runtime.connectNative(NATIVE_HOST);
    port = active;
    active.onMessage.addListener((value) => {
      if (port !== active) return;
      if (!waiting) {
        close();
        return;
      }
      try {
        const encoded = JSON.stringify(value);
        if (
          !encoded ||
          new TextEncoder().encode(encoded).length > maxFrameBytes
        ) {
          close();
          return;
        }
      } catch {
        close();
        return;
      }
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
  function close() {
    if (!port) {
      generation += 1;
      return;
    }
    const active = port;
    disconnected(active);
    active.disconnect();
  }
  function request(message: unknown): Promise<unknown> {
    try {
      const encoded = JSON.stringify(message);
      if (!encoded || new TextEncoder().encode(encoded).length > maxFrameBytes)
        return Promise.reject(new Error("CAPTURE_TOO_LARGE"));
    } catch {
      return Promise.reject(new Error("CAPTURE_INVALID"));
    }
    if (pending >= 8) return Promise.reject(new Error("BRIDGE_UNAVAILABLE"));
    pending += 1;
    const expectedGeneration = generation;
    const operation = queue.then(
      () =>
        new Promise<unknown>((resolve, reject) => {
          try {
            // A disconnected/expired connection invalidates its entire queue.
            // Do not forward queued page content through a fresh connection.
            if (generation !== expectedGeneration)
              throw new Error("BRIDGE_UNAVAILABLE");
            const active = connection();
            const timer = setTimeout(() => {
              close();
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
    const settled = operation.finally(() => {
      pending -= 1;
    });
    queue = settled.catch(() => {});
    return settled;
  }
  return { request, close };
}
