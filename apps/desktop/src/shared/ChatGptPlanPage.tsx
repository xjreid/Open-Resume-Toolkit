import { useState } from "react";
import type * as Wire from "@ort/contracts/wire";
import { useChatGptPlan } from "./use-chatgpt-plan";
import { planErrorMessage } from "./chatgpt-plan-presentation";
import { CodexAccountSettings } from "./CodexAccountSettings";
import { CodexRuntimeSetup } from "./CodexRuntimeSetup";
import { useCodexRuntimeReadiness } from "./use-codex-runtime-readiness";
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
  const serverLabel = !status
    ? "Checking…"
    : serverAction === "stop"
      ? "Stopping…"
      : serverAction === "start" || (enabled && !running && refreshing)
        ? "Starting…"
        : running
          ? "Running"
          : enabled
            ? "Unavailable"
            : installationNeeded
              ? runtime.readiness?.errorCode === "PLAN_RUNTIME_MISSING"
                ? "Not installed"
                : "Not ready"
              : "Stopped";

  return (
    <div
      className="ai-page-panels chatgpt-plan-page"
      hidden={!visible}
      aria-label="Codex settings"
    >
      <div className="ai-panel plan-workspace">
        <section className="plan-server" aria-labelledby="chatgpt-plan-title">
          <div className="plan-server-heading">
            <div>
              <h3 id="chatgpt-plan-title">Codex</h3>
              <p className="ai-help">
                Use your ChatGPT account for AI work in ORT.
              </p>
            </div>
            <div className="plan-server-actions">
              {enabled && (
                <button
                  type="button"
                  className="button--quiet plan-refresh"
                  aria-label={
                    refreshing || runtime.checking
                      ? "Refreshing connection"
                      : "Refresh connection"
                  }
                  title="Refresh connection"
                  disabled={locked || refreshing || runtime.checking}
                  onClick={refreshConnection}
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                    <path d="M20 7v5h-5M4 17v-5h5M6.1 7a7 7 0 0 1 11.5-1L20 9M4 15l2.4 3A7 7 0 0 0 17.9 17" />
                  </svg>
                  {refreshing || runtime.checking ? "Refreshing…" : "Refresh"}
                </button>
              )}
              {(enabled || !installationNeeded) && (
                <button
                  type="button"
                  className={enabled ? "button--secondary" : undefined}
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
                    ? "Stopping…"
                    : serverAction === "start"
                      ? "Starting…"
                      : enabled
                        ? "Stop server"
                        : "Start server"}
                </button>
              )}
            </div>
          </div>
          <dl className="plan-status-strip" aria-live="polite">
            <div>
              <dt>Codex server</dt>
              <dd>
                <span
                  className={`plan-status-dot${running ? " plan-status-dot--ready" : ""}`}
                />
                {serverLabel}
              </dd>
            </div>
            <div>
              <dt>ChatGPT account</dt>
              <dd>
                <span
                  className={`plan-status-dot${connected ? " plan-status-dot--ready" : ""}`}
                />
                {!status
                  ? "Checking…"
                  : connected
                    ? "Signed in"
                    : status.loginPending && running
                      ? "Signing in…"
                      : "Signed out"}
                {connected && (
                  <small className="plan-account-plan">
                    {status?.accountPlan
                      ? `${status.accountPlan.charAt(0).toUpperCase()}${status.accountPlan.slice(1)} plan`
                      : "Plan unavailable"}
                  </small>
                )}
                {connected && (
                  <button
                    type="button"
                    className="button--quiet plan-sign-out"
                    disabled={blocked || working || installing}
                    onClick={() => void signOut()}
                  >
                    Sign out
                  </button>
                )}
              </dd>
            </div>
          </dl>
          {!running && !installationNeeded && (
            <p className="ai-help plan-server-note">
              {enabled
                ? "Refresh the connection or restart the server to try again."
                : "Start the server, then sign in to ChatGPT."}
            </p>
          )}
        </section>
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
          />
        ) : running ? (
          <section
            className="plan-connect"
            aria-labelledby="plan-sign-in-title"
          >
            <h3 id="plan-sign-in-title">Connect your account</h3>
            <p className="plan-body">
              Sign in with your browser to use your ChatGPT plan.
            </p>
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
                disabled={
                  locked || runtime.checking || !runtime.readiness?.ready
                }
                onClick={() => void act("connect")}
              >
                Sign in to ChatGPT
              </button>
            )}
            <p className="ai-help plan-session-note">
              You’ll need to sign in again after restarting ORT.
            </p>
          </section>
        ) : null}
        {settings?.cleanupRequired && (
          <div className="plan-operation plan-cleanup">
            <p className="ai-help">
              Clear the previous session before starting the server or signing
              in again.
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
    </div>
  );
}
