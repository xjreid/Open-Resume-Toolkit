// Development-only preview. No native commands, provider calls or persistent writes.
import { useState } from "react";
import { createRoot } from "react-dom/client";
import { AiWorkspace } from "../src/shared/AiWorkspace";
import { AppShell } from "../src/shared/AppShell";
import { type Usage, type UsageBucket } from "../src/shared/AiUsageChart";
import "../src/shared/app.css";

type Provider = "openai" | "anthropic" | "gemini";
type SavedKey = {
  credentialId: string;
  identificationNumber: number;
  name?: string | null;
  provider: Provider;
  preset: "balanced";
  paused: boolean;
  removed: boolean;
  cleanupRequired: boolean;
};
type Registry = {
  keys: SavedKey[];
  primaryCredentialId: string | null;
  nextIdentificationNumber: number;
};
const now = new Date();
const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
const dateLabel = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
const seed = () =>
  Array.from({ length: 240 }, (_, index) => {
    const date = new Date(
      now.getFullYear(),
      now.getMonth(),
      now.getDate() - 239 + index,
    );
    const input = Math.round(
      2200 + 1400 * Math.sin(index / 5) + (index % 7) * 280,
    );
    return {
      at: date.getTime(),
      credentialId: `preview-key-${(index % 3) + 1}`,
      label: dateLabel(date),
      attempts: index % 6 === 0 ? 0 : 2,
      usage: {
        inputTokens: input,
        outputTokens: 540 + (index % 400),
        cachedInputTokens: 320,
        cacheWriteTokens: 80,
        reasoningTokens: 120,
      },
      costByCurrencyMicros: { USD: input * 2 + (540 + (index % 400)) * 8 },
      partial: false,
      unknownCount: 0,
    };
  }).filter((bucket) => bucket.attempts > 0);
let registry: Registry;
let records = seed();
let lifetimeRecords = records;
let caps: Array<Record<string, any>>;
let retention = "retain_until_cleared";
let partial = false;
const cap = (period: string, limitMicros: number, credentialId: string) => ({
  credentialId,
  period,
  currency: "USD",
  timeZone,
  limitMicros,
  activatedAtUnixMs: now.getTime(),
  periodStartUnixMs: now.getTime(),
  periodEndUnixMs:
    period === "all_time"
      ? null
      : new Date(now.getFullYear(), now.getMonth() + 1, 1).getTime(),
  countedMicros: Math.min(
    720000,
    lifetimeRecords
      .filter((record) => record.credentialId === credentialId)
      .reduce((total, record) => total + record.costByCurrencyMicros.USD, 0),
  ),
  reservedMicros: 0,
  unresolvedMicros: 0,
  revision: 1,
});
type Scenario = "connected" | "setup" | "partial" | "no_primary";
function reset(mode: Scenario) {
  registry = {
    keys:
      mode === "setup"
        ? []
        : (["openai", "anthropic", "gemini"] as const).map(
            (provider, index) => ({
              credentialId: `preview-key-${index + 1}`,
              identificationNumber: index + 1,
              provider,
              preset: "balanced",
              paused: index === 2,
              removed: false,
              cleanupRequired: false,
            }),
          ),
    primaryCredentialId:
      mode === "setup" || mode === "no_primary" ? null : "preview-key-1",
    nextIdentificationNumber: mode === "setup" ? 1 : 4,
  };
  records = mode === "setup" ? [] : seed();
  lifetimeRecords = records;
  caps =
    mode === "setup"
      ? []
      : [
          cap("all_time", 10000000, "preview-key-1"),
          cap("all_time", 5000000, "preview-key-2"),
        ];
  partial = mode === "partial";
  retention = "retain_until_cleared";
}
reset("connected");
const catalog = {
  catalogId: "sample-ui-data",
  expiresAt: "2027-01-01",
  entries: (["openai", "anthropic", "gemini"] as const).map((provider) => ({
    provider: provider === "openai" ? "open_ai" : provider,
    model: `Sample ${provider === "openai" ? "OpenAI" : provider === "anthropic" ? "Anthropic" : "Gemini"} model`,
    preset: "balanced",
    disabled: false,
    operations: ["credential_test"],
    maxInputTokens: 64000,
    maxOutputTokens: 6000,
    currency: "USD",
    prices: [
      { category: "input", microsPerMillion: 2000000 },
      { category: "output", microsPerMillion: 8000000 },
      { category: "cached_input", microsPerMillion: 500000 },
    ],
    source: "Sample pricing for UI inspection — not actual provider rates",
    verifiedAt: "2026-09-17",
    effectiveFrom: "2026-09-17",
    effectiveTo: null,
  })),
};

function monitoring(args: Record<string, any>) {
  const selected = records.filter(
    (record) =>
      record.at >= args.fromUnixMs &&
      record.at < args.toUnixMs &&
      (!args.credentialId || record.credentialId === args.credentialId),
  );
  const buckets = new Map<string, UsageBucket>();
  const usage: Usage = {
    inputTokens: 0,
    outputTokens: 0,
    cachedInputTokens: 0,
    cacheWriteTokens: 0,
    reasoningTokens: 0,
  };
  let cost = 0;
  for (const record of selected) {
    const label =
      args.bucketSize === "month" ? record.label.slice(0, 7) : record.label;
    const bucket = buckets.get(label) ?? {
      label,
      attempts: 0,
      usage: { inputTokens: 0, outputTokens: 0 },
      costByCurrencyMicros: { USD: 0 },
      partial: false,
      unknownCount: 0,
    };
    bucket.attempts += record.attempts;
    for (const category of Object.keys(usage) as Array<keyof Usage>) {
      usage[category] = (usage[category] ?? 0) + record.usage[category];
      bucket.usage[category] =
        (bucket.usage[category] ?? 0) + record.usage[category];
    }
    cost += record.costByCurrencyMicros.USD;
    bucket.costByCurrencyMicros.USD += record.costByCurrencyMicros.USD;
    buckets.set(label, bucket);
  }
  if (partial && buckets.size) {
    const bucket = [...buckets.values()].at(-1)!;
    bucket.partial = true;
    bucket.unknownCount = 1;
  }
  return {
    logicalOperations: selected.length,
    attempts: selected.length * 2,
    usage,
    costByCurrencyMicros: selected.length ? { USD: cost } : {},
    estimatedCostMicros: cost,
    currency: "USD",
    unresolvedReservedMicros: partial ? 120000 : 0,
    partial: partial && selected.length > 0,
    unknownCount: partial ? 1 : 0,
    byProvider: Object.fromEntries(
      registry.keys.map((key) => [
        key.provider,
        selected
          .filter((record) => record.credentialId === key.credentialId)
          .reduce((sum, record) => sum + record.attempts, 0),
      ]),
    ),
    byCredentialId: Object.fromEntries(
      registry.keys.map((key) => [
        key.credentialId,
        selected
          .filter((record) => record.credentialId === key.credentialId)
          .reduce((sum, record) => sum + record.attempts, 0),
      ]),
    ),
    byModel: { "Sample model": selected.length * 2 },
    byPreset: { balanced: selected.length * 2 },
    byStatus: { succeeded: selected.length * 2 },
    byOperationType: { credential_test: selected.length },
    timeBuckets: [...buckets.values()],
  };
}

// Credentials are discarded, never logged or stored. All controls use memory-only fixtures.
Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    transformCallback: () => 1,
    unregisterCallback: () => {},
    invoke: async (command: string, args: Record<string, any> = {}) => {
      let value: unknown;
      switch (command) {
        case "load_ai_connection":
          value = registry;
          break;
        case "load_ai_catalog":
          value = catalog;
          break;
        case "load_ai_caps":
          value = caps.filter(
            (item) => item.credentialId === registry.primaryCredentialId,
          );
          break;
        case "load_ai_key_settings":
          value = {
            lifetimeSpendPartial: partial,
            cap:
              caps.find((item) => item.credentialId === args.credentialId) ??
              null,
            lifetimeSpendByCurrencyMicros: {
              USD: lifetimeRecords
                .filter((item) => item.credentialId === args.credentialId)
                .reduce((sum, item) => sum + item.costByCurrencyMicros.USD, 0),
            },
          };
          break;
        case "load_ai_general_settings":
          value = {
            lifetimeSpendPartial: partial,
            cap: caps.find((item) => item.credentialId === "general") ?? null,
            lifetimeSpendByCurrencyMicros: {
              USD: lifetimeRecords.reduce(
                (sum, item) => sum + item.costByCurrencyMicros.USD,
                0,
              ),
            },
          };
          break;
        case "set_ai_key_preset":
          registry = {
            ...registry,
            keys: registry.keys.map((key) =>
              key.credentialId === args.request.credentialId
                ? { ...key, preset: args.request.preset }
                : key,
            ),
          };
          value = registry;
          break;
        case "load_ai_retention":
          value = { policy: retention, removedOperations: 0 };
          break;
        case "load_ai_monitoring":
          value = monitoring(args);
          break;
        case "add_ai_key": {
          const number = registry.nextIdentificationNumber;
          registry = {
            ...registry,
            nextIdentificationNumber: number + 1,
            keys: [
              ...registry.keys,
              {
                credentialId: `preview-key-${number}`,
                identificationNumber: number,
                provider: args.request.provider,
                preset: "balanced",
                paused: false,
                removed: false,
                cleanupRequired: false,
              },
            ],
          };
          value = registry;
          break;
        }
        case "rename_ai_key": {
          registry = {
            ...registry,
            keys: registry.keys.map((key) =>
              key.credentialId === args.request.credentialId
                ? { ...key, name: args.request.name.trim() || null }
                : key,
            ),
          };
          value = registry;
          break;
        }
        case "change_ai_key": {
          const { credentialId, action } = args.request;
          const key = registry.keys.find(
            (key) => key.credentialId === credentialId,
          )!;
          if (action === "select_primary" && (key.paused || key.removed))
            return { ok: false, error: { code: "AI_KEY_PAUSED" } };
          registry = {
            ...registry,
            primaryCredentialId:
              action === "select_primary"
                ? credentialId
                : (action === "pause" || action === "remove") &&
                    registry.primaryCredentialId === credentialId
                  ? null
                  : registry.primaryCredentialId,
            keys: registry.keys.map((key) =>
              key.credentialId === credentialId
                ? {
                    ...key,
                    paused:
                      action === "pause" || action === "remove"
                        ? true
                        : action === "unpause"
                          ? false
                          : key.paused,
                    removed: action === "remove" ? true : key.removed,
                  }
                : key,
            ),
          };
          if (action === "remove")
            caps = caps.filter((item) => item.credentialId !== credentialId);
          value = registry;
          break;
        }
        case "preview_ai_test":
          value = {
            credentialId: args.credentialId,
            provider: registry.keys.find(
              (key) => key.credentialId === args.credentialId,
            )?.provider,
            model: "Sample model",
            currency: "USD",
            estimatedInputTokens: 512,
            maximumCostMicros: 20000,
          };
          break;
        case "test_ai_connection":
          value = {
            confirmed: true,
            effectiveModel: "Sample model",
            usageComplete: true,
            estimatedCostMicros: 2400,
            usage: {
              inputTokens: 512,
              outputTokens: 64,
              cachedInputTokens: 0,
              cacheWriteTokens: 0,
              reasoningTokens: 0,
            },
          };
          break;
        case "save_ai_cap":
          value = {
            ...cap(
              args.request.period,
              args.request.limitMicros,
              args.request.credentialId,
            ),
            countedMicros:
              caps.find(
                (item) => item.credentialId === args.request.credentialId,
              )?.countedMicros ??
              lifetimeRecords
                .filter(
                  (item) => item.credentialId === args.request.credentialId,
                )
                .reduce((sum, item) => sum + item.costByCurrencyMicros.USD, 0),
          };
          caps = [
            ...caps.filter(
              (item) =>
                item.period !== args.request.period ||
                item.credentialId !== args.request.credentialId,
            ),
            value as Record<string, any>,
          ];
          break;
        case "save_ai_general_cap":
          value = {
            ...cap("all_time", args.limitMicros, "general"),
            countedMicros:
              caps.find((item) => item.credentialId === "general")
                ?.countedMicros ??
              lifetimeRecords.reduce(
                (sum, item) => sum + item.costByCurrencyMicros.USD,
                0,
              ),
          };
          caps = [
            ...caps.filter((item) => item.credentialId !== "general"),
            value as Record<string, any>,
          ];
          break;
        case "disable_ai_cap":
          caps = caps.filter(
            (item) =>
              item.period !== args.request.period ||
              item.credentialId !== args.request.credentialId,
          );
          value = true;
          break;
        case "disable_ai_general_cap":
          caps = caps.filter((item) => item.credentialId !== "general");
          value = true;
          break;
        case "reset_ai_cap":
          caps = caps.map((item) =>
            item.period === args.request.period &&
            item.credentialId === args.request.credentialId
              ? { ...item, countedMicros: 0 }
              : item,
          );
          value = true;
          break;
        case "reset_ai_general_cap":
          caps = caps.map((item) =>
            item.credentialId === "general"
              ? { ...item, countedMicros: 0 }
              : item,
          );
          value = true;
          break;
        case "save_ai_retention": {
          retention = args.policy;
          const retentionDays =
            retention === "30_days"
              ? 30
              : retention === "90_days"
                ? 90
                : retention === "one_year"
                  ? 365
                  : null;
          const before = records.length;
          if (retentionDays !== null) {
            const cutoff = now.getTime() - retentionDays * 24 * 60 * 60 * 1_000;
            records = records.filter((record) => record.at >= cutoff);
          }
          value = {
            policy: retention,
            removedOperations: before - records.length,
          };
          break;
        }
        case "clear_ai_monitoring": {
          const before = records.length;
          records = records.filter(
            (record) =>
              !args.months.some(
                (month: { fromUnixMs: number; toUnixMs: number }) =>
                  record.at >= month.fromUnixMs &&
                  record.at < month.toUnixMs &&
                  (!args.credentialId ||
                    record.credentialId === args.credentialId),
              ),
          );
          value = before - records.length;
          break;
        }
        case "export_ai_monitoring":
          value = "cancelled";
          break;
        default:
          return { ok: false, error: { code: "PREVIEW_ONLY" } };
      }
      return { ok: true, value };
    },
  },
});

function Preview() {
  const [mode, setMode] = useState<Scenario>("connected");
  const [revision, setRevision] = useState(0);
  return (
    <>
      <aside
        style={{
          padding: "10px 24px",
          borderBottom: "1px solid #cbd5e1",
          background: "#eef2f7",
          display: "flex",
          alignItems: "center",
          gap: 12,
          flexWrap: "wrap",
          fontSize: 12,
        }}
      >
        <span>
          UI preview · Sample data & pricing. No live API calls. Do not enter a
          real key.
        </span>
        <label style={{ display: "flex", alignItems: "center", gap: 8 }}>
          View
          <select
            aria-label="Preview scenario"
            value={mode}
            onChange={(event) => {
              const next = event.target.value as typeof mode;
              reset(next);
              setMode(next);
              setRevision((current) => current + 1);
            }}
          >
            <option value="connected">Connected</option>
            <option value="no_primary">No primary key</option>
            <option value="setup">Key setup</option>
            <option value="partial">Partial usage</option>
          </select>
        </label>
      </aside>
      <AppShell
        destination="ai"
        navigationBlocked={true}
        onNavigate={() => {}}
        status={<span className="ai-badge">Web preview</span>}
      >
        <AiWorkspace key={revision} blocked={false} />
      </AppShell>
    </>
  );
}
createRoot(document.getElementById("root")!).render(<Preview />);
