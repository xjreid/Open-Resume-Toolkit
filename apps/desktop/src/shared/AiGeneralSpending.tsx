import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type Cap = {
  limitMicros: number;
  countedMicros: number;
  reservedMicros: number;
  unresolvedMicros: number;
  revision: number;
  currency: string;
};
type Settings = {
  cap: Cap | null;
  lifetimeSpendByCurrencyMicros: Record<string, number>;
  lifetimeSpendPartial?: boolean;
};
type Reply<T> = { ok: true; value: T } | { ok: false; error: { code: string } };
const format = (micros: number, currency = "USD") =>
  currency === "USD"
    ? `$${(micros / 1_000_000).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 })}`
    : `${(micros / 1_000_000).toLocaleString()} ${currency}`;

export function AiGeneralSpending({
  blocked,
  refreshRevision,
  setWorking,
  onChanged,
}: {
  blocked: boolean;
  refreshRevision: number;
  setWorking: (value: boolean) => void;
  onChanged: () => void;
}) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [editing, setEditing] = useState(false);
  const [amount, setAmount] = useState("");
  const [confirm, setConfirm] = useState(false);
  const [notice, setNotice] = useState("");
  useEffect(() => {
    let current = true;
    setLoading(true);
    setError(false);
    void invoke<Reply<Settings>>("load_ai_general_settings")
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
          setSettings(null);
          setError(true);
        }
        setLoading(false);
      })
      .catch(() => {
        if (current) {
          setSettings(null);
          setError(true);
          setLoading(false);
        }
      });
    return () => {
      current = false;
    };
  }, [refreshRevision]);
  const cap = settings?.cap;
  const currency = cap?.currency ?? "USD";
  const lifetime = settings?.lifetimeSpendByCurrencyMicros[currency] ?? 0;
  const exposure = cap
    ? cap.countedMicros + cap.reservedMicros + cap.unresolvedMicros
    : lifetime;
  const percent = cap ? Math.floor((exposure / cap.limitMicros) * 100) : 0;
  const disabled = blocked || loading || !settings;
  async function refresh(
    action: () => Promise<Reply<unknown>>,
    success: string,
  ) {
    setWorking(true);
    setNotice("");
    try {
      const response = await action();
      setNotice(
        response.ok
          ? success
          : response.error.code === "AI_CAP_CHANGED"
            ? "The limit changed. Refresh and try again."
            : "The limit could not be updated.",
      );
    } catch {
      setNotice("The limit could not be updated.");
    } finally {
      setWorking(false);
      setConfirm(false);
      onChanged();
    }
  }
  async function save() {
    if (disabled) return;
    if (!amount.trim()) {
      setEditing(false);
      if (cap) await remove();
      return;
    }
    const raw = Number(amount) * 1_000_000;
    const micros = Math.round(raw);
    if (
      !Number.isFinite(raw) ||
      !Number.isSafeInteger(micros) ||
      micros <= 0 ||
      Math.abs(raw - micros) > 0.000001
    ) {
      setNotice("Enter a positive limit with up to six decimal places.");
      return;
    }
    setEditing(false);
    if (micros === cap?.limitMicros) return;
    await refresh(
      () =>
        invoke("save_ai_general_cap", {
          limitMicros: micros,
          timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          expectedRevision: cap?.revision ?? null,
        }),
      "General limit saved.",
    );
  }
  async function remove() {
    await refresh(
      () => invoke("disable_ai_general_cap"),
      "General limit removed. Lifetime spend and Data are unchanged.",
    );
  }
  return (
    <section
      className="ai-panel ai-general-spending"
      aria-labelledby="ai-general-spending-title"
    >
      <div className="ai-panel-heading">
        <div>
          <h3 id="ai-general-spending-title">General spending</h3>
          <p>One limit shared across all API keys.</p>
        </div>
      </div>
      <div className="ai-general-budget-row" aria-busy={loading}>
        <div
          className="ai-key-spend"
          aria-label="All keys lifetime estimated spend"
        >
          <strong>
            {settings
              ? Object.entries(settings.lifetimeSpendByCurrencyMicros)
                  .map(([unit, total]) => format(total, unit))
                  .join(" · ") || format(0)
              : "—"}
          </strong>
          <span>Total spent across all keys</span>
          {settings?.lifetimeSpendPartial && (
            <small title="Some past usage could not be priced.">
              Partial total
            </small>
          )}
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
                  <strong>{settings ? format(exposure, currency) : "—"}</strong>
                  <span>/</span>
                  {editing ? (
                    <input
                      autoFocus
                      className="ai-key-limit-input"
                      aria-label="General spending limit"
                      inputMode="decimal"
                      placeholder="Unlimited"
                      value={amount}
                      disabled={disabled}
                      onChange={(event) => setAmount(event.target.value)}
                      onBlur={() => void save()}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === "Escape")
                          event.currentTarget.blur();
                      }}
                    />
                  ) : (
                    <button
                      type="button"
                      className="ai-key-limit-value"
                      aria-label="Edit general spending limit"
                      disabled={disabled}
                      onClick={() => {
                        setAmount(
                          cap ? String(cap.limitMicros / 1_000_000) : "",
                        );
                        setEditing(true);
                      }}
                    >
                      {settings
                        ? cap
                          ? format(cap.limitMicros, currency)
                          : "Unlimited"
                        : "—"}
                    </button>
                  )}
                </div>
                <span>{settings ? `${percent}%` : "—"}</span>
              </div>
              <progress
                aria-label="General spending cap used"
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
                  onClick={() => setConfirm(true)}
                >
                  Restart cap
                </button>
                <button
                  type="button"
                  className="button--quiet button--compact"
                  disabled={disabled || !cap}
                  onClick={() => void remove()}
                >
                  Remove limit
                </button>
              </div>
            </>
          )}
        </div>
      </div>
      {confirm && (
        <div
          className="ai-confirm"
          role="group"
          aria-label="Confirm restart general cap"
        >
          <p>
            Restart the shared cap usage at zero? Lifetime spend and activity in
            Data stay unchanged.
          </p>
          <div className="button-row">
            <button
              type="button"
              className="button--secondary"
              onClick={() => setConfirm(false)}
            >
              Cancel
            </button>
            <button
              type="button"
              disabled={disabled}
              onClick={() =>
                void refresh(
                  () => invoke("reset_ai_general_cap"),
                  "General cap usage restarted. Lifetime spend and Data are unchanged.",
                )
              }
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
    </section>
  );
}
