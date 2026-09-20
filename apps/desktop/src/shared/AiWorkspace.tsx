import { Channel, invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import anthropicLogo from "../assets/providers/anthropic.svg";
import geminiLogo from "../assets/providers/gemini.png";
import openAiLogo from "../assets/providers/openai.svg";
import { AiKeyName } from "./AiKeyName";
import { AiKeyMenu } from "./AiKeyMenu";
import { AiGeneralSpending } from "./AiGeneralSpending";
import { AiDataKeyPicker, dataKeyDescription } from "./AiDataKeyPicker";
import {
  AiDataActionDialog,
  type ActivityMonth,
  type ActivityPeriod,
  type DataAction,
} from "./AiDataActionDialog";
import { AiKeyCustomization } from "./AiKeyCustomization";
import { AiUsageChart, totalTokens, type Usage } from "./AiUsageChart";

export type SavedKey = {
  credentialId: string;
  identificationNumber: number;
  name?: string | null;
  provider: "openai" | "anthropic" | "gemini";
  preset: "economy" | "balanced" | "quality";
  paused: boolean;
  removed: boolean;
  cleanupRequired: boolean;
};
export type KeyRegistry = {
  keys: SavedKey[];
  primaryCredentialId: string | null;
  nextIdentificationNumber: number;
};
type Response =
  | { ok: true; value: KeyRegistry }
  | { ok: false; error: { code: string } };
const providerName = (provider: SavedKey["provider"]) =>
  provider === "openai"
    ? "OpenAI"
    : provider === "anthropic"
      ? "Anthropic"
      : "Gemini";
const keyName = (key: SavedKey) =>
  key.name || `Key #${key.identificationNumber}`;
const providerLogos: Record<SavedKey["provider"], string> = {
  openai: openAiLogo,
  anthropic: anthropicLogo,
  gemini: geminiLogo,
};
function ProviderMark({ provider }: { provider: SavedKey["provider"] }) {
  return <img src={providerLogos[provider]} alt="" aria-hidden="true" />;
}
type Monitoring = {
  logicalOperations: number;
  attempts: number;
  usage: Usage;
  estimatedCostMicros: number;
  unresolvedReservedMicros: number;
  currency: string | null;
  partial: boolean;
  unknownCount: number;
  byProvider: Record<string, number>;
  byCredentialId?: Record<string, number>;
  byStatus: Record<string, number>;
  byModel: Record<string, number>;
  byPreset: Record<string, number>;
  byOperationType: Record<string, number>;
  costByCurrencyMicros: Record<string, number>;
  timeBuckets: Array<{
    label: string;
    attempts: number;
    usage: {
      inputTokens: number;
      cachedInputTokens?: number;
      cacheWriteTokens?: number;
      outputTokens: number;
      reasoningTokens?: number;
    };
    costByCurrencyMicros: Record<string, number>;
    partial: boolean;
    unknownCount: number;
  }>;
};
type MonitoringResponse =
  | { ok: true; value: Monitoring }
  | { ok: false; error: { code: string } };
type TestPreview = {
  credentialId: string;
  provider: string;
  model: string;
  currency: string;
  estimatedInputTokens: number;
  maximumCostMicros: number;
};
type TestResult = {
  confirmed: boolean;
  effectiveModel: string;
  usageComplete: boolean;
  estimatedCostMicros: number | null;
  usage: {
    inputTokens: number;
    cachedInputTokens: number;
    cacheWriteTokens: number;
    outputTokens: number;
    reasoningTokens: number;
  };
};
type RetentionPolicy =
  | "30_days"
  | "90_days"
  | "one_year"
  | "retain_until_cleared";
type CatalogEntry = {
  provider: "open_ai" | "anthropic" | "gemini";
  model: string;
  preset: "economy" | "balanced" | "quality";
  operations: string[];
  maxInputTokens: number;
  maxOutputTokens: number;
  currency: string;
  prices: Array<{ category: string; microsPerMillion: number }>;
  source: string;
  verifiedAt: string;
  effectiveFrom: string;
  effectiveTo: string | null;
  disabled: boolean;
};
export type Catalog = {
  catalogId: string;
  expiresAt: string;
  entries: CatalogEntry[];
};
function periodBounds(period: ActivityPeriod) {
  const now = new Date();
  const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  if (period === "Week")
    start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
  if (period === "Month") start.setDate(1);
  if (period === "Year") {
    start.setMonth(0);
    start.setDate(1);
  }
  return {
    fromUnixMs: period === "All time" ? 0 : start.getTime(),
    toUnixMs: now.getTime() + 1,
  };
}

function monitoringArgs(period: ActivityPeriod) {
  return {
    ...periodBounds(period),
    timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    bucketSize: period === "Week" || period === "Month" ? "day" : "month",
  };
}

function testFailureMessage(code: string) {
  const messages: Record<string, string> = {
    AI_CANCELLED:
      "Synthetic request cancelled. Usage may remain unknown after dispatch.",
    AI_CAP_REJECTED:
      "The configured spending cap rejected this request before dispatch.",
    AI_RETRY_BLOCKED:
      "The first provider attempt failed transiently; a second attempt was blocked by the local guardrail or storage policy. Review both activity and unresolved exposure in Monitoring.",
    AI_CONFIRMATION_STALE: "The model or estimate changed; review it again.",
    AI_AUTHENTICATION_FAILED:
      "The provider rejected this credential. Replace the key and run the test again.",
    AI_RATE_LIMITED:
      "The provider rate-limited this credential. Review its quota and try later.",
    AI_OUTPUT_INVALID:
      "The provider returned malformed or incompatible output; nothing was accepted.",
    AI_USAGE_UNKNOWN:
      "The response could not be priced because usage was missing; the reservation remains unresolved.",
    AI_PROVIDER_UNAVAILABLE:
      "The provider connection failed or timed out. Monitoring preserves any uncertain exposure.",
  };
  return (
    messages[code] ??
    "Synthetic provider request failed. Check the connection and Monitoring for any unknown usage."
  );
}

export function AiWorkspace({ blocked }: { blocked: boolean }) {
  const [page, setPage] = useState<"general" | "data">("general");
  const [registry, setRegistry] = useState<KeyRegistry | null>(null);
  const [keysUnavailable, setKeysUnavailable] = useState(false);
  const primaryKey =
    registry?.keys.find(
      (key) =>
        key.credentialId === registry.primaryCredentialId &&
        !key.paused &&
        !key.removed &&
        !key.cleanupRequired,
    ) ?? null;
  const [addOpen, setAddOpen] = useState(false);
  const [keyFilter, setKeyFilter] = useState("");
  const [dataAction, setDataAction] = useState<DataAction | null>(null);
  const [testTarget, setTestTarget] = useState<SavedKey | null>(null);
  const [provider, setProvider] = useState<
    "" | "openai" | "anthropic" | "gemini"
  >("");
  const [key, setKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [working, setWorking] = useState(false);
  const [notice, setNotice] = useState("");
  const [removeConfirm, setRemoveConfirm] = useState<string | null>(null);
  const [period, setPeriod] = useState<ActivityPeriod>("Month");
  const [monitoring, setMonitoring] = useState<Monitoring | null>(null);
  const [monitoringError, setMonitoringError] = useState(false);
  const [monitoringRevision, setMonitoringRevision] = useState(0);
  const [metric, setMetric] = useState<"cost" | "tokens">("cost");
  const [currency, setCurrency] = useState("USD");
  const [testPreview, setTestPreview] = useState<TestPreview | null>(null);
  const [testOutput, setTestOutput] = useState("");
  const [testActive, setTestActive] = useState(false);
  const [retention, setRetention] = useState<RetentionPolicy>(
    "retain_until_cleared",
  );
  const [retentionConfirm, setRetentionConfirm] = useState(false);
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [selectingPrimary, setSelectingPrimary] = useState(false);
  const [primaryCandidate, setPrimaryCandidate] = useState<string | null>(null);

  useEffect(() => {
    void invoke<{ ok: true; value: Catalog } | { ok: false }>("load_ai_catalog")
      .then((response) => {
        if (response.ok) setCatalog(response.value);
        else setCatalog(null);
      })
      .catch(() => setCatalog(null));
  }, []);

  function applyRegistry(value: KeyRegistry) {
    setRegistry(value);
    setKeysUnavailable(false);
    setTestPreview(null);
    setTestTarget(null);
    setTestOutput("");
  }
  async function refreshKeys() {
    try {
      const response = await invoke<Response>("load_ai_connection");
      if (response.ok) applyRegistry(response.value);
      else {
        setRegistry(null);
        setKeysUnavailable(true);
        setNotice(
          "Saved keys are unavailable. Reload before sending a request.",
        );
      }
    } catch {
      setRegistry(null);
      setKeysUnavailable(true);
      setNotice("Saved keys are unavailable. Reload before sending a request.");
    }
  }
  useEffect(() => {
    void refreshKeys();
  }, []);

  useEffect(() => {
    let current = true;
    setMonitoring(null);
    setMonitoringError(false);
    void invoke<MonitoringResponse>("load_ai_monitoring", {
      ...monitoringArgs(period),
      credentialId: keyFilter || null,
    })
      .then((response) => {
        if (!current) return;
        if (response.ok) setMonitoring(response.value);
        else setMonitoringError(true);
      })
      .catch(() => {
        if (current) setMonitoringError(true);
      });
    return () => {
      current = false;
    };
  }, [period, keyFilter, monitoringRevision]);

  useEffect(() => {
    void invoke<
      | {
          ok: true;
          value: { policy: RetentionPolicy; removedOperations: number };
        }
      | { ok: false }
    >("load_ai_retention")
      .then((response) => {
        if (response.ok) {
          setRetention(response.value.policy);
          if (response.value.removedOperations > 0)
            setMonitoringRevision((value) => value + 1);
        }
      })
      .catch(() => {});
  }, []);

  async function clearMonitoring(
    targetKey: string,
    selectedMonths: ActivityMonth[],
  ) {
    if (blocked || working) return;
    setDataAction(null);
    setWorking(true);
    try {
      const response = await invoke<
        { ok: true; value: number } | { ok: false; error: { code: string } }
      >("clear_ai_monitoring", {
        months: selectedMonths.map(({ label, fromUnixMs, toUnixMs }) => ({
          label,
          fromUnixMs,
          toUnixMs,
        })),
        credentialId: targetKey || null,
      });
      if (response.ok) {
        setNotice(
          `${response.value} completed AI operations cleared. Lifetime spend and spending-cap progress were not changed.`,
        );
        setMonitoringRevision((value) => value + 1);
      } else
        setNotice(
          response.error.code === "AI_BUSY"
            ? "Finish or cancel the active AI request before clearing."
            : "AI activity could not be cleared.",
        );
    } catch {
      setNotice("AI activity could not be cleared.");
    } finally {
      setWorking(false);
    }
  }

  async function saveRetention() {
    if (working) return;
    setWorking(true);
    try {
      const response = await invoke<
        | {
            ok: true;
            value: { policy: RetentionPolicy; removedOperations: number };
          }
        | { ok: false }
      >("save_ai_retention", { policy: retention });
      if (response.ok) {
        setNotice(
          response.value.removedOperations > 0
            ? `Retention saved; ${response.value.removedOperations} older operations were cleared. Spending caps were not reset.`
            : "AI activity retention saved. Spending caps were not reset.",
        );
        setMonitoringRevision((value) => value + 1);
      } else setNotice("AI activity retention could not be saved.");
    } catch {
      setNotice("AI activity retention could not be saved.");
    } finally {
      setWorking(false);
    }
  }

  async function reviewTest(target: SavedKey) {
    if (blocked || working || target.cleanupRequired) return;
    setTestTarget(target);
    setTestOutput("");
    setWorking(true);
    setNotice("");
    setTestPreview(null);
    try {
      const response = await invoke<
        | { ok: true; value: TestPreview }
        | { ok: false; error: { code: string } }
      >("preview_ai_test", { credentialId: target.credentialId });
      if (response.ok) setTestPreview(response.value);
      else
        setNotice(
          response.error.code === "AI_PRESET_UNAVAILABLE"
            ? "This provider and preset have no enabled signed-catalog entry."
            : "The synthetic test estimate is unavailable.",
        );
    } catch {
      setNotice("The synthetic test estimate is unavailable.");
    } finally {
      setWorking(false);
    }
  }

  async function confirmTest() {
    if (!testPreview || blocked || working) return;
    const preview = testPreview;
    setTestPreview(null);
    setTestOutput("");
    setWorking(true);
    setTestActive(true);
    setNotice("Synthetic request in progress…");
    const progress = new Channel<{ kind: string; text: string }>();
    progress.onmessage = (event) => {
      if (event.kind === "delta")
        setTestOutput((current) => (current + event.text).slice(0, 8192));
      if (event.kind === "retry")
        setNotice(
          "The provider returned a transient server error. Retrying once under the same operation; both attempts remain in Monitoring.",
        );
    };
    try {
      const response = await invoke<
        { ok: true; value: TestResult } | { ok: false; error: { code: string } }
      >("test_ai_connection", {
        onProgress: progress,
        credentialId: preview.credentialId,
        expectedModel: preview.model,
        expectedMaximumCostMicros: preview.maximumCostMicros,
      });
      if (response.ok) {
        const usage = response.value.usage;
        const usageText = `${usage.inputTokens} input, ${usage.cachedInputTokens} cached input, ${usage.cacheWriteTokens} cache-write, ${usage.outputTokens} output, and ${usage.reasoningTokens} reasoning tokens reported`;
        setNotice(
          response.value.usageComplete &&
            response.value.estimatedCostMicros !== null
            ? `Synthetic request completed with ${usageText}. Recorded estimate: ${(response.value.estimatedCostMicros / 1_000_000).toFixed(4)} ${preview.currency}.`
            : `Synthetic request completed with ${usageText}, but incomplete cost data; reserved exposure remains counted.`,
        );
        setMonitoringRevision((value) => value + 1);
      } else {
        setNotice(testFailureMessage(response.error.code));
        setMonitoringRevision((value) => value + 1);
      }
    } catch {
      setNotice("The synthetic provider request is unavailable.");
    } finally {
      setWorking(false);
      setTestActive(false);
    }
  }

  async function cancelTest() {
    try {
      await invoke("cancel_ai_test");
    } catch {
      setNotice("Cancellation could not be requested.");
    }
  }

  async function exportMonitoring(
    targetKey: string,
    selectedMonths: ActivityMonth[],
  ) {
    if (blocked || working) return;
    setDataAction(null);
    setWorking(true);
    try {
      const response = await invoke<
        { ok: true; value: string } | { ok: false; error: { code: string } }
      >("export_ai_monitoring", {
        months: selectedMonths.map(({ label, fromUnixMs, toUnixMs }) => ({
          label,
          fromUnixMs,
          toUnixMs,
        })),
        timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        credentialId: targetKey || null,
      });
      if (response.ok && response.value === "exported")
        setNotice(
          "AI aggregate JSON exported. The file is unencrypted; keep it in a private location.",
        );
      else if (!response.ok)
        setNotice(
          "AI aggregate JSON could not be exported; choose a new filename.",
        );
    } catch {
      setNotice("AI aggregate JSON could not be exported.");
    } finally {
      setWorking(false);
    }
  }

  async function addKey(event: FormEvent) {
    event.preventDefault();
    if (blocked || working || !registry || !key.trim() || !provider) return;
    setWorking(true);
    setNotice("");
    try {
      const response = await invoke<Response>("add_ai_key", {
        request: { provider, apiKey: key },
      });
      setKey("");
      if (response.ok) {
        applyRegistry(response.value);
        setAddOpen(false);
        setProvider("");
        setShowKey(false);
        setNotice("Key saved securely. Use Set primary key to select it.");
      } else {
        await refreshKeys();
        setNotice(
          response.error.code === "AI_CREDENTIAL_CLEANUP_REQUIRED"
            ? "Key setup is incomplete. Remove the key marked Cleanup required, then try again."
            : "The key could not be saved. Check your provider and vault.",
        );
      }
    } catch {
      setKey("");
      setShowKey(false);
      await refreshKeys();
      setNotice("The key could not be saved.");
    } finally {
      setWorking(false);
    }
  }

  async function changeKey(
    target: SavedKey,
    action: "select_primary" | "pause" | "unpause" | "remove",
  ) {
    if (blocked || working) return;
    setWorking(true);
    setRemoveConfirm(null);
    setNotice("");
    try {
      const response = await invoke<Response>("change_ai_key", {
        request: { credentialId: target.credentialId, action },
      });
      if (response.ok) {
        applyRegistry(response.value);
        setNotice(
          action === "select_primary"
            ? `Key #${target.identificationNumber} is now primary.`
            : action === "remove"
              ? `Key #${target.identificationNumber} removed. Its activity history remains.`
              : action === "pause"
                ? `Key #${target.identificationNumber} paused.`
                : `Key #${target.identificationNumber} unpaused. Use Set primary key to select it.`,
        );
        setMonitoringRevision((value) => value + 1);
        return true;
      } else {
        await refreshKeys();
        setNotice(
          response.error.code === "AI_BUSY"
            ? "Finish or cancel the active test first."
            : response.error.code === "AI_CREDENTIAL_CLEANUP_REQUIRED"
              ? "Key cleanup could not finish. The key is paused and cannot be primary. Retry removal."
              : "The key could not be updated. Check the credential vault and try again.",
        );
      }
    } catch {
      await refreshKeys();
      setNotice("The key could not be updated. Reload before retrying.");
    } finally {
      setWorking(false);
    }
  }

  useEffect(() => {
    if (
      primaryCandidate &&
      !registry?.keys.some(
        (key) =>
          key.credentialId === primaryCandidate &&
          !key.paused &&
          !key.removed &&
          !key.cleanupRequired,
      )
    )
      setPrimaryCandidate(null);
  }, [registry, primaryCandidate]);
  async function confirmPrimary() {
    if (!primaryCandidate) {
      if (blocked || working) return;
      setWorking(true);
      setNotice("");
      try {
        const response = await invoke<Response>("clear_ai_primary");
        if (response.ok) {
          applyRegistry(response.value);
          setNotice("No primary key is selected.");
          setSelectingPrimary(false);
          setMonitoringRevision((value) => value + 1);
        } else {
          await refreshKeys();
          setNotice("The primary key could not be cleared. Try again.");
        }
      } catch {
        await refreshKeys();
        setNotice(
          "The primary key could not be cleared. Reload before retrying.",
        );
      } finally {
        setWorking(false);
      }
      return;
    }
    const target = registry?.keys.find(
      (key) =>
        key.credentialId === primaryCandidate &&
        !key.paused &&
        !key.removed &&
        !key.cleanupRequired,
    );
    if (!target || blocked || working) return;
    if (await changeKey(target, "select_primary")) {
      setSelectingPrimary(false);
      setPrimaryCandidate(null);
    }
  }
  function savedName(id: string, name: string | null) {
    setRegistry((current) =>
      current
        ? {
            ...current,
            keys: current.keys.map((key) =>
              key.credentialId === id ? { ...key, name } : key,
            ),
          }
        : current,
    );
  }

  const visibleKeys = registry?.keys.filter((key) => !key.removed) ?? [];
  const activityKeys = [...(registry?.keys ?? [])];
  for (const id of Object.keys(monitoring?.byCredentialId ?? {})) {
    if (!activityKeys.some((key) => key.credentialId === id))
      activityKeys.push({
        credentialId: id,
        identificationNumber: 0,
        provider: "openai",
        preset: "balanced",
        paused: true,
        removed: true,
        cleanupRequired: false,
      });
  }
  const currencies = Object.keys(monitoring?.costByCurrencyMicros ?? {});
  const chartCurrency = currencies.includes(currency)
    ? currency
    : (currencies[0] ?? "USD");
  const selectedActivityKey = activityKeys.find(
    (saved) => saved.credentialId === keyFilter,
  );
  const activityView = dataKeyDescription(selectedActivityKey, catalog);

  return (
    <section className="ai-workspace" aria-label="AI settings">
      <nav className="workflow-steps" aria-label="AI section">
        {(["general", "data"] as const).map((tab) => (
          <button
            key={tab}
            type="button"
            className="button--secondary"
            aria-current={page === tab ? "page" : undefined}
            onClick={() => setPage(tab)}
          >
            {tab === "general" ? "My Keys" : "Data"}
          </button>
        ))}
      </nav>
      <div className="workspace-data ai-page-content">
        {notice && (
          <p className="notice ai-notice" role="status">
            {notice}
          </p>
        )}
        <div
          className="ai-page-panels"
          hidden={page !== "general"}
          aria-label="My Keys settings"
        >
          <section className="ai-panel" aria-labelledby="ai-connection-title">
            <div className="ai-panel-heading">
              <div>
                <h3 id="ai-connection-title">API keys</h3>
                <p className="ai-help">
                  Keys stay in your operating-system vault. Only the primary key
                  is used for AI.
                </p>
              </div>
              <button
                type="button"
                className="ai-add-key-trigger"
                disabled={blocked || working || !registry}
                aria-expanded={addOpen}
                aria-label="Add key"
                title="Add key"
                onClick={() => {
                  setKey("");
                  setProvider("");
                  setShowKey(false);
                  setAddOpen(true);
                }}
              >
                <span aria-hidden="true">+</span>
              </button>
            </div>
            <div
              className={
                primaryKey ? "ai-primary ai-primary--selected" : "ai-primary"
              }
              role="status"
            >
              <span
                className={
                  primaryKey ? "ai-badge ai-badge--active" : "ai-badge"
                }
              >
                {keysUnavailable
                  ? "Primary key unavailable"
                  : !registry
                    ? "Loading keys…"
                    : primaryKey
                      ? "Primary"
                      : "No primary key selected"}
              </span>
              <span>
                {keysUnavailable
                  ? "Reload to check your keys before using AI."
                  : !registry
                    ? ""
                    : primaryKey
                      ? `${keyName(primaryKey)} · ${providerName(primaryKey.provider)}`
                      : "Select an unpaused key to enable AI."}
              </span>
              <div className="button-row ai-primary-controls">
                {selectingPrimary ? (
                  <>
                    <span className="ai-help ai-primary-prompt">
                      Choose a key below, or confirm with none selected
                    </span>
                    <button
                      type="button"
                      className="button--quiet button--compact"
                      onClick={() => {
                        setSelectingPrimary(false);
                        setPrimaryCandidate(null);
                      }}
                    >
                      Cancel selection
                    </button>
                    <button
                      type="button"
                      aria-label="Confirm primary key"
                      disabled={blocked || working}
                      onClick={() => void confirmPrimary()}
                    >
                      Confirm
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    className="button--secondary button--compact"
                    disabled={
                      blocked ||
                      working ||
                      !visibleKeys.some(
                        (key) => !key.paused && !key.cleanupRequired,
                      )
                    }
                    onClick={() => {
                      setSelectingPrimary(true);
                      setPrimaryCandidate(null);
                    }}
                  >
                    Set primary key
                  </button>
                )}
              </div>
            </div>
            {addOpen && (
              <div
                className="ai-key-action-backdrop"
                role="presentation"
                onPointerDown={(event) => {
                  if (event.target === event.currentTarget && !working) {
                    setAddOpen(false);
                    setKey("");
                    setProvider("");
                    setShowKey(false);
                  }
                }}
              >
                <form
                  className="ai-key-action-popup ai-add-key-popup"
                  aria-label="Add new API key"
                  onSubmit={(event) => void addKey(event)}
                  onKeyDown={(event) => {
                    if (event.key === "Escape" && !working) {
                      setAddOpen(false);
                      setKey("");
                      setProvider("");
                      setShowKey(false);
                    }
                  }}
                >
                  <div className="ai-add-key-popup__heading">
                    <strong>Add API key</strong>
                    <span>Choose a provider, then enter its API key.</span>
                  </div>
                  <fieldset className="ai-provider-picker">
                    <legend>Provider</legend>
                    <div className="ai-provider-options">
                      {(["openai", "anthropic", "gemini"] as const).map(
                        (option, index) => (
                          <button
                            autoFocus={index === 0}
                            key={option}
                            type="button"
                            className={
                              provider === option
                                ? "ai-provider-option ai-provider-option--selected"
                                : "ai-provider-option"
                            }
                            aria-label={`Select ${providerName(option)}`}
                            aria-pressed={provider === option}
                            disabled={blocked || working}
                            onClick={() =>
                              setProvider((current) =>
                                current === option ? "" : option,
                              )
                            }
                          >
                            <span className="ai-provider-mark">
                              <ProviderMark provider={option} />
                            </span>
                            <span>{providerName(option)}</span>
                            <span
                              className="ai-provider-check"
                              aria-hidden="true"
                            >
                              {provider === option ? "✓" : ""}
                            </span>
                          </button>
                        ),
                      )}
                    </div>
                  </fieldset>
                  <label className="field ai-api-key-field">
                    API key
                    <span className="ai-api-key-input">
                      <input
                        type={showKey ? "text" : "password"}
                        autoComplete="off"
                        placeholder="Paste your API key"
                        value={key}
                        disabled={blocked || working}
                        onChange={(event) => setKey(event.target.value)}
                      />
                      <button
                        type="button"
                        className="ai-api-key-visibility"
                        aria-label={showKey ? "Hide API key" : "Show API key"}
                        aria-pressed={showKey}
                        disabled={blocked || working || !key}
                        onClick={() => setShowKey((visible) => !visible)}
                      >
                        {showKey ? "Hide" : "Show"}
                      </button>
                    </span>
                  </label>
                  <p className="ai-help">
                    The selected provider is fixed after saving. This won’t
                    change your primary key.
                  </p>
                  <div className="button-row ai-key-action-popup__actions">
                    <button
                      type="button"
                      className="button--secondary"
                      disabled={working}
                      onClick={() => {
                        setAddOpen(false);
                        setKey("");
                        setProvider("");
                        setShowKey(false);
                      }}
                    >
                      Cancel
                    </button>
                    <button
                      type="submit"
                      disabled={blocked || working || !key.trim() || !provider}
                    >
                      Save key
                    </button>
                  </div>
                </form>
              </div>
            )}
            {keysUnavailable ? (
              <p role="alert">
                Saved keys are unavailable. Reload before sending a request.
              </p>
            ) : (
              !registry && <p className="ai-help">Loading saved keys…</p>
            )}
            {registry && !visibleKeys.length && (
              <div className="ai-keys-empty">
                No saved keys. Add one to get started.
              </div>
            )}
            <div className="ai-key-list">
              {visibleKeys.map((saved) => {
                const isPrimary =
                  saved.credentialId === primaryKey?.credentialId;
                return (
                  <div
                    className={`ai-key-row${saved.paused ? " ai-key-row--paused" : ""}${isPrimary ? " ai-key-row--primary" : ""}${selectingPrimary && primaryCandidate === saved.credentialId ? " ai-key-row--candidate" : ""}${selectingPrimary && !saved.paused && !saved.cleanupRequired ? " ai-key-row--selectable" : ""}`}
                    key={saved.credentialId}
                    onClick={(event) => {
                      if (
                        selectingPrimary &&
                        !blocked &&
                        !working &&
                        !saved.paused &&
                        !saved.cleanupRequired &&
                        !(event.target as HTMLElement).closest(
                          "button, input, select, form",
                        )
                      )
                        setPrimaryCandidate((current) =>
                          current === saved.credentialId
                            ? null
                            : saved.credentialId,
                        );
                    }}
                    aria-label={`${keyName(saved)} · ${providerName(saved.provider)}`}
                  >
                    <AiKeyCustomization
                      saved={saved}
                      catalog={catalog}
                      blocked={
                        blocked || working || selectingPrimary || saved.paused
                      }
                      refreshRevision={monitoringRevision}
                      setWorking={setWorking}
                      onRegistry={applyRegistry}
                      onChanged={() =>
                        setMonitoringRevision((value) => value + 1)
                      }
                      status={
                        <>
                          {selectingPrimary && (
                            <input
                              type="checkbox"
                              name="primary-key-selection"
                              aria-label={`Select key #${saved.identificationNumber} as primary`}
                              checked={primaryCandidate === saved.credentialId}
                              disabled={
                                blocked ||
                                working ||
                                saved.paused ||
                                saved.cleanupRequired
                              }
                              onChange={() =>
                                setPrimaryCandidate((current) =>
                                  current === saved.credentialId
                                    ? null
                                    : saved.credentialId,
                                )
                              }
                            />
                          )}
                        </>
                      }
                      identity={
                        selectingPrimary || saved.paused ? (
                          <strong className="ai-key-static-name">
                            {keyName(saved)}
                          </strong>
                        ) : (
                          <AiKeyName
                            saved={saved}
                            blocked={blocked || working}
                            workspaceBlocked={blocked}
                            setWorking={setWorking}
                            onSaved={savedName}
                          />
                        )
                      }
                      actions={
                        <AiKeyMenu
                          saved={saved}
                          blocked={blocked || working}
                          onTest={() => {
                            setRemoveConfirm(null);
                            void reviewTest(saved);
                          }}
                          onPause={() =>
                            void changeKey(
                              saved,
                              saved.paused ? "unpause" : "pause",
                            )
                          }
                          onRemove={() => {
                            setTestPreview(null);
                            setTestTarget(null);
                            setRemoveConfirm(saved.credentialId);
                          }}
                        />
                      }
                    />
                  </div>
                );
              })}
            </div>
            {(() => {
              const target = visibleKeys.find(
                (saved) => saved.credentialId === removeConfirm,
              );
              if (!target) return null;
              const isPrimary =
                target.credentialId === primaryKey?.credentialId;
              return (
                <div
                  className="ai-key-action-backdrop"
                  role="presentation"
                  onPointerDown={(event) => {
                    if (event.target === event.currentTarget && !working)
                      setRemoveConfirm(null);
                  }}
                >
                  <div
                    className="ai-key-action-popup"
                    role="dialog"
                    aria-modal="false"
                    aria-label="Confirm provider credential removal"
                    onKeyDown={(event) => {
                      if (event.key === "Escape" && !working)
                        setRemoveConfirm(null);
                    }}
                  >
                    <strong>Remove {keyName(target)}?</strong>
                    <p>
                      This removes the key from the vault. Its activity history
                      stays.{" "}
                      {isPrimary
                        ? "No primary key will remain selected."
                        : "Your primary key won’t change."}
                    </p>
                    <div className="button-row ai-key-action-popup__actions">
                      <button
                        autoFocus
                        type="button"
                        className="button--secondary"
                        onClick={() => setRemoveConfirm(null)}
                      >
                        Cancel
                      </button>
                      <button
                        type="button"
                        className="button--danger"
                        disabled={blocked || working}
                        onClick={() => void changeKey(target, "remove")}
                      >
                        Remove key
                      </button>
                    </div>
                  </div>
                </div>
              );
            })()}
            {testTarget && (testPreview || testActive || testOutput) && (
              <div
                className="ai-key-action-backdrop"
                role="presentation"
                onPointerDown={(event) => {
                  if (event.target === event.currentTarget) {
                    setTestPreview(null);
                    setTestTarget(null);
                  }
                }}
              >
                <div
                  className="ai-key-action-popup ai-key-test-popup"
                  role="dialog"
                  aria-modal="false"
                  aria-label="Confirm synthetic provider request"
                  onKeyDown={(event) => {
                    if (event.key === "Escape") {
                      setTestPreview(null);
                      setTestTarget(null);
                    }
                  }}
                >
                  {testPreview && (
                    <>
                      <strong>
                        Test {keyName(testTarget)} · {testPreview.model}
                      </strong>
                      <p>
                        Fixed test only; no resume or personal content. At most{" "}
                        {testPreview.estimatedInputTokens} tokens of input.
                      </p>
                      <p>
                        Conservative maximum reservation:{" "}
                        {(testPreview.maximumCostMicros / 1_000_000).toFixed(4)}{" "}
                        {testPreview.currency}.
                      </p>
                      <p className="ai-help">
                        Provider terms, retention and privacy practices apply.
                        Actual billing may differ.
                      </p>
                      <div className="button-row ai-key-action-popup__actions">
                        <button
                          autoFocus
                          type="button"
                          className="button--secondary"
                          onClick={() => {
                            setTestPreview(null);
                            setTestTarget(null);
                          }}
                        >
                          Cancel
                        </button>
                        <button
                          type="button"
                          disabled={blocked || working}
                          onClick={() => void confirmTest()}
                        >
                          Confirm and send test
                        </button>
                      </div>
                    </>
                  )}
                  {testActive && (
                    <>
                      <strong>Testing {keyName(testTarget)}…</strong>
                      <p className="ai-help">
                        Waiting for the provider response.
                      </p>
                      <button
                        type="button"
                        className="button--secondary"
                        onClick={() => void cancelTest()}
                      >
                        Cancel active test
                      </button>
                    </>
                  )}
                  {testOutput && (
                    <details open>
                      <summary>Test response</summary>
                      <pre>{testOutput}</pre>
                    </details>
                  )}
                </div>
              </div>
            )}
          </section>
          <AiGeneralSpending
            blocked={blocked || working}
            refreshRevision={monitoringRevision}
            setWorking={setWorking}
            onChanged={() => setMonitoringRevision((value) => value + 1)}
          />
        </div>
        <div
          className="ai-page-panels"
          hidden={page !== "data"}
          aria-label="AI usage data"
        >
          <section className="ai-panel" aria-label="Usage data">
            <div className="ai-data-heading">
              <div>
                <strong>{activityView.title}</strong>
                <small>{activityView.detail}</small>
              </div>
              <AiDataKeyPicker
                keys={activityKeys}
                catalog={catalog}
                value={keyFilter}
                disabled={blocked || working || keysUnavailable}
                onChange={(value) => {
                  setKeyFilter(value);
                }}
              />
            </div>
            {monitoringError && (
              <p role="alert">AI Monitoring is unavailable.</p>
            )}
            {!monitoring && !monitoringError && (
              <p role="status">Loading local activity…</p>
            )}
            {monitoring && (
              <>
                <AiUsageChart
                  key={`${period}-${keyFilter}-${metric}-${chartCurrency}-${monitoringRevision}`}
                  buckets={monitoring.timeBuckets}
                  period={period}
                  currency={chartCurrency}
                  metric={metric}
                  onMetricChange={setMetric}
                  onPeriodChange={setPeriod}
                  currencies={currencies}
                  onCurrencyChange={setCurrency}
                  summary={{
                    label:
                      metric === "cost"
                        ? `Recorded estimate · ${period}`
                        : `Estimated tokens · ${period}`,
                    value:
                      metric === "cost"
                        ? Object.entries(monitoring.costByCurrencyMicros)
                            .map(
                              ([name, micros]) =>
                                `${(micros / 1_000_000).toFixed(6)} ${name}`,
                            )
                            .join(" · ") || "No recorded cost"
                        : totalTokens(monitoring.usage).toLocaleString(),
                    detail: `${monitoring.logicalOperations} operations · ${monitoring.attempts} attempts`,
                  }}
                />
                {monitoring.partial && (
                  <p className="ai-partial" role="status">
                    Partial or unknown usage: {monitoring.unknownCount}{" "}
                    attempts. Unresolved reserved exposure:{" "}
                    {monitoring.unresolvedReservedMicros} micros. Missing usage
                    is not zero spend.
                  </p>
                )}
                {monitoring.attempts === 0 && (
                  <p className="ai-help">
                    No recorded activity for {period.toLowerCase()}.
                  </p>
                )}
              </>
            )}
            <p className="ai-help ai-billing-note">
              Estimates cover this installation only. Your provider’s billing
              dashboard is the source of truth.
            </p>
          </section>
          <section
            className="ai-panel ai-data-settings"
            aria-labelledby="ai-data-settings-title"
          >
            <div className="ai-panel-heading ai-data-settings-heading">
              <div>
                <h3 id="ai-data-settings-title">Settings</h3>
                <p className="ai-help">
                  Manage saved activity independently from the graph above.
                </p>
              </div>
            </div>
            <div className="ai-data-settings-list">
              <div className="ai-data-setting-row">
                <div>
                  <strong>Export activity</strong>
                  <span>
                    Choose a key and one or more active months to export.
                  </span>
                </div>
                <button
                  type="button"
                  className="button--secondary"
                  disabled={blocked || working || keysUnavailable}
                  onClick={() => setDataAction("export")}
                >
                  Export JSON…
                </button>
              </div>
              <div className="ai-data-setting-row">
                <div>
                  <strong>Clear activity</strong>
                  <span>
                    Remove selected months without changing spend or caps.
                  </span>
                </div>
                <button
                  type="button"
                  className="button--secondary"
                  disabled={blocked || working || keysUnavailable}
                  onClick={() => setDataAction("clear")}
                >
                  Choose activity…
                </button>
              </div>
              <div
                className="ai-data-setting-row ai-data-setting-row--retention"
                aria-label="AI activity retention"
              >
                <div>
                  <strong>Activity retention</strong>
                  <span>
                    Automatically remove older local activity without resetting
                    caps.
                  </span>
                </div>
                <div className="ai-retention-controls">
                  <label className="field">
                    Retention policy
                    <select
                      value={retention}
                      disabled={blocked || working}
                      onChange={(event) =>
                        setRetention(event.target.value as RetentionPolicy)
                      }
                    >
                      <option value="30_days">30 days</option>
                      <option value="90_days">90 days</option>
                      <option value="one_year">One year</option>
                      <option value="retain_until_cleared">
                        Retain until cleared
                      </option>
                    </select>
                  </label>
                  <button
                    type="button"
                    className="button--secondary"
                    disabled={blocked || working}
                    onClick={() => {
                      if (retention === "retain_until_cleared")
                        void saveRetention();
                      else setRetentionConfirm(true);
                    }}
                  >
                    Apply retention
                  </button>
                </div>
              </div>
            </div>
            {retentionConfirm && (
              <div
                className="ai-data-action-backdrop"
                role="presentation"
                onPointerDown={(event) => {
                  if (event.target === event.currentTarget && !working)
                    setRetentionConfirm(false);
                }}
              >
                <div
                  className="ai-data-action-dialog ai-retention-confirm-dialog"
                  role="dialog"
                  aria-modal="true"
                  aria-labelledby="ai-retention-confirm-title"
                  onKeyDown={(event) => {
                    if (event.key === "Escape" && !working)
                      setRetentionConfirm(false);
                  }}
                >
                  <div className="ai-data-action-heading">
                    <div>
                      <span>Permanent deletion</span>
                      <h3 id="ai-retention-confirm-title">
                        Apply retention policy?
                      </h3>
                    </div>
                  </div>
                  <p className="ai-retention-confirm-copy">
                    Activity older than{" "}
                    <strong>
                      {retention === "30_days"
                        ? "30 days"
                        : retention === "90_days"
                          ? "90 days"
                          : "one year"}
                    </strong>{" "}
                    will be permanently deleted from this app. This cannot be
                    undone.
                  </p>
                  <p className="ai-help">
                    Lifetime spend and spending-cap progress on My Keys will not
                    change.
                  </p>
                  <div className="ai-data-action-footer">
                    <button
                      autoFocus
                      type="button"
                      className="button--secondary"
                      disabled={working}
                      onClick={() => setRetentionConfirm(false)}
                    >
                      Cancel
                    </button>
                    <button
                      type="button"
                      className="button--danger"
                      disabled={blocked || working}
                      onClick={() => {
                        setRetentionConfirm(false);
                        void saveRetention();
                      }}
                    >
                      Permanently delete older activity
                    </button>
                  </div>
                </div>
              </div>
            )}
            <AiDataActionDialog
              action={dataAction}
              keys={activityKeys}
              catalog={catalog}
              disabled={blocked || working}
              onCancel={() => setDataAction(null)}
              onConfirm={(targetKey, selectedMonths) => {
                if (dataAction === "export")
                  void exportMonitoring(targetKey, selectedMonths);
                else if (dataAction === "clear")
                  void clearMonitoring(targetKey, selectedMonths);
              }}
            />
          </section>
        </div>
      </div>
    </section>
  );
}
