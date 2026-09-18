import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type ReactNode } from "react";
import type { Catalog, KeyRegistry, SavedKey } from "./AiWorkspace";

type Cap = {
  credentialId: string;
  period: "all_time";
  currency: string;
  timeZone: string;
  limitMicros: number;
  countedMicros: number;
  reservedMicros: number;
  unresolvedMicros: number;
  revision: number;
};
type Settings = {
  cap: Cap | null;
  lifetimeSpendByCurrencyMicros: Record<string, number>;
  lifetimeSpendPartial?: boolean;
};
type Reply<T> = { ok: true; value: T } | { ok: false; error: { code: string } };
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
  status,
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
  status: ReactNode;
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
  useEffect(() => {
    let current = true;
    setRefreshing(true);
    setError(false);
    void invoke<Reply<Settings>>("load_ai_key_settings", { credentialId: id })
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
      entry.preset === saved.preset &&
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
      const response = await invoke<Reply<Cap>>("save_ai_cap", {
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
      const response = await invoke<Reply<boolean>>(
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
  async function selectPreset(preset: SavedKey["preset"]) {
    if (disabled) return;
    setWorking(true);
    try {
      const response = await invoke<Reply<KeyRegistry>>("set_ai_key_preset", {
        request: { credentialId: id, preset },
      });
      if (response.ok) {
        onRegistry(response.value);
        setNotice("Preset saved.");
      } else
        setNotice(
          "This preset is unavailable. Finish any active test and try again.",
        );
    } catch {
      setNotice("The preset could not be saved.");
    } finally {
      setWorking(false);
    }
  }
  return (
    <>
      <div
        className="ai-key-card-layout"
        aria-label={`Key #${saved.identificationNumber} settings`}
        aria-busy={refreshing}
      >
        <div className="ai-key-card-info">
          <div className="ai-key-selection">{status}</div>
          <div className="ai-key-card-identity">
            {identity}
            <span className="ai-key-provider">
              {saved.provider === "openai"
                ? "OpenAI"
                : saved.provider === "anthropic"
                  ? "Anthropic"
                  : "Gemini"}{" "}
              · #{saved.identificationNumber}
            </span>
            <label className="ai-key-preset">
              <span className="visually-hidden">
                Model preset for key #{saved.identificationNumber}
              </span>
              <select
                value={saved.preset}
                disabled={disabled || !catalog || !entry}
                title={
                  entry
                    ? `${saved.preset}: ${entry.model}`
                    : "Verified model pricing unavailable"
                }
                onChange={(event) =>
                  void selectPreset(event.target.value as SavedKey["preset"])
                }
              >
                {(["economy", "balanced", "quality"] as const).map((preset) => {
                  const model = catalog?.entries.find(
                    (item) =>
                      item.provider ===
                        (saved.provider === "openai"
                          ? "open_ai"
                          : saved.provider) &&
                      item.preset === preset &&
                      !item.disabled,
                  );
                  return (
                    <option key={preset} value={preset} disabled={!model}>
                      {preset[0].toUpperCase() + preset.slice(1)}:{" "}
                      {model?.model ??
                        (preset === "balanced" ? "Unavailable" : "Coming soon")}
                    </option>
                  );
                })}
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
                      aria-label={`Key #${saved.identificationNumber} spending limit`}
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
                      aria-label={`Edit key #${saved.identificationNumber} spending limit`}
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
                aria-label={`Key #${saved.identificationNumber} spending cap used`}
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
          aria-label={`Confirm ${confirm} cap for key #${saved.identificationNumber}`}
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
