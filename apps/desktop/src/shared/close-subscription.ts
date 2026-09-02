import type { CloseCommandResponse } from "@ort/contracts/lifecycle";

export interface CloseTransport {
  listen: (wake: () => void) => Promise<() => void>;
  status: () => Promise<CloseCommandResponse>;
}

// Events are notifications only. Fetch main-window-authorized native state,
// including attempts made before listener registration or after a renderer reload.
export function subscribeToCloseRequests(
  transport: CloseTransport,
  onStatus: (result: CloseCommandResponse) => void,
  onListenFailure: () => void,
) {
  let disposed = false;
  let generation = 0;
  let unlisten: (() => void) | undefined;
  let paused = false;
  const refresh = async () => {
    if (disposed || paused) return;
    const current = ++generation;
    const result = await transport.status();
    if (!disposed && !paused && current === generation) onStatus(result);
  };
  void Promise.resolve()
    .then(() => transport.listen(() => void refresh()))
    .then(
      (stop) => {
        if (disposed) stop();
        else {
          unlisten = stop;
          void refresh();
        }
      },
      () => {
        if (!disposed) onListenFailure();
      },
    );
  return {
    pause() {
      paused = true;
      generation += 1;
    },
    resume() {
      paused = false;
      void refresh();
    },
    dispose() {
      disposed = true;
      generation += 1;
      unlisten?.();
    },
  };
}
