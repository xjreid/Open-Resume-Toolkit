import { useState } from "react";

export type Usage = {
  inputTokens: number;
  cachedInputTokens?: number;
  cacheWriteTokens?: number;
  outputTokens: number;
  reasoningTokens?: number;
};
export type UsageBucket = {
  label: string;
  attempts: number;
  usage: Usage;
  costByCurrencyMicros: Record<string, number>;
  partial: boolean;
  unknownCount: number;
};
export type UsagePeriod = "Week" | "Month" | "Year" | "All time";
export const totalTokens = (usage: Usage) =>
  usage.inputTokens +
  (usage.cachedInputTokens ?? 0) +
  (usage.cacheWriteTokens ?? 0) +
  usage.outputTokens +
  (usage.reasoningTokens ?? 0);

// Calendar buckets include quiet intervals; spacing never compresses gaps in time.
export function chartBuckets(
  buckets: UsageBucket[],
  period: UsagePeriod,
  now = new Date(),
) {
  const monthly = period === "Year" || period === "All time";
  const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  if (period === "Week")
    start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
  if (period === "Month") start.setDate(1);
  if (monthly) start.setDate(1);
  if (period === "Year") start.setMonth(0);
  if (period === "All time" && buckets.length) {
    const first = buckets.map((bucket) => bucket.label).sort()[0];
    const [year, month] = first.split("-").map(Number);
    start.setFullYear(year, month - 1, 1);
  }
  const byLabel = new Map(buckets.map((bucket) => [bucket.label, bucket]));
  const result: UsageBucket[] = [];
  for (
    const date = new Date(start);
    date <= now;
    monthly
      ? date.setMonth(date.getMonth() + 1)
      : date.setDate(date.getDate() + 1)
  ) {
    const label = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}${monthly ? "" : `-${String(date.getDate()).padStart(2, "0")}`}`;
    result.push(
      byLabel.get(label) ?? {
        label,
        attempts: 0,
        usage: { inputTokens: 0, outputTokens: 0 },
        costByCurrencyMicros: {},
        partial: false,
        unknownCount: 0,
      },
    );
  }
  if (period !== "All time" || result.length <= 24) return result;
  const years = new Map<string, UsageBucket>();
  for (const bucket of result) {
    const label = bucket.label.slice(0, 4);
    const year = years.get(label) ?? {
      label,
      attempts: 0,
      usage: { inputTokens: 0, outputTokens: 0 },
      costByCurrencyMicros: {},
      partial: false,
      unknownCount: 0,
    };
    year.attempts += bucket.attempts;
    year.partial ||= bucket.partial;
    year.unknownCount += bucket.unknownCount;
    for (const category of [
      "inputTokens",
      "outputTokens",
      "cachedInputTokens",
      "cacheWriteTokens",
      "reasoningTokens",
    ] as const) {
      year.usage[category] =
        (year.usage[category] ?? 0) + (bucket.usage[category] ?? 0);
    }
    for (const [currency, cost] of Object.entries(
      bucket.costByCurrencyMicros,
    )) {
      year.costByCurrencyMicros[currency] =
        (year.costByCurrencyMicros[currency] ?? 0) + cost;
    }
    years.set(label, year);
  }
  return [...years.values()];
}

export function AiUsageChart({
  buckets,
  period,
  currency,
  metric,
}: {
  buckets: UsageBucket[];
  period: UsagePeriod;
  currency: string;
  metric: "cost" | "tokens";
}) {
  const [active, setActive] = useState<number | null>(null);
  const points = chartBuckets(buckets, period);
  const values = points.map((bucket) =>
    metric === "tokens"
      ? totalTokens(bucket.usage)
      : (bucket.costByCurrencyMicros[currency] ?? 0) / 1_000_000,
  );
  const rawStep = Math.max(metric === "tokens" ? 4 : 0.000004, ...values) / 4;
  const magnitude = 10 ** Math.floor(Math.log10(rawStep));
  const step =
    ([1, 2, 5, 10].find((value) => value * magnitude >= rawStep) ?? 10) *
    magnitude;
  const maximum = step * 4;
  const timestamps = points.map((bucket) => {
    const [year, month, day] = bucket.label.split("-").map(Number);
    return new Date(year, (month ?? 1) - 1, day ?? 1).getTime();
  });
  const span = timestamps[timestamps.length - 1] - timestamps[0];
  const x = (index: number) =>
    78 +
    (span === 0 ? 350 : ((timestamps[index] - timestamps[0]) * 700) / span);
  const y = (value: number) => 230 - (value / maximum) * 190;
  const selected =
    active === null ? null : points[Math.min(active, points.length - 1)];
  const format = (value: number) =>
    metric === "tokens" ? Math.round(value).toLocaleString() : value.toFixed(6);
  const dateLabel = (label: string) => {
    if (label.length === 4) return label;
    const [year, month, day] = label.split("-").map(Number);
    return new Date(year, month - 1, day ?? 1).toLocaleDateString(
      undefined,
      period === "Week"
        ? { weekday: "short", day: "numeric" }
        : period === "Month"
          ? { month: "short", day: "numeric" }
          : { month: "short", year: "2-digit" },
    );
  };
  const ticks = [
    ...new Set([
      0,
      Math.floor((points.length - 1) / 4),
      Math.floor((points.length - 1) / 2),
      Math.floor(((points.length - 1) * 3) / 4),
      points.length - 1,
    ]),
  ];
  return (
    <div className="ai-chart" aria-label="Token usage over time">
      <p className="ai-chart__axis">
        {metric === "tokens"
          ? "Estimated tokens"
          : `Estimated price (${currency})`}
      </p>
      <div className="ai-chart__plot">
        <svg
          viewBox="0 0 810 280"
          role="group"
          aria-label={`${metric === "tokens" ? "Estimated tokens" : "Estimated price"} over time. Focus a point for details.`}
          onPointerMove={(event) => {
            const rect = event.currentTarget.getBoundingClientRect();
            const position = ((event.clientX - rect.left) / rect.width) * 810;
            setActive(
              points.reduce(
                (closest, _, index) =>
                  Math.abs(x(index) - position) <
                  Math.abs(x(closest) - position)
                    ? index
                    : closest,
                0,
              ),
            );
          }}
          onPointerLeave={() => setActive(null)}
        >
          {[0, 0.25, 0.5, 0.75, 1].map((fraction) => (
            <g key={fraction}>
              <line
                x1="78"
                x2="778"
                y1={y(maximum * fraction)}
                y2={y(maximum * fraction)}
                className="ai-chart__grid"
              />
              <text x="68" y={y(maximum * fraction) + 4} textAnchor="end">
                {metric === "tokens"
                  ? (maximum * fraction).toLocaleString(undefined, {
                      notation: "compact",
                    })
                  : format(maximum * fraction)}
              </text>
            </g>
          ))}
          <polyline
            points={values
              .map((value, index) => `${x(index)},${y(value)}`)
              .join(" ")}
            className="ai-chart__line"
          />
          {points.map((bucket, index) => (
            <circle
              key={bucket.label}
              cx={x(index)}
              cy={y(values[index])}
              r={active === index ? 6 : 3.5}
              tabIndex={0}
              role="button"
              aria-describedby={
                active === index ? "ai-chart-tooltip" : undefined
              }
              aria-label={`${bucket.label}: ${format(values[index])} ${metric === "tokens" ? "tokens" : currency}${bucket.partial ? ", partial usage" : ""}`}
              onFocus={() => setActive(index)}
              onBlur={() => setActive(null)}
              onClick={() => setActive(index)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  setActive(index);
                }
                if (event.key === "Escape") setActive(null);
              }}
              className="ai-chart__point"
            />
          ))}
          {ticks.map((index) => (
            <text key={index} x={x(index)} y="259" textAnchor="middle">
              {dateLabel(points[index].label)}
            </text>
          ))}
        </svg>
        {selected && active !== null && (
          <div
            className="ai-chart__tooltip"
            role="tooltip"
            id="ai-chart-tooltip"
            aria-live="polite"
            style={{
              left: `${(x(Math.min(active, points.length - 1)) / 810) * 100}%`,
              top: `${(y(values[Math.min(active, points.length - 1)]) / 280) * 100}%`,
              transform: `translate(${x(active) < 260 ? "0%" : x(active) > 590 ? "-100%" : "-50%"}, calc(-100% - 14px))`,
            }}
          >
            <>
              <strong>
                {selected.label}:{" "}
                {metric === "tokens"
                  ? `${totalTokens(selected.usage).toLocaleString()} tokens`
                  : `${((selected.costByCurrencyMicros[currency] ?? 0) / 1_000_000).toFixed(6)} ${currency}`}
              </strong>
              {metric === "tokens" && (
                <dl>
                  {[
                    ["Input", selected.usage.inputTokens],
                    ["Output", selected.usage.outputTokens],
                    ["Cached input", selected.usage.cachedInputTokens ?? 0],
                    ["Cache write", selected.usage.cacheWriteTokens ?? 0],
                    ["Reasoning", selected.usage.reasoningTokens ?? 0],
                  ].map(([label, value]) => (
                    <div key={label as string}>
                      <dt>{label}</dt>
                      <dd>{value.toLocaleString()}</dd>
                    </div>
                  ))}
                </dl>
              )}
              {selected.partial && (
                <span>
                  Partial usage · {selected.unknownCount} unknown attempts.
                  Missing usage is not zero spend.
                </span>
              )}
            </>
          </div>
        )}
      </div>
    </div>
  );
}
