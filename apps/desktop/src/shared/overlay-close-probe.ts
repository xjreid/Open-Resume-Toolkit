type CloseReply = { attempt: string; dirty: boolean };

export async function probeOverlayClose(
  attempt: string,
  transport: {
    listen: (reply: (value: CloseReply) => void) => Promise<() => void>;
    emit: (attempt: string) => Promise<void>;
  },
  timeoutMs = 3000,
): Promise<boolean> {
  let stop: (() => void) | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await new Promise<boolean>((resolve, reject) => {
      let settled = false;
      const finish = (value: boolean | Error) => {
        if (settled) return;
        settled = true;
        if (value instanceof Error) reject(value);
        else resolve(value);
      };
      timer = setTimeout(
        () => finish(new Error("OVERLAY_CLOSE_TIMEOUT")),
        timeoutMs,
      );
      void transport
        .listen((reply) => {
          if (reply.attempt === attempt && typeof reply.dirty === "boolean")
            finish(reply.dirty);
        })
        .then((unlisten) => {
          if (settled) {
            unlisten();
            return;
          }
          stop = unlisten;
          void transport
            .emit(attempt)
            .catch((error: unknown) =>
              finish(error instanceof Error ? error : new Error(String(error))),
            );
        })
        .catch((error: unknown) =>
          finish(error instanceof Error ? error : new Error(String(error))),
        );
    });
  } finally {
    if (timer) clearTimeout(timer);
    stop?.();
  }
}
