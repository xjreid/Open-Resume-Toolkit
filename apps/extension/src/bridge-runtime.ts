import type { ChromeApi } from "./chrome-api.js";

export const RECONNECT_ALARM = "ort-bridge-reconnect";

// Native ports keep MV3 workers alive while ORT is connected. When ORT is
// absent, release the port and use an alarm that survives worker suspension.
// Only content-free polls are retried; captures never enter this scheduler.
export function createBridgeRuntime(
  api: ChromeApi,
  poll: () => Promise<boolean>,
  close: () => void,
  timers = {
    setTimeout: (callback: () => void, delay: number) =>
      setTimeout(callback, delay),
    clearTimeout: (id: ReturnType<typeof setTimeout>) => clearTimeout(id),
  },
) {
  let running: Promise<void> | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  // Recreate on each worker start. Keep the alarm while connected too, so an
  // unexpected worker/host exit still has a browser-owned recovery event.
  void api.alarms
    .create(RECONNECT_ALARM, { periodInMinutes: 0.5 })
    .catch(() => {});
  async function tick() {
    let connected = false;
    try {
      connected = await poll();
    } catch {
      // Protocol/controller failures have the same safe disconnected boundary.
    }
    if (connected) {
      timer = timers.setTimeout(() => void wake(), 500);
    } else {
      close();
    }
  }
  function wake() {
    if (running) return running;
    if (timer !== undefined) timers.clearTimeout(timer);
    timer = undefined;
    running = tick().finally(() => {
      running = null;
    });
    return running;
  }
  api.alarms.onAlarm.addListener((alarm) => {
    if (alarm.name === RECONNECT_ALARM) void wake();
  });
  api.runtime.onStartup.addListener(() => void wake());
  api.runtime.onInstalled.addListener(() => void wake());
  return { wake };
}
