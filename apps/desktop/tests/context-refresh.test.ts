import { afterEach, expect, it, vi } from "vitest";
import { ContextRefresh } from "../src/shared/context-refresh";
const flush = async () => {
  await Promise.resolve();
  await Promise.resolve();
};
afterEach(() => vi.useRealTimers());
it("coalesces concurrent events, stops hidden polling, and discards queued hidden work", async () => {
  vi.useFakeTimers();
  let finish: (() => void) | undefined;
  const fetch = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  const refresh = new ContextRefresh(fetch, 10_000);
  refresh.setVisible(true);
  refresh.refresh();
  refresh.refresh();
  refresh.refresh();
  expect(fetch).toHaveBeenCalledTimes(1);
  finish!();
  await flush();
  expect(fetch).toHaveBeenCalledTimes(2);
  refresh.refresh();
  refresh.setVisible(false);
  finish!();
  await flush();
  await vi.advanceTimersByTimeAsync(60_000);
  expect(fetch).toHaveBeenCalledTimes(2);
  refresh.setVisible(true);
  expect(fetch).toHaveBeenCalledTimes(3);
  finish!();
  await flush();
  await vi.advanceTimersByTimeAsync(9_999);
  expect(fetch).toHaveBeenCalledTimes(3);
  await vi.advanceTimersByTimeAsync(1);
  expect(fetch).toHaveBeenCalledTimes(4);
  refresh.stop();
  finish!();
  await flush();
  await vi.advanceTimersByTimeAsync(60_000);
  expect(fetch).toHaveBeenCalledTimes(4);
});
