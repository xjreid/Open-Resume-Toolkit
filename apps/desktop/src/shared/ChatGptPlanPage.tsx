import { useEffect, useState } from "react";
import { useChatGptPlan } from "./use-chatgpt-plan";
import type * as Wire from "@ort/contracts/wire";
import { invokeDesktop as invoke } from "./desktop-client";
import "./styles/chatgpt-plan.css";

import {
  planErrorMessage,
  REASONING_EFFORT_LABELS,
} from "./chatgpt-plan-presentation";
import { CodexRuntimeSetup } from "./CodexRuntimeSetup";
import { useCodexRuntimeReadiness } from "./use-codex-runtime-readiness";

const efforts = Object.entries(REASONING_EFFORT_LABELS) as [
  Wire.ReasoningEffort,
  string,
][];

export function ChatGptPlanPage({
  visible,
  blocked,
  status,
  onStatus,
}: {
  visible: boolean;
  blocked: boolean;
  status: Wire.PlanStatus | null;
  onStatus: (status: Wire.PlanStatus) => void;
}) {
  const { working, refreshing, notice, refresh, act, save, setNotice } =
    useChatGptPlan(visible, status, onStatus);
  const [installing, setInstalling] = useState(false);
  const runtime = useCodexRuntimeReadiness(visible);
  const [reserve, setReserve] = useState("20");
  useEffect(
    () => setReserve(String(status?.settings.reservePercent ?? 20)),
    [status?.settings.reservePercent],
  );
  const locked = blocked || working || installing || !!status?.operationActive;
  const settings = status?.settings;
  const model = status?.models.find((model) => model.id === settings?.model);
  const error =
    notice ||
    (status?.errorCode &&
    ![
      "PLAN_RUNTIME_MISSING",
      "PLAN_RUNTIME_UNTRUSTED",
      "PLAN_RUNTIME_INCOMPATIBLE",
      "PLAN_PLATFORM_UNSUPPORTED",
    ].includes(status.errorCode)
      ? planErrorMessage(status.errorCode)
      : "");
  return (
    <div
      className="ai-page-panels chatgpt-plan-page"
      hidden={!visible}
      aria-label="Codex settings"
    >
      <section className="ai-panel" aria-labelledby="chatgpt-plan-title">
        <div className="ai-panel-heading">
          <div>
            <h3 id="chatgpt-plan-title">Codex</h3>
            <p className="ai-help">
              Connect your ChatGPT account through Codex for resume tailoring
              and other AI work.
            </p>
          </div>
        </div>
        <label className="plan-enable">
          <input
            type="checkbox"
            checked={settings?.enabled ?? false}
            disabled={
              blocked ||
              working ||
              installing ||
              !settings ||
              (!settings.enabled &&
                (!!status?.operationActive ||
                  runtime.checking ||
                  !runtime.readiness?.ready))
            }
            onChange={(event) => void save({ enabled: event.target.checked })}
          />
          <span>
            <strong>Enable Codex</strong>
            <small>
              Allow the Codex runtime to run and use Codex for all AI work.
              Disable it to sign out, stop the runtime, and use API keys.
            </small>
          </span>
        </label>
        {settings?.enabled && (
          <p className="ai-help">
            Your Enable Codex preference stays saved. Sign-in stays in memory
            only, so sign in again after quitting ORT, switching profiles, or a
            Codex session restart. Your model, reserve settings, and activity
            history stay saved.
          </p>
        )}
        {error && (
          <p className="notice" role="alert">
            {error}
          </p>
        )}
        <CodexRuntimeSetup
          visible={visible}
          blocked={
            blocked ||
            working ||
            installing ||
            !!status?.operationActive ||
            !!status?.loginPending
          }
          readiness={runtime.readiness}
          checking={runtime.checking}
          error={runtime.error}
          onCheck={() => void runtime.refresh()}
          onInstalled={() => {
            void runtime.refresh();
            void refresh(true);
          }}
          onBusy={setInstalling}
        />
        {!status ? (
          <p role="status">Loading Codex settings…</p>
        ) : (
          <>
            {status.connected ? (
              <>
                <div className="plan-account">
                  <span
                    className={`application-dot${settings!.enabled ? " application-dot--ready" : ""}`}
                  />
                  <strong>ChatGPT account connected</strong>
                  <span>
                    {status.accountPlan
                      ? `${status.accountPlan} plan`
                      : "Authorization checked before each pass"}
                  </span>
                </div>
                <div className="plan-model-controls">
                  <label>
                    Model
                    <select
                      aria-label="Codex model"
                      value={settings!.model ?? ""}
                      disabled={locked}
                      onChange={(event) => {
                        const next = status.models.find(
                          (model) => model.id === event.target.value,
                        )!;
                        void save({
                          model: next.id,
                          reasoning: next.reasoningEfforts.includes(
                            settings!.reasoning,
                          )
                            ? settings!.reasoning
                            : next.reasoningEfforts[0],
                        });
                      }}
                    >
                      <option value="" disabled>
                        Choose a supported model
                      </option>
                      {status.models.map((model) => (
                        <option
                          key={model.id}
                          value={model.id}
                          disabled={!model.supported}
                        >
                          {model.name}
                          {model.supported ? "" : " — Unavailable"}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    Reasoning
                    <select
                      aria-label="Codex reasoning"
                      value={settings!.reasoning}
                      disabled={locked || !model?.supported}
                      onChange={(event) =>
                        void save({
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
                <p className="ai-help">
                  {model?.explanation ??
                    "Unavailable choices are not offered by the installed runtime. Account access is checked when a request runs."}{" "}
                  Your resume, job description, and instructions are sent to
                  OpenAI under your account’s plan and privacy terms.
                </p>
                {status.operationActive && (
                  <div className="plan-operation" role="status">
                    <span>
                      AI work is in progress. Model and reserve controls are
                      locked.
                    </span>
                    <button
                      type="button"
                      className="button--secondary"
                      onClick={() => void invoke("stop_chatgpt_plan")}
                    >
                      Stop
                    </button>
                  </div>
                )}
              </>
            ) : settings!.enabled || settings!.cleanupRequired ? (
              <div className="plan-connect">
                <p className="ai-help">
                  {settings!.enabled
                    ? "Sign in securely in your system browser."
                    : "Enable Codex above to connect your ChatGPT account."}{" "}
                  Codex keeps this ORT session’s authorization in memory without
                  saving it to Keychain or a credential file.
                </p>
                {status.loginPending ? (
                  <div className="plan-operation" role="status">
                    <span>Waiting for browser sign-in…</span>
                    <button
                      type="button"
                      className="button--secondary"
                      disabled={working}
                      onClick={() => void act("cancel")}
                    >
                      Cancel sign-in
                    </button>
                  </div>
                ) : (
                  <button
                    type="button"
                    disabled={
                      locked ||
                      runtime.checking ||
                      !runtime.readiness?.ready ||
                      !settings!.enabled ||
                      status.errorCode === "PLAN_RUNTIME_UNAVAILABLE" ||
                      status.errorCode === "PLAN_PLATFORM_UNSUPPORTED" ||
                      settings!.cleanupRequired
                    }
                    onClick={() => void act("connect")}
                  >
                    Connect ChatGPT account
                  </button>
                )}
              </div>
            ) : null}
            {settings!.enabled && (
              <div className="plan-runtime">
                <button
                  type="button"
                  className="button--quiet"
                  disabled={
                    working ||
                    installing ||
                    refreshing ||
                    runtime.checking ||
                    status.operationActive
                  }
                  onClick={() => {
                    void runtime.refresh();
                    void refresh(true);
                  }}
                >
                  Refresh connection
                </button>
              </div>
            )}
            {status.connected && (
              <>
                <div className="plan-section-heading">
                  <h4>Account-wide remaining usage</h4>
                  <button
                    type="button"
                    className="button--secondary"
                    disabled={locked || refreshing}
                    onClick={() => void refresh(true)}
                  >
                    Refresh usage
                  </button>
                </div>
                {status.quota ? (
                  <ul className="plan-quota-list">
                    {status.quota.windows.map((window) => (
                      <li key={`${window.limitId}-${window.window}`}>
                        <div>
                          <strong>
                            {window.name}
                            {window.windowDurationMinutes
                              ? ` · ${window.windowDurationMinutes >= 60 ? `${window.windowDurationMinutes / 60} hours` : `${window.windowDurationMinutes} minutes`}`
                              : ""}
                          </strong>
                          <small>
                            {window.resetsAt
                              ? `Resets ${new Date(window.resetsAt * 1000).toLocaleString()}`
                              : "Reset time not supplied"}
                          </small>
                        </div>
                        <span>
                          {window.remainingPercent.toLocaleString(undefined, {
                            maximumFractionDigits: 1,
                          })}
                          % remaining
                        </span>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="ai-help">
                    Remaining usage is unavailable.{" "}
                    {settings!.reserveEnabled
                      ? "The enabled reserve blocks new passes until Codex reports valid quota."
                      : "Your reserve is disabled."}
                  </p>
                )}
                <p className="ai-help">
                  This includes usage from other apps. ORT’s token activity is
                  shown separately in Data. Usage refreshes every 30 seconds
                  while this page is visible.
                </p>
                <div className="plan-reserve">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings!.reserveEnabled}
                      disabled={locked}
                      onChange={(event) =>
                        void save({ reserveEnabled: event.target.checked })
                      }
                    />
                    <strong>Keep a usage reserve</strong>
                  </label>
                  <form
                    onSubmit={(event) => {
                      event.preventDefault();
                      const value = Number(reserve);
                      if (
                        !Number.isInteger(value) ||
                        value < 1 ||
                        value > 100
                      ) {
                        setNotice(planErrorMessage("PLAN_SETTINGS_INVALID"));
                        return;
                      }
                      void save({ reservePercent: value });
                    }}
                  >
                    <label>
                      Minimum remaining
                      <input
                        type="number"
                        min="1"
                        max="100"
                        step="1"
                        inputMode="numeric"
                        aria-label="Usage reserve percentage"
                        value={reserve}
                        disabled={locked || !settings!.reserveEnabled}
                        onChange={(event) => setReserve(event.target.value)}
                      />
                    </label>
                    <span>%</span>
                    <button
                      type="submit"
                      className="button--secondary"
                      disabled={
                        locked ||
                        !settings!.reserveEnabled ||
                        reserve === String(settings!.reservePercent)
                      }
                    >
                      Save reserve
                    </button>
                  </form>
                </div>
                <p className="ai-help">
                  A fresh quota check runs before every pass. At exactly{" "}
                  {settings!.reservePercent}% remaining, dispatch is allowed.
                  The reserve controls new dispatches; ongoing requests and
                  usage elsewhere can cross it.
                </p>
              </>
            )}
            {(status.connected ||
              settings?.connectionId ||
              status.errorCode === "PLAN_CREDENTIAL_CLEANUP_REQUIRED") && (
              <div className="plan-disconnect">
                <p className="ai-help">
                  Signing out stops current Codex work and ends this account
                  session. Codex stays enabled; sign in again to resume AI work.
                </p>
                <button
                  type="button"
                  className="button--secondary"
                  disabled={blocked || working}
                  onClick={() => void act("disconnect")}
                >
                  {settings!.cleanupRequired ? "Retry sign-out" : "Sign out"}
                </button>
              </div>
            )}
          </>
        )}
      </section>
    </div>
  );
}
