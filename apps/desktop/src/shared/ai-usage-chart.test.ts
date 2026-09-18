import { expect, it } from "vitest";
import { chartBuckets, totalTokens, type UsageBucket } from "./AiUsageChart";

const bucket = (label: string): UsageBucket => ({
  label,
  attempts: 1,
  usage: {
    inputTokens: 10,
    outputTokens: 5,
    cachedInputTokens: 3,
    cacheWriteTokens: 2,
    reasoningTokens: 1,
  },
  costByCurrencyMicros: { USD: 42 },
  partial: false,
  unknownCount: 0,
});

it("totals all normalized token categories without dropping caches or reasoning", () => {
  expect(totalTokens(bucket("2026-09-15").usage)).toBe(21);
});

it("fills quiet days and starts weeks on Monday", () => {
  const points = chartBuckets(
    [bucket("2026-09-15")],
    "Week",
    new Date(2026, 8, 17),
  );
  expect(points.map((point) => point.label)).toEqual([
    "2026-09-14",
    "2026-09-15",
    "2026-09-16",
    "2026-09-17",
  ]);
  expect(points[0].attempts).toBe(0);
  expect(totalTokens(points[1].usage)).toBe(21);
});

it("fills monthly and all-time gaps without inventing pre-history", () => {
  const now = new Date(2026, 8, 17);
  expect(chartBuckets([], "Month", now)).toHaveLength(17);
  expect(chartBuckets([], "Year", now)).toHaveLength(9);
  expect(
    chartBuckets([bucket("2025-12"), bucket("2026-09")], "All time", now),
  ).toHaveLength(10);
  expect(chartBuckets([], "All time", now)).toHaveLength(1);
});

it("scales long all-time history to years while preserving token types, currencies and unknowns", () => {
  const partial = {
    ...bucket("2023-02"),
    partial: true,
    unknownCount: 1,
    costByCurrencyMicros: { USD: 42, EUR: 10 },
  };
  const points = chartBuckets(
    [bucket("2023-01"), partial],
    "All time",
    new Date(2026, 8, 17),
  );
  expect(points.map((point) => point.label)).toEqual([
    "2023",
    "2024",
    "2025",
    "2026",
  ]);
  expect(totalTokens(points[0].usage)).toBe(42);
  expect(points[0].usage.cachedInputTokens).toBe(6);
  expect(points[0].costByCurrencyMicros).toEqual({ USD: 84, EUR: 10 });
  expect(points[0].partial).toBe(true);
  expect(points[0].unknownCount).toBe(1);
  expect(points[1].attempts).toBe(0);
});
