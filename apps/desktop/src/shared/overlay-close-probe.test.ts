import { expect, it } from "vitest";
import { probeOverlayClose } from "./overlay-close-probe";

it("uses only the matching close attempt and removes the listener", async () => {
  let reply: ((value: { attempt: string; dirty: boolean }) => void) | null =
    null;
  let stopped = false;
  const result = probeOverlayClose("current", {
    listen: async (handler) => {
      reply = handler;
      return () => {
        stopped = true;
      };
    },
    emit: async () => {
      reply?.({ attempt: "old", dirty: false });
      reply?.({ attempt: "current", dirty: true });
    },
  });
  await expect(result).resolves.toBe(true);
  expect(stopped).toBe(true);
});

it("fails closed when the overlay does not answer", async () => {
  let stopped = false;
  await expect(
    probeOverlayClose(
      "current",
      {
        listen: async () => () => {
          stopped = true;
        },
        emit: async () => {},
      },
      1,
    ),
  ).rejects.toThrow("OVERLAY_CLOSE_TIMEOUT");
  expect(stopped).toBe(true);
});
