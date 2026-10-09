// Development-only preview. No native commands, provider calls or persistent writes.
import type * as Wire from "@ort/contracts/wire";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import { AiWorkspace } from "../src/shared/AiWorkspace";
import { AppShell } from "../src/shared/AppShell";
import { type Usage, type UsageBucket } from "../src/shared/AiUsageChart";
import "../src/shared/styles/index.css";

type Provider = "openai" | "anthropic" | "gemini";
type SavedKey = {
  credentialId: string;
  createdAt: string;
  name?: string | null;
  provider: Provider;
  model: string;
  paused: boolean;
  pauseReason?: string;
  removed: boolean;
  cleanupRequired: boolean;
};
type Registry = {
  keys: SavedKey[];
  primaryCredentialId: string | null;
  nextIdentificationNumber: number;
};
const planParameters = new URLSearchParams(location.search);
const codexEnabled =
  !planParameters.has("disabled") &&
  ["codex", "disconnected", "runtime-missing"].some((flag) =>
    planParameters.has(flag),
  );
const codexConnected =
  codexEnabled &&
  !planParameters.has("disconnected") &&
  !planParameters.has("runtime-missing");
const plan: Wire.PlanStatus = {
  settings: {
    cleanupRequired: false,
    connectionId: codexConnected ? fixturePlanId() : null,
    enabled: codexEnabled,
    model: "gpt-6.1-sol",
    reasoning: "medium",
    reserveEnabled: true,
    reservePercent: 20,
  },
  revision: 1,
  connected: codexConnected,
  accountPlan: codexConnected ? "plus" : null,
  loginPending: false,
  operationActive: false,
  runtimeVersion: codexEnabled ? "0.162.0" : null,
  errorCode: null,
  models: [
    "gpt-5.6-luna",
    "gpt-5.6-terra",
    "gpt-5.6-sol",
    "gpt-6-luna",
    "gpt-6-sol",
    "gpt-6.1-sol",
  ].map((id) => ({
    id,
    name: `GPT-${id
      .slice(4)
      .replace("-", " ")
      .replace(/(luna|terra|sol)$/, (s) => s[0].toUpperCase() + s.slice(1))}`,
    supported: id !== "gpt-5.6-terra",
    explanation:
      id === "gpt-5.6-terra"
        ? "This installed runtime does not offer this model."
        : null,
    reasoningEfforts: ["low", "medium", "high", "xhigh"],
  })),
  quota: {
    fetchedAtUnixMs: Date.now(),
    windows: [
      {
        limitId: "codex",
        name: "Codex primary",
        window: "primary",
        remainingPercent: 64,
        windowDurationMinutes: 300,
        resetsAt: Math.floor(Date.now() / 1000) + 5400,
      },
      {
        limitId: "codex",
        name: "Codex secondary",
        window: "secondary",
        remainingPercent: 38,
        windowDurationMinutes: 10080,
        resetsAt: Math.floor(Date.now() / 1000) + 144000,
      },
    ],
  },
};
const installation: Wire.RuntimeInstallStatus = {
  phase: "idle",
  downloadedBytes: 0,
  totalBytes: 98089521,
  errorCode: null,
};
const readiness: Wire.RuntimeReadiness = {
  ready: !planParameters.has("runtime-missing"),
  errorCode: planParameters.has("runtime-missing")
    ? "PLAN_RUNTIME_MISSING"
    : null,
};
if (new URLSearchParams(location.search).has("runtime-missing")) {
  plan.connected = false;
  plan.settings.connectionId = null;
  plan.runtimeVersion = null;
  plan.errorCode = "PLAN_RUNTIME_MISSING";
}
function fixturePlanId() {
  return "019a0000-0000-7000-8000-000000000050";
}
const models = {
  openai: "gpt-5.6-terra",
  anthropic: "claude-sonnet-5",
  gemini: "gemini-3.6-flash",
};
const now = new Date();
const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
const dateLabel = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
const fixtureKey = (number: number) =>
  `019a0000-0000-7000-8000-${String(number).padStart(12, "0")}`;
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
      credentialId:
        index % 4 === 0 ? fixturePlanId() : fixtureKey((index % 3) + 1),
      label: dateLabel(date),
      attempts: index % 6 === 0 ? 0 : 2,
      usage: {
        inputTokens: input,
        outputTokens: 540 + (index % 400),
        cachedInputTokens: 320,
        cacheWriteTokens: 80,
        reasoningTokens: 120,
      },
      costByCurrencyMicros: {
        USD: index % 4 === 0 ? 0 : input * 2 + (540 + (index % 400)) * 8,
      },
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
              credentialId: fixtureKey(index + 1),
              createdAt: new Date(
                now.getTime() - (3 - index) * 86400000,
              ).toISOString(),
              provider,
              model: models[provider],
              paused: index === 2,
              removed: false,
              cleanupRequired: false,
            }),
          ),
    primaryCredentialId:
      mode === "setup" || mode === "no_primary" ? null : fixtureKey(1),
    nextIdentificationNumber: mode === "setup" ? 1 : 4,
  };
  records = mode === "setup" ? [] : seed();
  lifetimeRecords = records;
  caps =
    mode === "setup"
      ? []
      : [
          cap("all_time", 10000000, fixtureKey(1)),
          cap("all_time", 5000000, fixtureKey(2)),
        ];
  partial = mode === "partial";
  retention = "retain_until_cleared";
}
reset("connected");
const catalog = {
  formatVersion: 1,
  minimumAppVersion: "0.0.0-dev",
  issuedAt: "2026-09-17T00:00:00Z",
  catalogId: "sample-ui-data",
  expiresAt: "2027-01-01",
  entries: (["openai", "anthropic", "gemini"] as const).map((provider) => ({
    provider: provider === "openai" ? "open_ai" : provider,
    model: models[provider],
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
      (!args.credentialId || record.credentialId === args.credentialId) &&
      (!args.connectionSource || record.credentialId === fixturePlanId()),
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
    connectionSources: Object.fromEntries(
      [...new Set(records.map((r) => r.credentialId))].map((id) => [
        id,
        id === fixturePlanId() ? "chatgpt_plan" : "direct_api",
      ]),
    ),
    planAttempts:
      selected.filter((r) => r.credentialId === fixturePlanId()).length * 2,
    logicalOperations: selected.length,
    attempts: selected.length * 2,
    usage,
    costByCurrencyMicros: selected.length ? { USD: cost } : {},
    totalTokens:
      usage.inputTokens +
      usage.cachedInputTokens +
      usage.cacheWriteTokens +
      usage.outputTokens +
      usage.reasoningTokens,
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
    timeBuckets: [...buckets.values()].map((bucket) => ({
      ...bucket,
      totalTokens:
        bucket.usage.inputTokens +
        bucket.usage.cachedInputTokens +
        bucket.usage.cacheWriteTokens +
        bucket.usage.outputTokens +
        bucket.usage.reasoningTokens,
    })),
    recentFailures:
      partial && selected.length
        ? [
            {
              operationId: "preview-operation-1",
              attemptId: "preview-attempt-2",
              provider: "gemini",
              requestedModel: "gemini-3.5-flash-lite",
              effectiveModel: null,
              operationType: "tailor_resume",
              callNumber: 2,
              connectionSource: "direct_api",
              reasoning: null,
              monetaryCostTracking: "estimated",
              reportedRetries: 0,
              startedAtUnixMs: now.getTime() - 120000,
              durationMs: 2350,
              category: "transient",
              usage: null,
              usageComplete: false,
              details: {
                code: "AI_PROVIDER_SERVICE_UNAVAILABLE",
                httpStatus: 503,
                finishReason: null,
                providerReason: null,
                pageCount: null,
                validationIssues: [],
              },
            },
            {
              operationId: "preview-operation-2",
              attemptId: "preview-attempt-4",
              provider: "gemini",
              requestedModel: "gemini-3.5-flash-lite",
              effectiveModel: "gemini-3.5-flash-lite",
              operationType: "tailor_resume",
              callNumber: 4,
              connectionSource: "direct_api",
              reasoning: null,
              monetaryCostTracking: "estimated",
              reportedRetries: 0,
              startedAtUnixMs: now.getTime() - 240000,
              durationMs: 15420,
              category: "invalid_output",
              usage: {
                inputTokens: 4500,
                outputTokens: 2200,
                reasoningTokens: 100,
                cachedInputTokens: 0,
                cacheWriteTokens: 0,
              },
              usageComplete: true,
              details: {
                code: "AI_MATERIAL_INVALID",
                httpStatus: null,
                finishReason: null,
                providerReason: null,
                pageCount: null,
                validationIssues: [
                  "Missing required schema v6 field: reviewIssues. Return the complete candidate, including empty arrays where appropriate.",
                ],
              },
            },
          ]
        : [],
  };
}

Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
  value: { unregisterListener: () => {} },
});

// Credentials are discarded, never logged or stored. All controls use memory-only fixtures.
Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    transformCallback: () => 1,
    unregisterCallback: () => {},
    invoke: async (command: string, args: Record<string, any> = {}) => {
      let value: unknown;
      switch (command) {
        case "check_codex_runtime":
          value = { ...readiness };
          break;
        case "load_codex_runtime_install":
          value = { ...installation };
          break;
        case "install_codex_runtime":
          installation.phase = "downloading";
          installation.downloadedBytes = 48000000;
          await new Promise((resolve) => setTimeout(resolve, 2000));
          installation.phase = "awaiting_approval";
          installation.downloadedBytes = installation.totalBytes;
          await new Promise((resolve) => setTimeout(resolve, 2000));
          installation.phase = "failed";
          installation.errorCode = "PLAN_INSTALL_PERMISSION_DENIED";
          value = { ...installation };
          break;
        case "cancel_codex_runtime_install":
          installation.phase = "cancelled";
          installation.errorCode = "PLAN_INSTALL_CANCELLED";
          value = { ...installation };
          break;
        case "plugin:event|listen":
          return 1;
        case "plugin:event|unlisten":
          return null;
        case "load_chatgpt_plan":
          value = plan;
          break;
        case "save_chatgpt_plan": {
          const { expectedRevision: _, ...settings } = args.request;
          plan.settings = { ...plan.settings, ...settings };
          plan.revision = (plan.revision ?? 0) + 1;
          if (!plan.settings.enabled) {
            plan.connected = false;
            plan.settings.connectionId = null;
            plan.runtimeVersion = null;
            plan.quota = null;
            plan.accountPlan = null;
          } else if (!plan.runtimeVersion) plan.runtimeVersion = "0.162.0";
          value = plan;
          break;
        }
        case "connect_chatgpt_plan":
          plan.connected = true;
          plan.accountPlan = "plus";
          plan.settings.connectionId = fixturePlanId();
          value = plan;
          break;
        case "disconnect_chatgpt_plan":
          plan.connected = false;
          plan.settings.connectionId = null;
          plan.runtimeVersion = null;
          plan.quota = null;
          plan.accountPlan = null;
          value = plan;
          break;
        case "cancel_chatgpt_login":
          plan.loginPending = false;
          value = plan;
          break;
        case "load_ai_connection":
          value = {
            keys: registry.keys,
            primaryCredentialId: registry.primaryCredentialId,
          };
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
        case "set_ai_key_model":
          registry = {
            ...registry,
            keys: registry.keys.map((key) =>
              key.credentialId === args.request.credentialId
                ? { ...key, model: args.request.model }
                : key,
            ),
          };
          value = {
            keys: registry.keys,
            primaryCredentialId: registry.primaryCredentialId,
          };
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
                credentialId: fixtureKey(number),
                createdAt: new Date().toISOString(),
                provider: args.request.provider,
                model: models[args.request.provider as Provider],
                paused: false,
                removed: false,
                cleanupRequired: false,
              },
            ],
          };
          value = {
            keys: registry.keys,
            primaryCredentialId: registry.primaryCredentialId,
          };
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
          value = {
            keys: registry.keys,
            primaryCredentialId: registry.primaryCredentialId,
          };
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
          value = {
            keys: registry.keys,
            primaryCredentialId: registry.primaryCredentialId,
          };
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
    <div className="preview-root">
      <aside
        className="preview-toolbar"
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
        onOpenApplication={() => {}}
        overlayVisible={false}
        onNavigate={() => {}}
        status={<span className="ai-badge">Web preview</span>}
      >
        <AiWorkspace key={revision} blocked={false} />
      </AppShell>
    </div>
  );
}
createRoot(document.getElementById("root")!).render(<Preview />);
