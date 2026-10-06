import { expect, it, vi } from "vitest";
import { SaveCoordinator } from "./save-coordinator";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

it("coalesces typing during a write and serializes acknowledgements", async () => {
  const pending = { current: "first" as string | null };
  const running = { current: null as Promise<void> | null };
  const first = deferred<number>();
  const commit = vi
    .fn()
    .mockReturnValueOnce(first.promise)
    .mockResolvedValueOnce(2);
  const acknowledge = vi.fn();
  const queue = new SaveCoordinator(
    pending,
    running,
    commit,
    acknowledge,
    vi.fn(),
  );
  const flush = queue.flush();
  pending.current = "discarded intermediate";
  pending.current = "latest";
  first.resolve(1);
  await flush;
  expect(commit.mock.calls).toEqual([["first"], ["latest"]]);
  expect(acknowledge.mock.calls).toEqual([
    [1, "first"],
    [2, "latest"],
  ]);
  expect(pending.current).toBeNull();
  expect(running.current).toBeNull();
});

it("retains the newest edit after failure and retries it explicitly", async () => {
  const pending = { current: "first" as string | null };
  const running = { current: null as Promise<void> | null };
  const first = deferred<number>();
  const commit = vi
    .fn()
    .mockReturnValueOnce(first.promise)
    .mockResolvedValueOnce(2);
  const status = vi.fn();
  const queue = new SaveCoordinator(pending, running, commit, vi.fn(), status);
  const flush = queue.flush();
  pending.current = "latest";
  first.reject(new Error("conflict"));
  await expect(flush).rejects.toThrow("conflict");
  expect(pending.current).toBe("latest");
  expect(status).toHaveBeenLastCalledWith("error");
  await queue.flush();
  expect(commit.mock.calls).toEqual([["first"], ["latest"]]);
});

it("retires a profile queue without sending pending work or delivering old acknowledgements", async () => {
  const pending = { current: "old" as string | null };
  const running = { current: null as Promise<void> | null };
  const first = deferred<number>();
  const commit = vi.fn().mockReturnValue(first.promise);
  const ack = vi.fn();
  const queue = new SaveCoordinator(pending, running, commit, ack, vi.fn());
  const flush = queue.flush();
  pending.current = "queued old edit";
  queue.dispose();
  first.resolve(1);
  await flush;
  await queue.flush();
  expect(commit).toHaveBeenCalledTimes(1);
  expect(ack).not.toHaveBeenCalled();
  expect(pending.current).toBeNull();
});
