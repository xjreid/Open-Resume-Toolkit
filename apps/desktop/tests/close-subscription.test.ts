import { describe, expect, it, vi } from "vitest";
import { setImmediate } from "node:timers/promises";
import type { CloseCommandResponse } from "@ort/contracts/lifecycle";
import { subscribeToCloseRequests } from "../src/shared/close-subscription";

const noAttempt: CloseCommandResponse = {
  ok: true,
  value: { pendingAttempt: null },
};
const attempt: CloseCommandResponse = {
  ok: true,
  value: { pendingAttempt: "01990000-0000-7000-8000-000000000000" },
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
const flush = () => setImmediate();

describe("close event subscription", () => {
  it("fetches a native attempt missed before listener registration", async () => {
    const receive = vi.fn();
    const stop = vi.fn();
    const subscription = subscribeToCloseRequests(
      { listen: async () => stop, status: async () => attempt },
      receive,
      vi.fn(),
    );
    await flush();
    expect(receive).toHaveBeenCalledWith(attempt);
    subscription.dispose();
    expect(stop).toHaveBeenCalledOnce();
  });

  it("ignores old status responses and treats spoofed events only as wakeups", async () => {
    const old = deferred<CloseCommandResponse>();
    let wake!: () => void;
    const receive = vi.fn();
    const status = vi
      .fn()
      .mockReturnValueOnce(old.promise)
      .mockResolvedValue(noAttempt);
    const subscription = subscribeToCloseRequests(
      {
        listen: async (callback) => {
          wake = callback;
          return () => {};
        },
        status,
      },
      receive,
      vi.fn(),
    );
    await flush();
    wake();
    await flush();
    old.resolve(attempt);
    await flush();
    expect(receive).toHaveBeenCalledExactlyOnceWith(noAttempt);
    subscription.dispose();
  });

  it("does not resurrect a cancelled attempt from an in-flight status read", async () => {
    const old = deferred<CloseCommandResponse>();
    const receive = vi.fn();
    const status = vi
      .fn()
      .mockReturnValueOnce(old.promise)
      .mockResolvedValue(noAttempt);
    const subscription = subscribeToCloseRequests(
      { listen: async () => () => {}, status },
      receive,
      vi.fn(),
    );
    await flush();
    subscription.pause();
    old.resolve(attempt);
    await flush();
    expect(receive).not.toHaveBeenCalled();
    subscription.resume();
    await flush();
    expect(receive).toHaveBeenCalledExactlyOnceWith(noAttempt);
    subscription.dispose();
  });

  it("cleans up a late listener registration after StrictMode disposal", async () => {
    const registration = deferred<() => void>();
    const stop = vi.fn();
    const status = vi.fn();
    const subscription = subscribeToCloseRequests(
      { listen: () => registration.promise, status },
      vi.fn(),
      vi.fn(),
    );
    subscription.dispose();
    registration.resolve(stop);
    await flush();
    expect(stop).toHaveBeenCalledOnce();
    expect(status).not.toHaveBeenCalled();
  });

  it("surfaces registration failure without approving a quit", async () => {
    const receive = vi.fn();
    const failure = vi.fn();
    const subscription = subscribeToCloseRequests(
      {
        listen: () => {
          throw new Error("synthetic");
        },
        status: async () => attempt,
      },
      receive,
      failure,
    );
    await flush();
    expect(failure).toHaveBeenCalledOnce();
    expect(receive).not.toHaveBeenCalled();
    subscription.dispose();
  });
});
