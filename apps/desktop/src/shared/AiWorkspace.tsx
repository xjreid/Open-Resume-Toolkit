import { Channel, invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { AiKeyName } from "./AiKeyName";
import { AiKeyMenu } from "./AiKeyMenu";
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
function periodBounds(period: "Week" | "Month" | "Year" | "All time") {
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

function monitoringArgs(period: "Week" | "Month" | "Year" | "All time") {
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
  const [clearKey, setClearKey] = useState("");
  const [testTarget, setTestTarget] = useState<SavedKey | null>(null);
  const [provider, setProvider] = useState<
    "" | "openai" | "anthropic" | "gemini"
  >("");
  const [key, setKey] = useState("");
  const [working, setWorking] = useState(false);
  const [notice, setNotice] = useState("");
  const [removeConfirm, setRemoveConfirm] = useState<string | null>(null);
  const [period, setPeriod] = useState<"Week" | "Month" | "Year" | "All time">(
    "Month",
  );
  const [monitoring, setMonitoring] = useState<Monitoring | null>(null);
  const [monitoringError, setMonitoringError] = useState(false);
  const [monitoringRevision, setMonitoringRevision] = useState(0);
  const [clearConfirm, setClearConfirm] = useState(false);
  const [clearPeriod, setClearPeriod] = useState(period);
  const [metric, setMetric] = useState<"cost" | "tokens">("cost");
  const [currency, setCurrency] = useState("USD");
  const [testPreview, setTestPreview] = useState<TestPreview | null>(null);
  const [testOutput, setTestOutput] = useState("");
  const [testActive, setTestActive] = useState(false);
  const [retention, setRetention] = useState<RetentionPolicy>(
    "retain_until_cleared",
  );
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

  async function clearMonitoring() {
    if (blocked || working) return;
    setWorking(true);
    setClearConfirm(false);
    try {
      const response = await invoke<
        { ok: true; value: number } | { ok: false; error: { code: string } }
      >("clear_ai_monitoring", {
        ...periodBounds(clearPeriod),
        credentialId: clearKey || null,
      });
      if (response.ok) {
        setNotice(
          `${response.value} completed AI operations cleared. Spending cap counters were not reset.`,
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

  async function exportMonitoring() {
    if (!monitoring || blocked || working) return;
    setWorking(true);
    try {
      const response = await invoke<
        { ok: true; value: string } | { ok: false; error: { code: string } }
      >("export_ai_monitoring", {
        ...monitoringArgs(period),
        credentialId: keyFilter || null,
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
  const keyLabel = (id: string) => {
    const key = activityKeys.find((key) => key.credentialId === id);
    return key?.identificationNumber
      ? `${keyName(key)} · ${providerName(key.provider)}${key.removed ? " · Removed" : ""}`
      : `Archived key · ${id.slice(0, 8)}`;
  };
  const currencies = Object.keys(monitoring?.costByCurrencyMicros ?? {});
  const chartCurrency = currencies.includes(currency)
    ? currency
    : (currencies[0] ?? "USD");

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
                disabled={blocked || working || !registry}
                aria-expanded={addOpen}
                onClick={() => setAddOpen(true)}
              >
                Add key
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
                      ? "Primary key"
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
                      Choose a key below
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
                      disabled={blocked || working || !primaryCandidate}
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
              <form
                className="ai-add-key"
                aria-label="Add new API key"
                onSubmit={(event) => void addKey(event)}
              >
                <div className="ai-field-row">
                  <label className="field">
                    API key
                    <input
                      type="password"
                      autoComplete="off"
                      placeholder="Paste your API key"
                      value={key}
                      disabled={blocked || working}
                      onChange={(event) => setKey(event.target.value)}
                    />
                  </label>
                  <label className="field">
                    Provider
                    <select
                      value={provider}
                      disabled={blocked || working}
                      onChange={(event) =>
                        setProvider(event.target.value as typeof provider)
                      }
                    >
                      <option value="" disabled>
                        Select a provider
                      </option>
                      <option value="openai">OpenAI</option>
                      <option value="anthropic">Anthropic</option>
                      <option value="gemini">Gemini</option>
                    </select>
                  </label>
                </div>
                <p className="ai-help">
                  Each key’s provider is fixed. Saving a new key won’t change
                  your primary key.
                </p>
                <div className="button-row">
                  <button
                    type="submit"
                    disabled={blocked || working || !key.trim() || !provider}
                  >
                    Save key
                  </button>
                  <button
                    type="button"
                    className="button--secondary"
                    disabled={working}
                    onClick={() => {
                      setAddOpen(false);
                      setKey("");
                      setProvider("");
                    }}
                  >
                    Cancel
                  </button>
                </div>
              </form>
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
                        setPrimaryCandidate(saved.credentialId);
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
                              type="radio"
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
                                setPrimaryCandidate(saved.credentialId)
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
                          onTest={() => void reviewTest(saved)}
                          onPause={() =>
                            void changeKey(
                              saved,
                              saved.paused ? "unpause" : "pause",
                            )
                          }
                          onRemove={() => setRemoveConfirm(saved.credentialId)}
                        />
                      }
                    />
                    {removeConfirm === saved.credentialId && (
                      <div
                        className="ai-confirm"
                        role="group"
                        aria-label="Confirm provider credential removal"
                      >
                        <p>
                          Remove {keyName(saved)} from the vault? Activity
                          stays.{" "}
                          {isPrimary
                            ? "No primary key will remain selected."
                            : "Your primary key won’t change."}
                        </p>
                        <div className="button-row">
                          <button
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
                            onClick={() => void changeKey(saved, "remove")}
                          >
                            Remove key
                          </button>
                        </div>
                      </div>
                    )}
                    {testTarget?.credentialId === saved.credentialId &&
                      testPreview && (
                        <div
                          className="ai-confirm"
                          role="group"
                          aria-label="Confirm synthetic provider request"
                        >
                          <strong>
                            Test {keyName(saved)} · {testPreview.model}
                          </strong>
                          <p>
                            Fixed test only; no resume or personal content. At
                            most {testPreview.estimatedInputTokens} tokens of
                            input.
                          </p>
                          <p>
                            Conservative maximum reservation:{" "}
                            {(
                              testPreview.maximumCostMicros / 1_000_000
                            ).toFixed(4)}{" "}
                            {testPreview.currency}.
                          </p>
                          <p className="ai-help">
                            Provider terms, retention and privacy practices
                            apply. Actual billing may differ.
                          </p>
                          <div className="button-row">
                            <button
                              type="button"
                              className="button--secondary"
                              onClick={() => setTestPreview(null)}
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
                        </div>
                      )}
                    {testTarget?.credentialId === saved.credentialId &&
                      testActive && (
                        <button
                          type="button"
                          className="button--secondary"
                          onClick={() => void cancelTest()}
                        >
                          Cancel active test
                        </button>
                      )}
                    {testTarget?.credentialId === saved.credentialId &&
                      testOutput && (
                        <details>
                          <summary>Test response</summary>
                          <pre>{testOutput}</pre>
                        </details>
                      )}
                  </div>
                );
              })}
            </div>
          </section>
        </div>
        <div
          className="ai-page-panels"
          hidden={page !== "data"}
          aria-label="AI usage data"
        >
          <section className="ai-panel" aria-labelledby="ai-usage-title">
            <div className="ai-panel-heading">
              <h3 id="ai-usage-title">Usage</h3>
              <div className="button-row">
                <button
                  type="button"
                  className="button--quiet button--compact"
                  disabled={!monitoring || blocked || working}
                  onClick={() => void exportMonitoring()}
                >
                  Export aggregate JSON
                </button>
                <button
                  type="button"
                  className="button--quiet button--compact"
                  disabled={!monitoring?.attempts || blocked || working}
                  onClick={() => {
                    setClearPeriod(period);
                    setClearKey(keyFilter);
                    setClearConfirm(true);
                  }}
                >
                  Clear selected activity
                </button>
              </div>
            </div>
            <div className="ai-chart-toolbar">
              <div
                className="ai-segments"
                role="group"
                aria-label="Monitoring period"
              >
                {(["Week", "Month", "Year", "All time"] as const).map(
                  (value) => (
                    <button
                      type="button"
                      className="button--secondary"
                      aria-pressed={period === value}
                      key={value}
                      onClick={() => {
                        setPeriod(value);
                        setClearConfirm(false);
                      }}
                    >
                      {value}
                    </button>
                  ),
                )}
              </div>
              <div className="ai-field-row">
                <label className="field">
                  Activity by key
                  <select
                    value={keyFilter}
                    onChange={(event) => {
                      setKeyFilter(event.target.value);
                      setClearConfirm(false);
                    }}
                  >
                    <option value="">All keys</option>
                    {activityKeys.map((saved) => (
                      <option
                        key={saved.credentialId}
                        value={saved.credentialId}
                      >
                        {keyLabel(saved.credentialId)}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field">
                  Y axis
                  <select
                    value={metric}
                    onChange={(event) =>
                      setMetric(event.target.value as typeof metric)
                    }
                  >
                    <option value="cost">Estimated price</option>
                    <option value="tokens">Estimated tokens</option>
                  </select>
                </label>
                {metric === "cost" && currencies.length > 1 && (
                  <label className="field">
                    Currency
                    <select
                      value={chartCurrency}
                      onChange={(event) => setCurrency(event.target.value)}
                    >
                      {currencies.map((value) => (
                        <option key={value}>{value}</option>
                      ))}
                    </select>
                  </label>
                )}
              </div>
            </div>
            {clearConfirm && (
              <div
                className="ai-confirm"
                role="group"
                aria-label="Confirm AI activity clearing"
              >
                <p>
                  Clear completed activity for {clearPeriod.toLowerCase()} ·{" "}
                  {clearKey ? keyLabel(clearKey) : "All keys"}? This does not
                  reset spending caps or provider billing.
                </p>
                <div className="button-row">
                  <button
                    type="button"
                    className="button--secondary"
                    onClick={() => setClearConfirm(false)}
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    className="button--danger"
                    disabled={blocked || working}
                    onClick={() => void clearMonitoring()}
                  >
                    Clear activity
                  </button>
                </div>
              </div>
            )}
            {monitoringError && (
              <p role="alert">AI Monitoring is unavailable.</p>
            )}
            {!monitoring && !monitoringError && (
              <p role="status">Loading local activity…</p>
            )}
            {monitoring && (
              <>
                <div className="ai-usage-summary">
                  <div>
                    <span>Estimated recorded cost</span>
                    <strong>
                      {Object.entries(monitoring.costByCurrencyMicros)
                        .map(
                          ([name, micros]) =>
                            `${(micros / 1_000_000).toFixed(6)} ${name}`,
                        )
                        .join(" · ") || "No recorded cost"}
                    </strong>
                  </div>
                  <div>
                    <span>Estimated tokens</span>
                    <strong>
                      {totalTokens(monitoring.usage).toLocaleString()}
                    </strong>
                  </div>
                  <div>
                    <span>Activity</span>
                    <strong>
                      {monitoring.logicalOperations} operations ·{" "}
                      {monitoring.attempts} attempts
                    </strong>
                  </div>
                </div>
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
                <AiUsageChart
                  key={`${period}-${keyFilter}-${metric}-${chartCurrency}-${monitoringRevision}`}
                  buckets={monitoring.timeBuckets}
                  period={period}
                  currency={chartCurrency}
                  metric={metric}
                />
                {monitoring.attempts > 0 && (
                  <details className="ai-details">
                    <summary>Activity breakdown</summary>
                    <table className="ai-breakdown-table">
                      <caption className="visually-hidden">
                        Activity by provider, model, preset, operation and
                        outcome
                      </caption>
                      <thead>
                        <tr>
                          <th scope="col">Category</th>
                          <th scope="col">Name</th>
                          <th scope="col">Attempts</th>
                        </tr>
                      </thead>
                      <tbody>
                        {[
                          ["Providers", monitoring.byProvider],
                          ["Models", monitoring.byModel],
                          ["Presets", monitoring.byPreset],
                          ["Operation types", monitoring.byOperationType],
                          ["Outcomes", monitoring.byStatus],
                        ].flatMap(([label, data]) =>
                          Object.entries(data).map(([name, count]) => (
                            <tr key={`${label}-${name}`}>
                              <td>{label as string}</td>
                              <td>{name}</td>
                              <td>{count}</td>
                            </tr>
                          )),
                        )}
                      </tbody>
                    </table>
                  </details>
                )}
              </>
            )}
            <div className="ai-retention" aria-label="AI activity retention">
              <div>
                <h4>Activity retention</h4>
                <p className="ai-help">
                  Automatically delete older local activity. Cap counters and
                  provider records are unchanged.
                </p>
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
                  onClick={() => void saveRetention()}
                >
                  Apply retention
                </button>
              </div>
            </div>
            <p className="ai-help ai-billing-note">
              Estimates cover this installation only. Your provider’s billing
              dashboard is the source of truth.
            </p>
          </section>
        </div>
      </div>
    </section>
  );
}
