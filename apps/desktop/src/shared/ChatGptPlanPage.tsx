import { useState } from "react";
import type * as Wire from "@ort/contracts/wire";
import { useChatGptPlan } from "./use-chatgpt-plan";
import { planErrorMessage } from "./chatgpt-plan-presentation";
import { CodexAccountSettings } from "./CodexAccountSettings";
import { CodexRuntimeSetup } from "./CodexRuntimeSetup";
import { useCodexRuntimeReadiness } from "./use-codex-runtime-readiness";
import { CodexPageIcon } from "./CodexPageIcon";
import openaiLogo from "../assets/providers/openai.svg";
import "./styles/chatgpt-plan.css";

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
  const [serverAction, setServerAction] = useState<"start" | "stop" | null>(
    null,
  );
  const runtime = useCodexRuntimeReadiness(visible);
  const settings = status?.settings;
  const enabled = settings?.enabled ?? false;
  const running =
    enabled && !!status?.runtimeVersion && !settings?.cleanupRequired;
  const connected = running && !!status?.connected;
  const locked = blocked || working || installing || !!status?.operationActive;
  const installationNeeded = runtime.readiness?.ready === false;
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

  async function changeServer() {
    const starting = !enabled;
    setServerAction(starting ? "start" : "stop");
    try {
      await save({ enabled: starting });
      // Saving permission does not launch the native runtime. Loading status
      // starts it and reports the session before offering account sign-in.
      if (starting) await refresh(true);
    } finally {
      setServerAction(null);
    }
  }
  function refreshConnection() {
    setNotice("");
    void runtime.refresh();
    void refresh(true);
  }
  async function signOut() {
    await act("disconnect");
    // Sign-out retires the account's runtime session. Reopen the enabled
    // server with no authorization so another browser sign-in is available.
    await refresh(true);
  }
  const installed = runtime.readiness?.ready === true;
  const installationLabel =
    runtime.checking && !runtime.readiness
      ? "Checking Codex Runtime…"
      : installed
        ? "Codex Runtime is Installed"
        : runtime.error
          ? "Codex Runtime could not be checked"
          : runtime.readiness?.errorCode === "PLAN_RUNTIME_MISSING"
            ? "Codex Runtime isn’t Detected"
            : installationNeeded
              ? "Codex Runtime needs attention"
              : "Checking Codex Runtime…";
  const serverLabel =
    serverAction === "stop"
      ? "Quitting…"
      : serverAction === "start" || (!running && refreshing)
        ? "Launching…"
        : running
          ? "Running"
          : "Unavailable";

  return (
    <div
      className="ai-page-panels chatgpt-plan-page plan-workspace"
      hidden={!visible}
      aria-label="ChatGPT settings"
    >
      <div className="plan-server-frame">
        <section className="plan-server" aria-labelledby="chatgpt-plan-title">
          <div className="plan-server-identity">
            <h3 id="chatgpt-plan-title" className="plan-brand">
              <img src={openaiLogo} alt="" />
              <span>ChatGPT</span>
            </h3>
            <p
              className={`plan-installation-status${installed ? " plan-installation-status--ready" : ""}`}
              role="status"
            >
              {installed && <CodexPageIcon name="check" />}
              {installationLabel}
            </p>
          </div>
          <div className="plan-server-actions">
            {enabled && (
              <>
                <span
                  className={`plan-runtime-status${running ? " plan-runtime-status--running" : ""}`}
                  role="status"
                >
                  <span
                    className={`plan-status-dot${running ? " plan-status-dot--ready" : ""}`}
                    aria-hidden="true"
                  />
                  {serverLabel}
                </span>
                <button
                  type="button"
                  className="button--quiet plan-refresh"
                  aria-label={
                    refreshing || runtime.checking
                      ? "Refreshing connection"
                      : "Refresh connection"
                  }
                  title="Refresh runtime status and account usage"
                  disabled={locked || refreshing || runtime.checking}
                  onClick={refreshConnection}
                >
                  <CodexPageIcon name="refresh" />
                </button>
              </>
            )}
            <button
              type="button"
              className={enabled ? "button--danger plan-power" : "plan-power"}
              title={enabled ? "Stop AI work and sign out" : undefined}
              disabled={
                blocked ||
                working ||
                installing ||
                !!serverAction ||
                !settings ||
                (!enabled &&
                  (!!status?.operationActive ||
                    runtime.checking ||
                    !runtime.readiness?.ready ||
                    settings.cleanupRequired))
              }
              onClick={() => void changeServer()}
            >
              {serverAction === "stop"
                ? "Quitting…"
                : serverAction === "start"
                  ? "Launching…"
                  : enabled
                    ? "Quit"
                    : "Launch"}
            </button>
          </div>
        </section>
      </div>
      {error && (
        <p className="notice plan-notice" role="alert">
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
        showCheck={!enabled}
        onCheck={refreshConnection}
        onInstalled={refreshConnection}
        onBusy={setInstalling}
      />
      {connected && status ? (
        <CodexAccountSettings
          status={status}
          locked={locked}
          onSave={save}
          onNotice={setNotice}
          onSignOut={() => void signOut()}
          signOutDisabled={blocked || working || installing}
        />
      ) : running ? (
        <section className="plan-connect" aria-labelledby="plan-sign-in-title">
          <h3 id="plan-sign-in-title" className="visually-hidden">
            Connect your account
          </h3>
          {status?.loginPending ? (
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
              className="plan-login-button"
              disabled={locked || runtime.checking || !runtime.readiness?.ready}
              onClick={() => void act("connect")}
            >
              <span>Log in with</span>{" "}
              <span className="plan-login-brand">
                <img src={openaiLogo} alt="" />
                OpenAI
              </span>
            </button>
          )}
          <p className="ai-help plan-session-note">
            Opens in your browser. Sign in again after restarting ORT.
          </p>
        </section>
      ) : null}
      {settings?.cleanupRequired && (
        <div className="plan-operation plan-cleanup">
          <p className="ai-help">
            Clear the previous session before starting the server or signing in
            again.
          </p>
          <button
            type="button"
            className="button--secondary"
            disabled={blocked || working}
            onClick={() => void signOut()}
          >
            Retry sign-out
          </button>
        </div>
      )}
      <details className="plan-details plan-connection-details">
        <summary>Connection &amp; privacy</summary>
        <div className="plan-details-content">
          <p>
            Stopping the server ends Codex work and signs you out. Your model,
            reserve settings, and activity history stay saved. Stop the server
            to use API keys.
          </p>
          <p>
            Sign-in stays in memory only, without saving it to Keychain or a
            credential file. Sign in again after quitting ORT, switching
            profiles, or restarting the server. Signing out ends this account
            session; the server stays enabled.
          </p>
          <p>
            Your resume, job description, and instructions are sent to OpenAI
            under your account’s plan and privacy terms. Account access is
            checked when a request runs.
          </p>
          {running && <p>Codex runtime v{status!.runtimeVersion}</p>}
        </div>
      </details>
    </div>
  );
}
