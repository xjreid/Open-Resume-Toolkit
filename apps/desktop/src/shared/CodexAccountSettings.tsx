import { useEffect, useState } from "react";
import type * as Wire from "@ort/contracts/wire";
import { invokeDesktop as invoke } from "./desktop-client";
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
  signOutDisabled,
  onSave,
  onSignOut,
  onNotice,
}: {
  status: Wire.PlanStatus;
  locked: boolean;
  signOutDisabled: boolean;
  onSave: (change: Partial<Wire.PlanSettings>) => void;
  onSignOut: () => void;
  onNotice: (notice: string) => void;
}) {
  const { settings } = status;
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
        className="ai-panel plan-account-settings"
        aria-labelledby="plan-account-title"
      >
        <div className="plan-account-heading">
          <div>
            <h3 id="plan-account-title">ChatGPT account</h3>
            <p className="plan-account-plan">
              {status.accountPlan
                ? `${status.accountPlan.charAt(0).toUpperCase()}${status.accountPlan.slice(1)} plan`
                : "Plan unavailable"}
            </p>
          </div>
          <button
            type="button"
            className="button--secondary"
            disabled={signOutDisabled}
            onClick={onSignOut}
          >
            Sign out
          </button>
        </div>
        <div className="plan-model-controls">
          <label>
            Model
            <select
              aria-label="Codex model"
              value={settings.model ?? ""}
              disabled={locked}
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
        <p className="ai-help plan-data-note">
          Your resume, job description, and instructions are sent to OpenAI
          under your account’s plan and privacy terms. Account access is checked
          when a request runs.
        </p>
        {status.operationActive && (
          <div className="plan-operation" role="status">
            <span>
              AI work is in progress. Model and reserve controls are locked.
            </span>
            <button
              type="button"
              className="button--secondary"
              onClick={() => void invoke("stop_chatgpt_plan")}
            >
              Stop AI work
            </button>
          </div>
        )}
        <p className="ai-help plan-session-note">
          Sign-in stays in memory only. Sign in again after quitting ORT,
          switching profiles, or restarting the server. Signing out ends this
          account session; the server stays enabled.
        </p>
      </section>
      <section
        className="ai-panel plan-usage-settings"
        aria-labelledby="plan-usage-title"
      >
        <div className="plan-usage-layout">
          <div className="plan-usage">
            <h3 id="plan-usage-title">Account usage</h3>
            <p className="ai-help">
              Remaining usage across your ChatGPT account, including other apps.
            </p>
            {status.quota?.windows.length ? (
              <ul className="plan-quota-list">
                {status.quota.windows.map((window) => (
                  <li key={`${window.limitId}-${window.window}`}>
                    <div className="plan-quota-heading">
                      <strong>{window.name}</strong>
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
                      {window.windowDurationMinutes && (
                        <span>{quotaWindowLabel(window)} window</span>
                      )}
                      <span>
                        {window.resetsAt
                          ? `Resets ${new Date(window.resetsAt * 1000).toLocaleString()}`
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
                    ? "Your reserve blocks new AI work until Codex reports valid usage."
                    : "Your reserve is disabled."}{" "}
                  Refresh the connection to check again.
                </span>
              </p>
            )}
            <p className="ai-help plan-usage-note">
              Updates every 30 seconds while this page is open. ORT’s token
              activity is shown separately in Data.
            </p>
          </div>
          <div className="plan-reserve" aria-labelledby="plan-reserve-title">
            <h3 id="plan-reserve-title">Usage reserve</h3>
            <p className="ai-help">
              Keep some account usage available for work outside ORT.
            </p>
            <label className="plan-reserve-toggle">
              <input
                type="checkbox"
                checked={settings.reserveEnabled}
                disabled={locked}
                onChange={(event) =>
                  onSave({ reserveEnabled: event.target.checked })
                }
              />
              <span>Keep a usage reserve</span>
            </label>
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
                  remaining. At exactly {settings.reservePercent}% remaining,
                  dispatch is allowed.
                </>
              ) : (
                "The reserve is off. ORT can start AI work without a minimum remaining percentage."
              )}
            </p>
            <p className="ai-help plan-reserve-limit">
              Checked before every pass. Ongoing requests and usage elsewhere
              can cross the reserve.
            </p>
          </div>
        </div>
      </section>
    </>
  );
}
