import { invokeDesktop as invoke } from "./desktop-client";
import type * as Wire from "@ort/contracts/wire";
import { useEffect, useState, type ReactNode } from "react";
import {
  keyDisplayName,
  providerName,
  ProviderLogo,
} from "./AiKeyPresentation";
import type { Catalog, KeyRegistry, SavedKey } from "./AiWorkspace";

type Cap = Wire.AiCapPolicySummary;
type Settings = Wire.AiKeySettings;
const number = (micros: number) =>
  (micros / 1_000_000).toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  });
const money = (micros: number, currency = "USD") =>
  currency === "USD" ? `$${number(micros)}` : `${number(micros)} ${currency}`;

export function AiKeyCustomization({
  saved,
  catalog,
  blocked,
  refreshRevision,
  setWorking,
  onRegistry,
  onChanged,
  identity,
  actions,
}: {
  saved: SavedKey;
  catalog: Catalog | null;
  blocked: boolean;
  refreshRevision: number;
  setWorking: (value: boolean) => void;
  onRegistry: (value: KeyRegistry) => void;
  onChanged: () => void;
  identity: ReactNode;
  actions: ReactNode;
}) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [amount, setAmount] = useState("");
  const [editingLimit, setEditingLimit] = useState(false);
  const [refreshing, setRefreshing] = useState(true);
  const [error, setError] = useState(false);
  const [notice, setNotice] = useState("");
  const [confirm, setConfirm] = useState<"reset" | "disable" | null>(null);
  const id = saved.credentialId;
  const label = keyDisplayName(saved);
  useEffect(() => {
    let current = true;
    setRefreshing(true);
    setError(false);
    void invoke("load_ai_key_settings", { credentialId: id })
      .then((response) => {
        if (!current) return;
        if (response.ok) {
          setSettings(response.value);
          setAmount(
            response.value.cap
              ? String(response.value.cap.limitMicros / 1_000_000)
              : "",
          );
        } else {
          setError(true);
          setSettings(null);
        }
        setRefreshing(false);
      })
      .catch(() => {
        if (current) {
          setError(true);
          setSettings(null);
          setRefreshing(false);
        }
      });
    return () => {
      current = false;
    };
  }, [id, refreshRevision]);

  const entry = catalog?.entries.find(
    (entry) =>
      entry.provider ===
        (saved.provider === "openai" ? "open_ai" : saved.provider) &&
      entry.model === saved.model &&
      !entry.disabled,
  );
  const cap = settings?.cap;
  const currency = cap?.currency ?? entry?.currency ?? "USD";
  const exposure = cap
    ? cap.countedMicros + cap.reservedMicros + cap.unresolvedMicros
    : (settings?.lifetimeSpendByCurrencyMicros[currency] ?? 0);
  const percent = cap ? Math.floor((exposure / cap.limitMicros) * 100) : 0;
  const disabled = blocked || refreshing || !settings || saved.cleanupRequired;

  async function saveCap() {
    if (disabled) return;
    if (!amount.trim()) {
      setEditingLimit(false);
      if (cap) await capAction("disable");
      return;
    }
    const limit = Number(amount) * 1_000_000;
    const micros = Math.round(limit);
    if (
      !Number.isFinite(limit) ||
      !Number.isSafeInteger(micros) ||
      micros <= 0 ||
      Math.abs(limit - micros) > 0.000001
    ) {
      setNotice("Enter a positive limit with up to six decimal places.");
      return;
    }
    setEditingLimit(false);
    if (micros === cap?.limitMicros) return;
    setWorking(true);
    try {
      const response = await invoke("save_ai_cap", {
        request: {
          credentialId: id,
          period: "all_time",
          limitMicros: micros,
          timeZone:
            cap?.timeZone ?? Intl.DateTimeFormat().resolvedOptions().timeZone,
          expectedRevision: cap?.revision ?? null,
        },
      });
      if (response.ok) {
        setNotice("Limit saved.");
        onChanged();
      } else {
        setNotice(
          response.error.code === "AI_CAP_CHANGED"
            ? "The limit changed. Refresh and try again."
            : "The limit could not be saved. Finish any active test and try again.",
        );
        onChanged();
      }
    } catch {
      setNotice("The limit could not be saved.");
      onChanged();
    } finally {
      setWorking(false);
    }
  }
  async function capAction(action: "reset" | "disable") {
    if (disabled) return;
    setConfirm(null);
    setWorking(true);
    try {
      const response = await invoke(
        action === "reset" ? "reset_ai_cap" : "disable_ai_cap",
        { request: { credentialId: id, period: "all_time" } },
      );
      setNotice(
        response.ok
          ? action === "reset"
            ? "Cap usage restarted. Lifetime spend and Data are unchanged."
            : "Limit removed. Lifetime spend and Data are unchanged."
          : "Finish any active test and try again.",
      );
      onChanged();
    } catch {
      setNotice("The limit could not be updated.");
      onChanged();
    } finally {
      setWorking(false);
    }
  }
  async function selectModel(model: string) {
    if (disabled) return;
    setWorking(true);
    try {
      const response = await invoke("set_ai_key_model", {
        request: { credentialId: id, model },
      });
      if (response.ok) {
        onRegistry(response.value);
        setNotice("Model saved.");
      } else
        setNotice(
          "This model is unavailable. Finish any active test and try again.",
        );
    } catch {
      setNotice("The model could not be saved.");
    } finally {
      setWorking(false);
    }
  }
  return (
    <>
      <div
        className="ai-key-card-layout"
        aria-label={`${label} settings`}
        aria-busy={refreshing}
      >
        <div className="ai-key-card-info">
          <div className="ai-key-card-identity">
            <div className="ai-key-identity-line">
              <span className="ai-key-provider-logo">
                <ProviderLogo provider={saved.provider} />
              </span>
              <span className="ai-key-provider-name">
                {providerName(saved.provider)}
              </span>
              <span className="ai-key-identity-separator" aria-hidden="true">
                ·
              </span>
              {identity}
            </div>
            <label className="ai-key-model">
              <span className="visually-hidden">Model for {label}</span>
              <select
                value={saved.model}
                disabled={disabled || !catalog}
                title={saved.model || "Verified model pricing unavailable"}
                onChange={(event) => void selectModel(event.target.value)}
              >
                {!entry && (
                  <option value={saved.model} disabled>
                    {saved.model || "Model unavailable"}
                  </option>
                )}
                {[
                  ...new Map(
                    (catalog?.entries ?? [])
                      .filter(
                        (item) =>
                          item.provider ===
                            (saved.provider === "openai"
                              ? "open_ai"
                              : saved.provider) &&
                          !item.disabled &&
                          item.operations.includes("credential_test"),
                      )
                      .map((item) => [item.model, item]),
                  ).values(),
                ].map((item) => (
                  <option key={item.model} value={item.model}>
                    {item.model}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="ai-key-spend" aria-label="Lifetime estimated spend">
            <strong>
              {settings
                ? Object.entries(settings.lifetimeSpendByCurrencyMicros)
                    .map(([currency, total]) => money(total, currency))
                    .join(" · ") || money(0, currency)
                : "—"}
            </strong>
            <span>Total spent</span>
            {settings?.lifetimeSpendPartial && (
              <small title="Some past usage could not be priced.">
                Partial total
              </small>
            )}
          </div>
        </div>
        <div className="ai-key-card-budget">
          {error ? (
            <p role="alert">
              Spending data unavailable.{" "}
              <button
                type="button"
                className="button--quiet button--compact"
                disabled={blocked}
                onClick={onChanged}
              >
                Retry
              </button>
            </p>
          ) : (
            <>
              <div className="ai-key-cap-totals">
                <div className="ai-key-cap-numbers">
                  <strong>{settings ? money(exposure, currency) : "—"}</strong>
                  <span>/</span>
                  {editingLimit ? (
                    <input
                      autoFocus
                      className="ai-key-limit-input"
                      aria-label={`${label} spending limit`}
                      inputMode="decimal"
                      placeholder="Unlimited"
                      value={amount}
                      disabled={disabled}
                      onChange={(event) => setAmount(event.target.value)}
                      onBlur={() => void saveCap()}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === "Escape")
                          event.currentTarget.blur();
                      }}
                    />
                  ) : (
                    <button
                      type="button"
                      className="ai-key-limit-value"
                      aria-label={`Edit ${label} spending limit`}
                      title="Edit spending limit"
                      disabled={disabled}
                      onClick={() => {
                        setAmount(
                          cap ? String(cap.limitMicros / 1_000_000) : "",
                        );
                        setEditingLimit(true);
                      }}
                    >
                      {settings
                        ? cap
                          ? money(cap.limitMicros, currency)
                          : "Unlimited"
                        : "—"}
                    </button>
                  )}
                </div>
                <span>{settings ? `${percent}%` : "—"}</span>
              </div>
              <progress
                aria-label={`${label} spending cap used`}
                value={
                  settings
                    ? cap
                      ? Math.min(exposure, cap.limitMicros)
                      : 0
                    : undefined
                }
                max={cap?.limitMicros ?? 1}
              />
              <div className="button-row ai-key-cap-actions">
                <button
                  type="button"
                  className="button--quiet button--compact"
                  disabled={disabled || !cap}
                  onClick={() => setConfirm("reset")}
                >
                  Restart cap
                </button>
                <button
                  type="button"
                  className="button--quiet button--compact"
                  disabled={disabled || !cap}
                  onClick={() => void capAction("disable")}
                >
                  Remove limit
                </button>
              </div>
              {cap && cap.reservedMicros + cap.unresolvedMicros > 0 && (
                <p className="ai-help">
                  Includes{" "}
                  {money(cap.reservedMicros + cap.unresolvedMicros, currency)}{" "}
                  pending or unknown spend.
                </p>
              )}
            </>
          )}
        </div>
        <div className="button-row ai-key-actions">{actions}</div>
      </div>
      {confirm && !saved.paused && (
        <div
          className="ai-confirm"
          role="group"
          aria-label={`Confirm ${confirm} cap for ${label}`}
        >
          <p>
            Restart this key’s cap usage at zero? Lifetime spend and activity in
            Data stay unchanged.
          </p>
          <div className="button-row">
            <button
              type="button"
              className="button--secondary"
              onClick={() => setConfirm(null)}
            >
              Cancel
            </button>
            <button
              type="button"
              disabled={disabled}
              onClick={() => void capAction(confirm)}
            >
              Confirm restart
            </button>
          </div>
        </div>
      )}
      {notice && (
        <p className="ai-help ai-key-card-notice" role="status">
          {notice}
        </p>
      )}
    </>
  );
}
