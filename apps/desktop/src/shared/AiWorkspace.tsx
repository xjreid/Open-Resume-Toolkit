import { Channel, invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";

type Connection = {
  mode: "no_ai" | "direct_api";
  provider: "openai" | "anthropic" | "gemini" | null;
  preset: "economy" | "balanced" | "quality" | null;
  credentialId: string | null;
};
type Response =
  | { ok: true; value: Connection }
  | { ok: false; error: { code: string } };

type Monitoring = {
  logicalOperations: number;
  attempts: number;
  usage: { inputTokens: number; outputTokens: number };
  estimatedCostMicros: number;
  unresolvedReservedMicros: number;
  currency: string | null;
  partial: boolean;
  unknownCount: number;
  byProvider: Record<string, number>;
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
type CapPeriod = "week" | "month" | "year" | "all_time";
type Cap = {
  credentialId: string;
  period: CapPeriod;
  currency: string;
  timeZone: string;
  limitMicros: number;
  activatedAtUnixMs: number;
  periodStartUnixMs: number;
  periodEndUnixMs: number | null;
  countedMicros: number;
  reservedMicros: number;
  unresolvedMicros: number;
  revision: number;
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
type Catalog = {
  catalogId: string;
  expiresAt: string;
  entries: CatalogEntry[];
};
const priceDimensions = [
  ["input", "Input"],
  ["cached_input", "Cached input"],
  ["cache_write", "Cache write"],
  ["output", "Output"],
  ["reasoning", "Reasoning"],
] as const;

function selectedCatalogEntry(
  catalog: Catalog | null,
  provider: Connection["provider"],
  preset: Connection["preset"],
) {
  return (
    catalog?.entries.find(
      (entry) =>
        entry.provider === (provider === "openai" ? "open_ai" : provider) &&
        entry.preset === preset &&
        !entry.disabled,
    ) ?? null
  );
}

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
  const [connection, setConnection] = useState<Connection | null>(null);
  const [provider, setProvider] = useState<"openai" | "anthropic" | "gemini">(
    "openai",
  );
  const [preset, setPreset] = useState<"economy" | "balanced" | "quality">(
    "balanced",
  );
  const [key, setKey] = useState("");
  const [copyGuardrails, setCopyGuardrails] = useState(false);
  const [working, setWorking] = useState(false);
  const [notice, setNotice] = useState("");
  const [removeConfirm, setRemoveConfirm] = useState(false);
  const [period, setPeriod] = useState<"Week" | "Month" | "Year" | "All time">(
    "Month",
  );
  const [monitoring, setMonitoring] = useState<Monitoring | null>(null);
  const [monitoringError, setMonitoringError] = useState(false);
  const [monitoringRevision, setMonitoringRevision] = useState(0);
  const [clearConfirm, setClearConfirm] = useState(false);
  const [testPreview, setTestPreview] = useState<TestPreview | null>(null);
  const [testOutput, setTestOutput] = useState("");
  const [testActive, setTestActive] = useState(false);
  const [caps, setCaps] = useState<Cap[]>([]);
  const [capPeriod, setCapPeriod] = useState<CapPeriod>("month");
  const [capAmount, setCapAmount] = useState("");
  const [capConfirm, setCapConfirm] = useState<"disable" | "reset" | null>(
    null,
  );
  const [retention, setRetention] = useState<RetentionPolicy>(
    "retain_until_cleared",
  );
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [catalogUnavailable, setCatalogUnavailable] = useState(false);

  useEffect(() => {
    void invoke<{ ok: true; value: Catalog } | { ok: false }>("load_ai_catalog")
      .then((response) => {
        if (response.ok) setCatalog(response.value);
        else setCatalogUnavailable(true);
      })
      .catch(() => setCatalogUnavailable(true));
  }, []);

  useEffect(() => {
    void invoke<Response>("load_ai_connection")
      .then((response) => {
        if (response.ok) {
          setConnection(response.value);
          if (response.value.provider) setProvider(response.value.provider);
          if (response.value.preset) setPreset(response.value.preset);
        } else setNotice("AI connection state is unavailable.");
      })
      .catch(() => setNotice("AI connection state is unavailable."));
  }, []);

  useEffect(() => {
    let current = true;
    setMonitoring(null);
    setMonitoringError(false);
    void invoke<MonitoringResponse>(
      "load_ai_monitoring",
      monitoringArgs(period),
    )
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
  }, [period, monitoringRevision]);

  useEffect(() => {
    let current = true;
    void invoke<
      { ok: true; value: Cap[] } | { ok: false; error: { code: string } }
    >("load_ai_caps")
      .then((response) => {
        if (current && response.ok) setCaps(response.value);
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [connection?.credentialId]);

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

  useEffect(() => {
    const existing = caps.find((cap) => cap.period === capPeriod);
    setCapAmount(existing ? (existing.limitMicros / 1_000_000).toString() : "");
  }, [capPeriod, caps]);

  async function saveCap(event: FormEvent) {
    event.preventDefault();
    const dollars = Number(capAmount);
    const limitMicros = Math.round(dollars * 1_000_000);
    if (!Number.isSafeInteger(limitMicros) || limitMicros <= 0 || working) {
      setNotice(
        "Enter a positive spending cap with no more than six decimal places.",
      );
      return;
    }
    const existing = caps.find((cap) => cap.period === capPeriod);
    const timeZone =
      existing?.timeZone ?? Intl.DateTimeFormat().resolvedOptions().timeZone;
    setWorking(true);
    try {
      const response = await invoke<
        { ok: true; value: Cap } | { ok: false; error: { code: string } }
      >("save_ai_cap", {
        request: {
          period: capPeriod,
          limitMicros,
          timeZone,
          expectedRevision: existing?.revision ?? null,
        },
      });
      if (response.ok) {
        setCaps((current) => [
          ...current.filter((cap) => cap.period !== response.value.period),
          response.value,
        ]);
        setNotice(
          `${capPeriod.replace("_", " ")} cap saved. Its activation baseline and recorded time zone remain fixed.`,
        );
      } else
        setNotice(
          response.error.code === "AI_CAP_CHANGED"
            ? "The cap changed; reload and review before saving again."
            : "The spending cap could not be saved.",
        );
    } catch {
      setNotice("The spending cap command is unavailable.");
    } finally {
      setWorking(false);
    }
  }

  async function capAction(action: "disable" | "reset") {
    setCapConfirm(null);
    setWorking(true);
    try {
      const command = action === "disable" ? "disable_ai_cap" : "reset_ai_cap";
      const response = await invoke<
        { ok: true; value: boolean } | { ok: false; error: { code: string } }
      >(command, { request: { period: capPeriod } });
      if (response.ok) {
        try {
          const latest = await invoke<
            { ok: true; value: Cap[] } | { ok: false }
          >("load_ai_caps");
          if (latest.ok) {
            setCaps(latest.value);
            setNotice(
              action === "disable"
                ? "Cap disabled. Activity history and provider billing are unchanged."
                : "All-time cap baseline reset to zero at the new activation time. Activity history and provider billing are unchanged.",
            );
          } else
            setNotice(
              "The cap action completed, but the displayed state could not be refreshed. Reload before another request.",
            );
        } catch {
          setNotice(
            "The cap action completed, but the displayed state could not be refreshed. Reload before another request.",
          );
        }
      } else
        setNotice(
          response.error.code === "AI_BUSY"
            ? "Finish or cancel the active AI request first."
            : "The cap action could not be completed.",
        );
    } catch {
      setNotice("The cap action command is unavailable.");
    } finally {
      setWorking(false);
    }
  }

  async function clearMonitoring() {
    setClearConfirm(false);
    try {
      const response = await invoke<
        { ok: true; value: number } | { ok: false; error: { code: string } }
      >("clear_ai_monitoring", periodBounds(period));
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

  async function reviewTest() {
    if (blocked || working || connection?.mode !== "direct_api") return;
    setWorking(true);
    setNotice("");
    setTestPreview(null);
    try {
      const response = await invoke<
        | { ok: true; value: TestPreview }
        | { ok: false; error: { code: string } }
      >("preview_ai_test");
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
    if (!monitoring) return;
    try {
      const response = await invoke<
        { ok: true; value: string } | { ok: false; error: { code: string } }
      >("export_ai_monitoring", monitoringArgs(period));
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
    }
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!key || working) return;
    setWorking(true);
    setNotice("");
    try {
      const response = await invoke<Response>("save_ai_connection", {
        request: { provider, preset, apiKey: key, copyGuardrails },
      });
      setKey("");
      if (response.ok) {
        setConnection(response.value);
        setTestPreview(null);
        setNotice(
          "Credential saved to the operating-system vault. It will be used only for a confirmed AI request.",
        );
      } else {
        if (
          response.error.code === "AI_CREDENTIAL_CLEANUP_REQUIRED" ||
          response.error.code === "AI_OLD_CREDENTIAL_CLEANUP_REQUIRED"
        ) {
          void invoke<Response>("load_ai_connection")
            .then((latest) => {
              if (latest.ok) setConnection(latest.value);
            })
            .catch(() => {});
        }
        setNotice(
          response.error.code === "AI_OLD_CREDENTIAL_CLEANUP_REQUIRED"
            ? "The replacement was saved, but the previous vault item could not be verified as removed. Reload this view and remove the old item through the operating-system credential manager."
            : response.error.code === "AI_CREDENTIAL_CLEANUP_REQUIRED"
              ? "The credential transition could not be verified. AI may be paused; reload the connection state and inspect the operating-system credential manager before retrying."
              : response.error.code === "AI_VAULT_UNAVAILABLE"
                ? "The operating-system credential vault is unavailable."
                : "The AI connection could not be saved.",
        );
      }
    } catch {
      setNotice("The AI connection command is unavailable.");
    } finally {
      setWorking(false);
    }
  }

  async function disable() {
    setWorking(true);
    setNotice("");
    try {
      const response = await invoke<Response>("disable_ai");
      if (response.ok) {
        setConnection(response.value);
        setTestPreview(null);
        setNotice(
          "No AI is active. Manual and local features remain available.",
        );
      } else setNotice("No AI mode could not be saved.");
    } catch {
      setNotice("The AI connection command is unavailable.");
    } finally {
      setWorking(false);
    }
  }

  async function activate() {
    setWorking(true);
    setNotice("");
    try {
      const response = await invoke<Response>("activate_saved_ai");
      if (response.ok) {
        setConnection(response.value);
        setTestPreview(null);
        setNotice(
          "The saved Direct API connection is active for future confirmed requests.",
        );
      } else setNotice("The saved AI connection could not be activated.");
    } catch {
      setNotice("The AI connection command is unavailable.");
    } finally {
      setWorking(false);
    }
  }

  async function removeCredential() {
    setWorking(true);
    setNotice("");
    try {
      const response = await invoke<Response>("remove_ai_credential");
      if (response.ok) {
        setConnection(response.value);
        setTestPreview(null);
        setNotice(
          "The provider credential was removed from the operating-system vault. Existing content-free activity is unchanged.",
        );
      } else {
        if (response.error.code === "AI_CREDENTIAL_CLEANUP_REQUIRED") {
          void invoke<Response>("load_ai_connection")
            .then((latest) => {
              if (latest.ok) setConnection(latest.value);
            })
            .catch(() => {});
        }
        setNotice(
          response.error.code === "AI_CREDENTIAL_CLEANUP_REQUIRED"
            ? "Credential removal could not be verified. AI may be paused; reload the connection state and inspect the operating-system credential manager before retrying."
            : "The credential could not be removed from the operating-system vault.",
        );
      }
    } catch {
      setNotice("The AI connection command is unavailable.");
    } finally {
      setWorking(false);
    }
  }

  const catalogEntry = selectedCatalogEntry(catalog, provider, preset);

  return (
    <section className="workspace-data" aria-labelledby="ai-title">
      <h2 id="ai-title">AI connection and monitoring</h2>
      <p>
        AI is optional. Direct requests go from this app to the provider you
        select; Open Resume Toolkit has no shared key or relay.
      </p>
      <div className="data-card">
        <p className="eyebrow">Connection</p>
        <h3>
          {connection?.mode === "direct_api"
            ? `${connection.provider} · ${connection.preset}`
            : "No AI"}
        </h3>
        <p>
          Keys stay in the operating-system credential vault and are never shown
          again, included in backups, or written to activity history.
        </p>
        {connection?.mode === "no_ai" && connection.credentialId ? (
          <p>A saved credential is retained but inactive.</p>
        ) : null}
        <form onSubmit={(event) => void save(event)}>
          <label>
            Provider
            <select
              value={provider}
              disabled={blocked || working}
              onChange={(event) =>
                setProvider(event.target.value as typeof provider)
              }
            >
              <option value="openai">OpenAI API</option>
              <option value="anthropic">Anthropic API</option>
              <option value="gemini">Google Gemini API</option>
            </select>
          </label>
          <label>
            Preset
            <select
              value={preset}
              disabled={blocked || working}
              onChange={(event) =>
                setPreset(event.target.value as typeof preset)
              }
            >
              <option value="economy" disabled>
                Economy (not yet verified)
              </option>
              <option value="balanced">Balanced</option>
              <option value="quality" disabled>
                Quality (not yet verified)
              </option>
            </select>
          </label>
          <p className="description">
            The signed development catalog currently enables Balanced for each
            provider. Other presets become selectable only after their verified
            catalog entries are bundled.
          </p>
          <div
            className="data-card"
            aria-label="Selected signed model catalog entry"
          >
            <h4>Selected model and verified pricing</h4>
            {catalogUnavailable ? (
              <p>
                Signed model and pricing information is unavailable. No request
                can be confirmed without it.
              </p>
            ) : null}
            {!catalog && !catalogUnavailable ? (
              <p>Loading signed model catalog…</p>
            ) : null}
            {catalog && !catalogEntry ? (
              <p>
                This provider and preset are unavailable in the signed catalog.
              </p>
            ) : null}
            {catalogEntry ? (
              <div>
                <p>
                  <strong>{catalogEntry.model}</strong> · Balanced preset for
                  supported text operations
                </p>
                <p>
                  Catalog {catalog!.catalogId} · expires {catalog!.expiresAt} ·
                  verified {catalogEntry.verifiedAt}
                </p>
                <p>
                  ORT-tested input limit:{" "}
                  {catalogEntry.maxInputTokens.toLocaleString()} tokens ·
                  ORT-tested output limit:{" "}
                  {catalogEntry.maxOutputTokens.toLocaleString()} tokens.
                  Provider model limits may be larger. The synthetic connection
                  test uses a 64-token output limit.
                </p>
                <p>
                  Catalog-listed operation types:{" "}
                  {catalogEntry.operations.join(", ")}. Only the synthetic
                  credential test is implemented in M3; later workflows require
                  their own validation. Reasoning setting: provider default; no
                  catalog-confirmed setting is available.
                </p>
                <p>
                  Estimated rates per one million tokens (
                  {catalogEntry.currency}); unavailable does not mean zero:
                </p>
                <ul>
                  {priceDimensions.map(([category, label]) => {
                    const price = catalogEntry.prices.find(
                      (item) => item.category === category,
                    );
                    return (
                      <li key={category}>
                        {label}:{" "}
                        {price
                          ? `${(price.microsPerMillion / 1_000_000).toFixed(4)} ${catalogEntry.currency}`
                          : "unavailable"}
                      </li>
                    );
                  })}
                </ul>
                <p>
                  Effective from {catalogEntry.effectiveFrom}
                  {catalogEntry.effectiveTo
                    ? ` through ${catalogEntry.effectiveTo}`
                    : " · no published end date"}
                  . Official source: {catalogEntry.source}
                </p>
                <p>
                  Provider account tiers, credits, taxes, tool fees,
                  service-tier conditions, and later adjustments are not
                  represented by this entry; local costs remain estimates. Check
                  the provider billing dashboard for actual charges.
                </p>
              </div>
            ) : null}
          </div>
          <label>
            API key
            <input
              type="password"
              autoComplete="off"
              value={key}
              disabled={blocked || working}
              onChange={(event) => setKey(event.target.value)}
            />
          </label>
          {connection?.credentialId ? (
            <label>
              <input
                type="checkbox"
                checked={copyGuardrails}
                disabled={blocked || working}
                onChange={(event) => setCopyGuardrails(event.target.checked)}
              />
              Copy enabled cap amounts to the replacement key with new zero
              baselines. Existing counters never transfer.
            </label>
          ) : null}
          <div className="button-row">
            <button type="submit" disabled={blocked || working || !key}>
              Save credential
            </button>
            <button
              type="button"
              className="button--secondary"
              disabled={blocked || working || connection?.mode !== "direct_api"}
              onClick={() => void disable()}
            >
              Pause AI
            </button>
            <button
              type="button"
              className="button--secondary"
              disabled={
                blocked ||
                working ||
                connection?.mode !== "no_ai" ||
                !connection.credentialId
              }
              onClick={() => void activate()}
            >
              Resume saved connection
            </button>
            <button
              type="button"
              className="button--secondary"
              disabled={blocked || working || !connection?.credentialId}
              aria-expanded={removeConfirm}
              onClick={() => setRemoveConfirm(true)}
            >
              Remove saved credential
            </button>
          </div>
          {removeConfirm ? (
            <div role="group" aria-label="Confirm provider credential removal">
              <p>
                This permanently removes the key from the operating-system
                vault. Activity history remains; using this provider again
                requires entering a key.
              </p>
              <div className="button-row">
                <button
                  type="button"
                  className="button--secondary"
                  onClick={() => setRemoveConfirm(false)}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  className="button--danger"
                  onClick={() => {
                    setRemoveConfirm(false);
                    void removeCredential();
                  }}
                >
                  Remove credential
                </button>
              </div>
            </div>
          ) : null}
        </form>
        <div className="data-card" aria-label="Synthetic AI connection test">
          <h4>Connection test</h4>
          <p>
            A test sends only a fixed synthetic JSON request to the selected
            provider. It does not send your resume or application materials.
          </p>
          <button
            type="button"
            className="button--secondary"
            disabled={blocked || working || connection?.mode !== "direct_api"}
            onClick={() => void reviewTest()}
          >
            Review synthetic test
          </button>
          {testPreview ? (
            <div role="group" aria-label="Confirm synthetic provider request">
              <p>
                {testPreview.provider} · {testPreview.model}
              </p>
              <p>
                Content sent: one fixed synthetic connection-test instruction
                and JSON value only—no resume, job, question, or user
                instruction content. Estimated preflight input: at most{" "}
                {testPreview.estimatedInputTokens} tokens.
              </p>
              <p>
                Conservative maximum reservation:{" "}
                {(testPreview.maximumCostMicros / 1_000_000).toFixed(4)}{" "}
                {testPreview.currency}. Actual provider billing may differ.
              </p>
              <p>
                The selected provider’s terms, retention and privacy practices,
                rate limits, direct charges, and account eligibility apply. Open
                Resume Toolkit cannot recover the provider account, refund
                charges, or guarantee provider availability.
              </p>
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
          ) : null}
          {testActive ? (
            <button
              type="button"
              className="button--secondary"
              onClick={() => void cancelTest()}
            >
              Cancel active test
            </button>
          ) : null}
          {testOutput ? <pre aria-live="polite">{testOutput}</pre> : null}
        </div>
        <div className="data-card" aria-label="Direct API spending caps">
          <h4>Spending caps</h4>
          <p>
            Caps are local estimated-cost guardrails for this credential only.
            They do not limit provider billing outside this installation.
          </p>
          <form onSubmit={(event) => void saveCap(event)}>
            <label>
              Calendar period
              <select
                value={capPeriod}
                disabled={
                  blocked || working || connection?.mode !== "direct_api"
                }
                onChange={(event) =>
                  setCapPeriod(event.target.value as CapPeriod)
                }
              >
                <option value="week">Week</option>
                <option value="month">Month</option>
                <option value="year">Year</option>
                <option value="all_time">All time</option>
              </select>
            </label>
            <label>
              Estimated cost limit
              <input
                inputMode="decimal"
                value={capAmount}
                disabled={
                  blocked || working || connection?.mode !== "direct_api"
                }
                onChange={(event) => setCapAmount(event.target.value)}
                aria-describedby="cap-disclosure"
              />
            </label>
            <p id="cap-disclosure">
              {caps.find((cap) => cap.period === capPeriod)?.currency ??
                "Catalog currency"}
              ; week boundaries begin Monday 00:00. New caps start at zero when
              activated in{" "}
              {caps.find((cap) => cap.period === capPeriod)?.timeZone ??
                Intl.DateTimeFormat().resolvedOptions().timeZone}{" "}
              and do not include earlier ORT or outside-provider calls. This
              credential identity is included; successful estimated cost and
              unresolved post-dispatch reservations count, while a cancelled
              never-dispatched reservation does not.
            </p>
            <button
              type="submit"
              disabled={
                blocked ||
                working ||
                connection?.mode !== "direct_api" ||
                !capAmount
              }
            >
              Save cap
            </button>
          </form>
          {caps.length ? (
            <div>
              {[...caps]
                .sort((a, b) => a.period.localeCompare(b.period))
                .map((cap) => {
                  const exposure =
                    cap.countedMicros +
                    cap.reservedMicros +
                    cap.unresolvedMicros;
                  const percent = Math.floor(
                    (exposure * 100) / cap.limitMicros,
                  );
                  const warning =
                    percent >= 100
                      ? "Blocked at 100%"
                      : percent >= 80
                        ? "Warning: at least 80% used"
                        : percent >= 50
                          ? "Warning: at least 50% used"
                          : null;
                  return (
                    <p key={cap.period}>
                      <strong>{cap.period.replace("_", " ")}</strong>:{" "}
                      {(exposure / 1_000_000).toFixed(4)} /{" "}
                      {(cap.limitMicros / 1_000_000).toFixed(4)} {cap.currency}{" "}
                      ({percent}%) · active{" "}
                      {new Date(cap.activatedAtUnixMs).toLocaleString()} ·{" "}
                      {cap.periodEndUnixMs
                        ? `resets ${new Date(cap.periodEndUnixMs).toLocaleString()} (${cap.timeZone})`
                        : `no automatic reset (${cap.timeZone})`}
                      {warning ? ` · ${warning}` : ""}
                    </p>
                  );
                })}
            </div>
          ) : (
            <p>No spending caps enabled.</p>
          )}
          {caps.some((cap) => cap.period === capPeriod) ? (
            <div className="button-row">
              <button
                type="button"
                className="button--secondary"
                disabled={blocked || working}
                onClick={() => setCapConfirm("disable")}
              >
                Disable selected cap
              </button>
              {capPeriod === "all_time" ? (
                <button
                  type="button"
                  className="button--secondary"
                  disabled={blocked || working}
                  onClick={() => setCapConfirm("reset")}
                >
                  Reset all-time baseline
                </button>
              ) : null}
            </div>
          ) : null}
          {capConfirm ? (
            <div role="group" aria-label={`Confirm ${capConfirm} cap`}>
              <p>
                {capConfirm === "disable"
                  ? "Disabling stops enforcement for this period. Counters and activity are not transferred to another credential."
                  : "Resetting sets counted, reserved, and unresolved all-time guardrail totals to zero. It does not clear activity or provider billing."}
              </p>
              <button
                type="button"
                className="button--secondary"
                onClick={() => setCapConfirm(null)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="button--danger"
                onClick={() => void capAction(capConfirm)}
              >
                Confirm {capConfirm}
              </button>
            </div>
          ) : null}
        </div>
        {notice ? (
          <p className="notice" role="status">
            {notice}
          </p>
        ) : null}
      </div>
      <div className="data-card">
        <p className="eyebrow">AI Monitoring</p>
        <h3>Local aggregate activity</h3>
        <div className="button-row" role="group" aria-label="Monitoring period">
          {(["Week", "Month", "Year", "All time"] as const).map((value) => (
            <button
              type="button"
              className="button--secondary"
              aria-pressed={period === value}
              onClick={() => setPeriod(value)}
              key={value}
            >
              {value}
            </button>
          ))}
        </div>
        {monitoringError ? (
          <p role="alert">AI Monitoring is unavailable.</p>
        ) : null}
        {!monitoring && !monitoringError ? (
          <p>Loading local activity…</p>
        ) : null}
        {monitoring?.attempts === 0 ? (
          <p>No recorded activity for {period.toLowerCase()}.</p>
        ) : null}
        {monitoring && monitoring.attempts > 0 ? (
          <div>
            <p>
              {monitoring.logicalOperations} operations · {monitoring.attempts}{" "}
              attempts
            </p>
            <p>
              {monitoring.usage.inputTokens} input tokens ·{" "}
              {monitoring.usage.outputTokens} output tokens
            </p>
            {Object.entries(monitoring.costByCurrencyMicros).map(
              ([currency, micros]) => (
                <p key={currency}>
                  Estimated recorded cost: {(micros / 1_000_000).toFixed(4)}{" "}
                  {currency}
                </p>
              ),
            )}
            {monitoring.partial ? (
              <p>
                Partial or unknown usage: {monitoring.unknownCount} attempts.
                Reserved exposure: {monitoring.unresolvedReservedMicros} micros;
                this is not zero spend.
              </p>
            ) : null}
            {monitoring.timeBuckets.length ? (
              <div aria-label="Token usage over time">
                <h4>Token usage over time</h4>
                {monitoring.timeBuckets.map((bucket) => {
                  const tokens =
                    bucket.usage.inputTokens +
                    (bucket.usage.cachedInputTokens ?? 0) +
                    (bucket.usage.cacheWriteTokens ?? 0) +
                    bucket.usage.outputTokens +
                    (bucket.usage.reasoningTokens ?? 0);
                  const maximum = Math.max(
                    1,
                    ...monitoring.timeBuckets.map(
                      (entry) =>
                        entry.usage.inputTokens +
                        (entry.usage.cachedInputTokens ?? 0) +
                        (entry.usage.cacheWriteTokens ?? 0) +
                        entry.usage.outputTokens +
                        (entry.usage.reasoningTokens ?? 0),
                    ),
                  );
                  return (
                    <div key={bucket.label}>
                      <span>
                        {bucket.label}: {tokens} tokens · {bucket.attempts}{" "}
                        attempts
                        {bucket.partial
                          ? ` · ${bucket.unknownCount} partial`
                          : ""}
                      </span>
                      <progress
                        aria-label={`Token usage on ${bucket.label}`}
                        value={tokens}
                        max={maximum}
                      >
                        {tokens}
                      </progress>
                      {Object.entries(bucket.costByCurrencyMicros).map(
                        ([currency, micros]) => {
                          const maximumCost = Math.max(
                            1,
                            ...monitoring.timeBuckets.map(
                              (entry) =>
                                entry.costByCurrencyMicros[currency] ?? 0,
                            ),
                          );
                          return (
                            <span key={currency}>
                              {" "}
                              · Estimated {(micros / 1_000_000).toFixed(4)}{" "}
                              {currency}
                              <progress
                                aria-label={`Estimated ${currency} cost on ${bucket.label}`}
                                value={micros}
                                max={maximumCost}
                              >
                                {micros}
                              </progress>
                            </span>
                          );
                        },
                      )}
                    </div>
                  );
                })}
              </div>
            ) : null}
            <p>
              Providers:{" "}
              {Object.entries(monitoring.byProvider)
                .map(([name, count]) => `${name} ${count}`)
                .join(" · ")}
            </p>
            <p>
              Models:{" "}
              {Object.entries(monitoring.byModel)
                .map(([name, count]) => `${name} ${count}`)
                .join(" · ")}
            </p>
            <p>
              Presets:{" "}
              {Object.entries(monitoring.byPreset)
                .map(([name, count]) => `${name} ${count}`)
                .join(" · ")}
            </p>
            <p>
              Operation types:{" "}
              {Object.entries(monitoring.byOperationType)
                .map(([name, count]) => `${name} ${count}`)
                .join(" · ")}
            </p>
            <p>
              Outcomes:{" "}
              {Object.entries(monitoring.byStatus)
                .map(([name, count]) => `${name} ${count}`)
                .join(" · ")}
            </p>
          </div>
        ) : null}
        <button
          type="button"
          className="button--secondary"
          disabled={!monitoring || blocked}
          onClick={() => void exportMonitoring()}
        >
          Export aggregate JSON
        </button>
        <button
          type="button"
          className="button--secondary"
          disabled={!monitoring || blocked}
          onClick={() => setClearConfirm(true)}
        >
          Clear selected activity
        </button>
        {clearConfirm ? (
          <div role="group" aria-label="Confirm AI activity clearing">
            <p>
              This removes completed activity for {period.toLowerCase()} from
              this profile. It does not reset spending caps or affect provider
              billing.
            </p>
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
              onClick={() => void clearMonitoring()}
            >
              Clear activity
            </button>
          </div>
        ) : null}
        <div className="data-card" aria-label="AI activity retention">
          <h4>Activity retention</h4>
          <p>
            Retain-until-cleared is the default. Applying an age limit
            permanently clears older content-free activity; it does not reset
            cap counters or delete provider-side records.
          </p>
          <label>
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
              <option value="retain_until_cleared">Retain until cleared</option>
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
        <p>
          Provider billing may include use outside this installation. Check your
          provider’s own usage and billing dashboard for authoritative charges
          and account-wide limits.
        </p>
        <p>
          Spending caps are disabled until explicitly configured. Unknown or
          unpriced usage is never treated as zero.
        </p>
      </div>
    </section>
  );
}
