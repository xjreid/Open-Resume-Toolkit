import { useEffect, useRef, useState } from "react";
import type { RuntimeInstallStatus } from "@ort/contracts/wire";
import { invokeDesktop as invoke } from "./desktop-client";
import { planErrorMessage } from "./chatgpt-plan-presentation";

const activePhases = new Set([
  "downloading",
  "verifying",
  "awaiting_approval",
  "checking",
]);

export function CodexRuntimeInstaller({
  visible,
  blocked,
  needed,
  onBusy,
  onComplete,
}: {
  visible: boolean;
  blocked: boolean;
  needed: boolean;
  onBusy: (busy: boolean) => void;
  onComplete: () => void;
}) {
  const [state, setState] = useState<RuntimeInstallStatus | null>(null);
  const [starting, setStarting] = useState(false);
  const [notice, setNotice] = useState("");
  const callbacks = useRef({ onBusy, onComplete });
  callbacks.current = { onBusy, onComplete };
  const previous = useRef<string | null>(null);
  const mounted = useRef(true);
  const busy = starting || activePhases.has(state?.phase ?? "");
  function accept(next: RuntimeInstallStatus, notify = !starting) {
    if (!mounted.current) return;
    setState(next);
    if (
      notify &&
      next.phase === "complete" &&
      previous.current &&
      previous.current !== "complete"
    )
      callbacks.current.onComplete();
    previous.current = next.phase;
  }
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    callbacks.current.onBusy(busy);
  }, [busy]);
  useEffect(() => {
    if (!visible) return;
    let current = true;
    async function refresh() {
      try {
        const response = await invoke("load_codex_runtime_install");
        if (current && response.ok) accept(response.value);
        else if (current && !response.ok)
          setNotice(planErrorMessage(response.error.code));
      } catch {
        if (current)
          setNotice(
            "Installation status is unavailable. Refresh this page to try again.",
          );
      }
    }
    void refresh();
    const timer = busy
      ? window.setInterval(() => void refresh(), 750)
      : undefined;
    return () => {
      current = false;
      clearInterval(timer);
    };
  }, [visible, busy]);
  async function install() {
    setStarting(true);
    setNotice("");
    try {
      const result = await invoke("install_codex_runtime");
      if (result.ok) {
        accept(result.value, false);
        // The command has now released the native operation lock. A poll may
        // observe Complete just before that, so refresh readiness here too.
        if (mounted.current && result.value.phase === "complete")
          callbacks.current.onComplete();
      } else if (mounted.current)
        setNotice(planErrorMessage(result.error.code));
    } catch {
      if (mounted.current)
        setNotice(
          "The installer could not finish. Check installation before retrying.",
        );
    } finally {
      if (mounted.current) setStarting(false);
    }
  }
  async function cancel() {
    const result = await invoke("cancel_codex_runtime_install").catch(
      () => null,
    );
    if (result?.ok) setNotice("Cancelling the download and checks…");
    else
      setNotice(
        result
          ? planErrorMessage(result.error.code)
          : "Cancellation could not be confirmed. Check the installation status.",
      );
  }
  const percent = state?.totalBytes
    ? Math.min(
        100,
        Math.floor((state.downloadedBytes / state.totalBytes) * 100),
      )
    : 0;
  const message = {
    idle: "",
    downloading: `Downloading the official Codex runtime… ${percent}%`,
    verifying: "Verifying the archive and executable checksums…",
    awaiting_approval:
      "Approve installation in the macOS prompt, or cancel there. The protected copy is verified before it replaces the runtime.",
    checking: "Verifying the protected installation…",
    complete:
      "Codex runtime installed and verified. You can start the Codex server.",
    cancelled: "Installation cancelled. You can try again when ready.",
    failed: state?.errorCode
      ? planErrorMessage(state.errorCode)
      : "Installation failed. Try again.",
  }[state?.phase ?? "idle"];
  if (!needed && !busy) return null;
  return (
    <div
      className="plan-runtime-installer"
      aria-label="Codex runtime installation"
    >
      {needed && !busy && (
        <>
          <p className="ai-help">
            Optional download: Codex 0.162.0 for Apple Silicon, about 94 MB.
            macOS will ask for administrator approval to install it. This does
            not start the server or sign you in.
          </p>
          <button
            type="button"
            disabled={blocked || !state}
            onClick={() => void install()}
          >
            {state?.phase === "failed"
              ? "Retry runtime installation"
              : "Install Codex runtime"}
          </button>
        </>
      )}
      {busy && state?.phase === "downloading" && (
        <progress
          aria-label="Codex runtime download"
          value={state.downloadedBytes}
          max={state.totalBytes}
        />
      )}
      {(message || starting) && (
        <p
          className="ai-help"
          role={state?.phase === "failed" ? "alert" : "status"}
        >
          {message || "Preparing the verified runtime download…"}
        </p>
      )}
      {notice && (
        <p className="ai-help" role="status">
          {notice}
        </p>
      )}
      {busy && ["downloading", "verifying"].includes(state?.phase ?? "") && (
        <button
          type="button"
          className="button--quiet"
          onClick={() => void cancel()}
        >
          Cancel installation
        </button>
      )}
    </div>
  );
}
