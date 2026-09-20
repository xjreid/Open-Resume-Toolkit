import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { dataKeyDescription } from "./AiDataKeyPicker";
import { formatKeyCreatedAt, ProviderLogo } from "./AiKeyPresentation";
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
  onConfirm: (keys: string[], months: ActivityMonth[]) => void;
}) {
  const [step, setStep] = useState<"key" | "months">("key");
  const [selectedKeys, setSelectedKeys] = useState<string[]>([]);
  const [months, setMonths] = useState<ActivityMonth[]>([]);
  const [selectedMonths, setSelectedMonths] = useState<string[]>([]);
  const [monthsLoading, setMonthsLoading] = useState(false);
  const [monthsError, setMonthsError] = useState(false);
  const firstChoice = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!action) return;
    setStep("key");
    setSelectedKeys([]);
    setMonths([]);
    setSelectedMonths([]);
    setMonthsError(false);
    firstChoice.current?.focus();
  }, [action]);

  useEffect(() => {
    if (!action || step !== "months" || selectedKeys.length === 0) return;
    let current = true;
    setMonthsLoading(true);
    setMonthsError(false);
    setMonths([]);
    setSelectedMonths([]);
    const allKeysSelected = selectedKeys.includes("");
    const scopes: Array<string | null> = allKeysSelected
      ? [null]
      : selectedKeys;
    void Promise.all(
      scopes.map((credentialId) =>
        invoke<MonthResponse>("load_ai_monitoring", {
          fromUnixMs: 0,
          toUnixMs: Date.now() + 1,
          timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          bucketSize: "month",
          credentialId,
        }),
      ),
    )
      .then((responses) => {
        if (!current) return;
        if (responses.some((response) => !response.ok)) {
          setMonthsError(true);
          return;
        }
        const attemptsByMonth = new Map<string, number>();
        for (const response of responses) {
          if (!response.ok) continue;
          for (const bucket of response.value.timeBuckets) {
            attemptsByMonth.set(
              bucket.label,
              (attemptsByMonth.get(bucket.label) ?? 0) + bucket.attempts,
            );
          }
        }
        setMonths(
          [...attemptsByMonth]
            .map(([label, attempts]) => monthRange(label, attempts))
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
  }, [action, selectedKeys, step]);

  if (!action) return null;

  const isClear = action === "clear";
  const allKeysSelected = selectedKeys.includes("");
  const selectedKeyRecords = keys.filter((key) =>
    selectedKeys.includes(key.credentialId),
  );
  const selectedDescription = allKeysSelected
    ? "All keys"
    : selectedKeyRecords.length === 1
      ? dataKeyDescription(selectedKeyRecords[0], catalog).title
      : `${selectedKeyRecords.length} keys`;
  const allSelected =
    months.length > 0 && selectedMonths.length === months.length;

  function toggleMonth(label: string) {
    setSelectedMonths((current) =>
      current.includes(label)
        ? current.filter((item) => item !== label)
        : [...current, label],
    );
  }

  function toggleKey(credentialId: string) {
    setSelectedKeys((current) =>
      current.includes(credentialId)
        ? current.filter((item) => item !== credentialId)
        : [...current, credentialId],
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
              <span>Choose one or more API keys, or select All keys.</span>
            </div>
            <div className="ai-data-action-options">
              <button
                ref={firstChoice}
                type="button"
                className={`ai-data-key-option ai-data-key-option--all${allKeysSelected ? " ai-data-key-option--selected" : ""}`}
                aria-label={`${allKeysSelected ? "Deselect" : "Select"} all keys`}
                aria-pressed={allKeysSelected}
                onClick={() => toggleKey("")}
              >
                <span className="ai-data-key-option-copy">
                  <strong>All keys</strong>
                  <small>Activity from every provider and model</small>
                </span>
                {allKeysSelected && <span aria-hidden="true">✓</span>}
              </button>
              {keys.map((key) => {
                const description = dataKeyDescription(key, catalog);
                const selected = selectedKeys.includes(key.credentialId);
                return (
                  <button
                    type="button"
                    key={key.credentialId}
                    className={`ai-data-key-option${selected ? " ai-data-key-option--selected" : ""}`}
                    aria-label={`${selected ? "Deselect" : "Select"} ${description.title}`}
                    aria-pressed={selected}
                    onClick={() => toggleKey(key.credentialId)}
                  >
                    <span className="ai-data-key-option-logo">
                      <ProviderLogo provider={key.provider} />
                    </span>
                    <span className="ai-data-key-option-copy">
                      <strong>{description.title}</strong>
                      <small>{description.detail}</small>
                    </span>
                    <span className="ai-data-key-option-meta">
                      <small>{formatKeyCreatedAt(key.createdAt)}</small>
                      {selected && <span aria-hidden="true">✓</span>}
                    </span>
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
                disabled={disabled || selectedKeys.length === 0}
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
                  {selectedDescription} · Only months with activity
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
                  onConfirm(
                    selectedKeys,
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
