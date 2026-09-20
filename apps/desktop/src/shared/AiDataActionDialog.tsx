import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { dataKeyDescription } from "./AiDataKeyPicker";
import type { Catalog, SavedKey } from "./AiWorkspace";

export type ActivityPeriod = "Week" | "Month" | "Year" | "All time";
export type DataAction = "export" | "clear";
export type ActivityMonth = {
  label: string;
  fromUnixMs: number;
  toUnixMs: number;
  attempts: number;
};

type MonthResponse =
  | {
      ok: true;
      value: { timeBuckets: Array<{ label: string; attempts: number }> };
    }
  | { ok: false };

function monthRange(label: string, attempts: number): ActivityMonth | null {
  const match = /^(\d{4})-(\d{2})$/.exec(label);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]) - 1;
  if (month < 0 || month > 11) return null;
  return {
    label,
    fromUnixMs: new Date(year, month, 1).getTime(),
    toUnixMs: new Date(year, month + 1, 1).getTime(),
    attempts,
  };
}

function monthName(label: string) {
  const [year, month] = label.split("-").map(Number);
  return new Intl.DateTimeFormat(undefined, {
    month: "long",
    year: "numeric",
  }).format(new Date(year, month - 1, 1));
}

export function AiDataActionDialog({
  action,
  keys,
  catalog,
  disabled,
  onCancel,
  onConfirm,
}: {
  action: DataAction | null;
  keys: SavedKey[];
  catalog: Catalog | null;
  disabled: boolean;
  onCancel: () => void;
  onConfirm: (key: string, months: ActivityMonth[]) => void;
}) {
  const [step, setStep] = useState<"key" | "months">("key");
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [months, setMonths] = useState<ActivityMonth[]>([]);
  const [selectedMonths, setSelectedMonths] = useState<string[]>([]);
  const [monthsLoading, setMonthsLoading] = useState(false);
  const [monthsError, setMonthsError] = useState(false);
  const firstChoice = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!action) return;
    setStep("key");
    setSelectedKey(null);
    setMonths([]);
    setSelectedMonths([]);
    setMonthsError(false);
    firstChoice.current?.focus();
  }, [action]);

  useEffect(() => {
    if (!action || step !== "months" || selectedKey === null) return;
    let current = true;
    setMonthsLoading(true);
    setMonthsError(false);
    setMonths([]);
    setSelectedMonths([]);
    void invoke<MonthResponse>("load_ai_monitoring", {
      fromUnixMs: 0,
      toUnixMs: Date.now() + 1,
      timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      bucketSize: "month",
      credentialId: selectedKey || null,
    })
      .then((response) => {
        if (!current) return;
        if (!response.ok) {
          setMonthsError(true);
          return;
        }
        setMonths(
          response.value.timeBuckets
            .map((bucket) => monthRange(bucket.label, bucket.attempts))
            .filter((month): month is ActivityMonth => month !== null)
            .sort((a, b) => b.label.localeCompare(a.label)),
        );
      })
      .catch(() => {
        if (current) setMonthsError(true);
      })
      .finally(() => {
        if (current) setMonthsLoading(false);
      });
    return () => {
      current = false;
    };
  }, [action, selectedKey, step]);

  if (!action) return null;

  const isClear = action === "clear";
  const selectedDescription = dataKeyDescription(
    keys.find((key) => key.credentialId === selectedKey),
    catalog,
  );
  const allSelected =
    months.length > 0 && selectedMonths.length === months.length;

  function toggleMonth(label: string) {
    setSelectedMonths((current) =>
      current.includes(label)
        ? current.filter((item) => item !== label)
        : [...current, label],
    );
  }

  return (
    <div
      className="ai-data-action-backdrop"
      role="presentation"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget && !disabled) onCancel();
      }}
    >
      <div
        className="ai-data-action-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="ai-data-action-title"
        onKeyDown={(event) => {
          if (event.key === "Escape" && !disabled) onCancel();
        }}
      >
        <div className="ai-data-action-heading">
          <div>
            <span>Step {step === "key" ? "1" : "2"} of 2</span>
            <h3 id="ai-data-action-title">
              {isClear ? "Clear activity" : "Export activity"}
            </h3>
          </div>
          <button
            type="button"
            className="button--quiet button--compact"
            disabled={disabled}
            aria-label="Close"
            onClick={onCancel}
          >
            Close
          </button>
        </div>

        {step === "key" ? (
          <>
            <div className="ai-data-action-copy">
              <strong>Which activity?</strong>
              <span>Choose all keys or one API key.</span>
            </div>
            <div className="ai-data-action-options">
              <button
                ref={firstChoice}
                type="button"
                className={`ai-data-key-option${selectedKey === "" ? " ai-data-key-option--selected" : ""}`}
                aria-label="Select all keys"
                aria-pressed={selectedKey === ""}
                onClick={() => setSelectedKey("")}
              >
                <span className="ai-data-key-option-icon">∑</span>
                <span>
                  <strong>All keys</strong>
                  <small>Activity from every provider and model</small>
                </span>
                {selectedKey === "" && <span aria-hidden="true">✓</span>}
              </button>
              {keys.map((key) => {
                const description = dataKeyDescription(key, catalog);
                return (
                  <button
                    type="button"
                    key={key.credentialId}
                    className={`ai-data-key-option${selectedKey === key.credentialId ? " ai-data-key-option--selected" : ""}`}
                    aria-label={`Select ${description.title}`}
                    aria-pressed={selectedKey === key.credentialId}
                    onClick={() => setSelectedKey(key.credentialId)}
                  >
                    <span className="ai-data-key-option-icon">
                      {key.identificationNumber || "–"}
                    </span>
                    <span>
                      <strong>{description.title}</strong>
                      <small>{description.detail}</small>
                    </span>
                    {selectedKey === key.credentialId && (
                      <span aria-hidden="true">✓</span>
                    )}
                  </button>
                );
              })}
            </div>
            <div className="ai-data-action-footer">
              <button
                type="button"
                className="button--secondary"
                disabled={disabled}
                onClick={onCancel}
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={disabled || selectedKey === null}
                onClick={() => setStep("months")}
              >
                Continue
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="ai-data-action-copy ai-data-action-copy--months">
              <div>
                <strong>Select months</strong>
                <span>
                  {selectedDescription.title} · Only months with activity
                </span>
              </div>
              {months.length > 0 && (
                <button
                  type="button"
                  className="button--quiet button--compact"
                  onClick={() =>
                    setSelectedMonths(
                      allSelected ? [] : months.map((month) => month.label),
                    )
                  }
                >
                  {allSelected ? "Deselect all" : "Select all"}
                </button>
              )}
            </div>
            {monthsLoading ? (
              <p className="ai-data-months-state" role="status">
                Finding months with activity…
              </p>
            ) : monthsError ? (
              <p className="ai-data-months-state" role="alert">
                Activity months could not be loaded. Go back and try again.
              </p>
            ) : months.length === 0 ? (
              <p className="ai-data-months-state">
                No retained activity is available for this selection.
              </p>
            ) : (
              <div className="ai-data-month-options">
                {months.map((month) => {
                  const selected = selectedMonths.includes(month.label);
                  return (
                    <button
                      type="button"
                      key={month.label}
                      className={
                        selected
                          ? "ai-data-month-option ai-data-month-option--selected"
                          : "ai-data-month-option"
                      }
                      aria-label={`Select ${monthName(month.label)}`}
                      aria-pressed={selected}
                      onClick={() => toggleMonth(month.label)}
                    >
                      <span className="ai-data-month-check" aria-hidden="true">
                        {selected ? "✓" : ""}
                      </span>
                      <span>
                        <strong>{monthName(month.label)}</strong>
                        <small>
                          {month.attempts.toLocaleString()} provider{" "}
                          {month.attempts === 1 ? "attempt" : "attempts"}
                        </small>
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
            {isClear && (
              <p className="ai-data-action-warning">
                This removes only the selected graph activity. Lifetime spend
                and spending-cap progress on My Keys are not changed.
              </p>
            )}
            <div className="ai-data-action-footer">
              <button
                type="button"
                className="button--secondary"
                disabled={disabled}
                onClick={() => setStep("key")}
              >
                Back
              </button>
              <button
                type="button"
                className={isClear ? "button--danger" : undefined}
                disabled={disabled || selectedMonths.length === 0}
                onClick={() => {
                  if (selectedKey === null) return;
                  onConfirm(
                    selectedKey,
                    months.filter((month) =>
                      selectedMonths.includes(month.label),
                    ),
                  );
                }}
              >
                {isClear
                  ? `Clear ${selectedMonths.length || ""} ${selectedMonths.length === 1 ? "month" : "months"}`
                  : `Export ${selectedMonths.length || ""} ${selectedMonths.length === 1 ? "month" : "months"}`}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
