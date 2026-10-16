import { useEffect, useState } from "react";
import type * as Wire from "@ort/contracts/wire";
import { invokeDesktop as invoke } from "./desktop-client";
import { CodexPageIcon } from "./CodexPageIcon";
import {
  planErrorMessage,
  quotaWindowLabel,
  REASONING_EFFORT_LABELS,
} from "./chatgpt-plan-presentation";

const efforts = Object.entries(REASONING_EFFORT_LABELS) as [
  Wire.ReasoningEffort,
  string,
][];

export function CodexAccountSettings({
  status,
  locked,
  onSave,
  onNotice,
  onSignOut,
  signOutDisabled,
}: {
  status: Wire.PlanStatus;
  locked: boolean;
  onSave: (change: Partial<Wire.PlanSettings>) => void;
  onNotice: (notice: string) => void;
  onSignOut: () => void;
  signOutDisabled: boolean;
}) {
  const { settings } = status;
  const multipleLimits =
    new Set(status.quota?.windows.map((window) => window.limitId)).size > 1;
  const planName = status.accountPlan
    ? `${status.accountPlan.charAt(0).toUpperCase()}${status.accountPlan.slice(1)} plan`
    : "Plan unavailable";
  const model = status.models.find(
    (candidate) => candidate.id === settings.model,
  );
  const [reserve, setReserve] = useState(String(settings.reservePercent));
  useEffect(
    () => setReserve(String(settings.reservePercent)),
    [settings.reservePercent],
  );

  return (
    <>
      <section
        className="plan-account-settings"
        aria-labelledby="plan-model-title"
      >
        <h3 id="plan-model-title" className="visually-hidden">
          AI settings
        </h3>
        <div className="plan-model-controls">
          <label>
            Model
            <select
              aria-label="Codex model"
              value={settings.model ?? ""}
              disabled={
                locked ||
                !status.models.some((candidate) => candidate.supported)
              }
              onChange={(event) => {
                const next = status.models.find(
                  (candidate) => candidate.id === event.target.value,
                )!;
                onSave({
                  model: next.id,
                  reasoning: next.reasoningEfforts.includes(settings.reasoning)
                    ? settings.reasoning
                    : next.reasoningEfforts[0],
                });
              }}
            >
              <option value="" disabled>
                Choose a supported model
              </option>
              {status.models.map((candidate) => (
                <option
                  key={candidate.id}
                  value={candidate.id}
                  disabled={!candidate.supported}
                >
                  {candidate.name}
                  {candidate.supported ? "" : " — Unavailable"}
                </option>
              ))}
            </select>
          </label>
          <label>
            Reasoning
            <select
              aria-label="Codex reasoning"
              value={settings.reasoning}
              disabled={locked || !model?.supported}
              onChange={(event) =>
                onSave({
                  reasoning: event.target.value as Wire.ReasoningEffort,
                })
              }
            >
              {efforts.map(([value, label]) => (
                <option
                  value={value}
                  key={value}
                  disabled={!model?.reasoningEfforts.includes(value)}
                >
                  {label}
                </option>
              ))}
            </select>
          </label>
        </div>
        {model?.explanation && <p className="ai-help">{model.explanation}</p>}
        {!status.models.some((candidate) => candidate.supported) && (
          <p className="ai-help" role="status">
            No supported models available. Refresh to check again.
          </p>
        )}
        {status.operationActive && (
          <div className="plan-operation" role="status">
            <span>AI work in progress. Settings are paused.</span>
            <button
              type="button"
              className="button--secondary"
              onClick={() => void invoke("stop_chatgpt_plan")}
            >
              Stop AI work
            </button>
          </div>
        )}
      </section>
      <section
        className="plan-usage-settings"
        aria-labelledby="plan-usage-title"
      >
        <div className="plan-account-heading">
          <div className="plan-account-identity">
            <h3 id="plan-usage-title">Account</h3>
            <span className="plan-account-plan">{planName}</span>
          </div>
          <button
            type="button"
            className="button--danger plan-sign-out"
            disabled={signOutDisabled}
            onClick={onSignOut}
          >
            <CodexPageIcon name="logout" />
            Sign out
          </button>
        </div>
        <div className="plan-usage-layout">
          <div className="plan-usage">
            <h4>Usage limits</h4>
            {status.quota?.windows.length ? (
              <ul className="plan-quota-list">
                {status.quota.windows.map((window) => (
                  <li key={`${window.limitId}-${window.window}`}>
                    {multipleLimits && <p className="ai-help">{window.name}</p>}
                    <div className="plan-quota-heading">
                      <strong>
                        {window.windowDurationMinutes === 300
                          ? "5-hour limit"
                          : window.windowDurationMinutes === 10080
                            ? "Weekly limit"
                            : quotaWindowLabel(window)}
                      </strong>
                      <span>
                        {window.remainingPercent.toLocaleString(undefined, {
                          maximumFractionDigits: 1,
                        })}
                        % remaining
                      </span>
                    </div>
                    <meter
                      min="0"
                      max="100"
                      value={window.remainingPercent}
                      aria-label={`${window.name} remaining usage`}
                    />
                    <div className="plan-quota-details">
                      <span>
                        {window.resetsAt
                          ? `Resets ${new Date(window.resetsAt * 1000).toLocaleString(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" })}`
                          : "Reset time not supplied"}
                      </span>
                    </div>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="plan-usage-unavailable">
                <strong>Usage unavailable</strong>
                <span>
                  {settings.reserveEnabled
                    ? "New AI work is paused until usage is available."
                    : "Your reserve is off."}{" "}
                  Refresh to check again.
                </span>
              </p>
            )}
            <p className="ai-help plan-usage-note">
              Account-wide usage. Updates every 30 seconds.
            </p>
          </div>
          <div className="plan-reserve" aria-labelledby="plan-reserve-title">
            <div className="plan-reserve-heading">
              <h4 id="plan-reserve-title">Usage reserve</h4>
              <label className="plan-reserve-toggle">
                <input
                  type="checkbox"
                  role="switch"
                  checked={settings.reserveEnabled}
                  disabled={locked}
                  onChange={(event) =>
                    onSave({ reserveEnabled: event.target.checked })
                  }
                />
                <span className="visually-hidden">Keep a usage reserve</span>
              </label>
            </div>
            <p className="ai-help">Save some usage for other apps.</p>
            <form
              onSubmit={(event) => {
                event.preventDefault();
                const value = Number(reserve);
                if (!Number.isInteger(value) || value < 1 || value > 100) {
                  onNotice(planErrorMessage("PLAN_SETTINGS_INVALID"));
                  return;
                }
                onSave({ reservePercent: value });
              }}
            >
              <label>
                Minimum remaining
                <span className="plan-reserve-input">
                  <input
                    type="number"
                    min="1"
                    max="100"
                    step="1"
                    inputMode="numeric"
                    aria-label="Usage reserve percentage"
                    aria-describedby="plan-reserve-help"
                    value={reserve}
                    disabled={locked || !settings.reserveEnabled}
                    onChange={(event) => setReserve(event.target.value)}
                  />
                  <span aria-hidden="true">%</span>
                </span>
              </label>
              <button
                type="submit"
                className="button--secondary"
                disabled={
                  locked ||
                  !settings.reserveEnabled ||
                  reserve === String(settings.reservePercent)
                }
              >
                Save reserve
              </button>
            </form>
            <p className="ai-help" id="plan-reserve-help">
              {settings.reserveEnabled ? (
                <>
                  ORT pauses new AI work below {settings.reservePercent}%
                  remaining.
                </>
              ) : (
                "Reserve is off."
              )}
            </p>
            <details className="plan-details plan-reserve-details">
              <summary>How the reserve works</summary>
              <div className="plan-details-content">
                <p>
                  At exactly {settings.reservePercent}% remaining, dispatch is
                  allowed. Checked before every pass. Ongoing requests and usage
                  elsewhere can cross the reserve.
                </p>
                <p>
                  When enabled, your reserve blocks new AI work until Codex
                  reports valid usage. Usage includes other apps on your
                  account.
                </p>
              </div>
            </details>
          </div>
        </div>
      </section>
      <p className="ai-help plan-data-note">
        AI work sends your resume, job description, and instructions to OpenAI.
      </p>
    </>
  );
}
