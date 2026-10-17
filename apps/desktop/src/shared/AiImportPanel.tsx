import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  aiImportError,
  aiImportStatus,
  importProviderLabel,
  type AiImportStatus,
} from "./ai-import";

export function AiImportPanel({
  active: visible,
  disabled,
  pending,
  onImport,
  onReady,
}: {
  active: boolean;
  disabled: boolean;
  pending: boolean;
  onImport: (button: HTMLButtonElement) => void;
  onReady?: (button: HTMLButtonElement) => void;
}) {
  const [status, setStatus] = useState<AiImportStatus | null>(null);
  const [checking, setChecking] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const importButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!visible) return;
    setChecking(!pending);
    let active = true;
    let epoch = 0;
    const update = () => {
      if (pending) return;
      const current = ++epoch;
      void aiImportStatus().then((result) => {
        if (!active || current !== epoch) return;
        setChecking(false);
        if (result.ok) {
          setStatus(result.value);
          setError(null);
        } else {
          setStatus(null);
          setError(result.error.code);
        }
      });
    };
    update();
    const subscriptions = [
      "ort:ai-model-changed",
      "ort:ai-operation-state",
    ].map((event) => listen(event, update).catch(() => () => {}));
    window.addEventListener("focus", update);
    const timer = window.setInterval(update, 15000);
    return () => {
      active = false;
      window.clearInterval(timer);
      window.removeEventListener("focus", update);
      subscriptions.forEach(
        (subscription) => void subscription.then((stop) => stop()),
      );
    };
  }, [pending, visible]);
  const ready = !checking && !error && status?.ready && !status.busy;
  useEffect(() => {
    if (ready && !disabled && importButton.current)
      onReady?.(importButton.current);
  }, [ready, disabled, onReady]);
  return (
    <section
      className="resume-import-method"
      aria-labelledby="ai-import-heading"
    >
      <h3 id="ai-import-heading">Import with AI</h3>
      <div className="resume-import-provider" aria-label="AI import provider">
        <strong
          className={`resume-import-provider__status${ready && !pending ? " is-ready" : ""}`}
          role="status"
        >
          {pending
            ? "Importing with AI…"
            : checking
              ? "Checking AI…"
              : ready
                ? "Ready to import"
                : status?.busy
                  ? "AI is busy"
                  : status?.issue === "AI_MODEL_IMAGE_UNSUPPORTED"
                    ? "PDF import unavailable"
                    : "AI is not ready"}
        </strong>
        <dl>
          <div>
            <dt>Provider</dt>
            <dd>{importProviderLabel(status?.provider ?? null)}</dd>
          </div>
          <div>
            <dt>Model</dt>
            <dd>{status?.model || "No model selected"}</dd>
          </div>
          {status?.reasoning && (
            <div>
              <dt>Reasoning</dt>
              <dd className="resume-import-provider__reasoning">
                {status.reasoning}
              </dd>
            </div>
          )}
        </dl>
        {status?.issue && (
          <p className="resume-import-provider__issue">
            {aiImportError(status.issue)}
          </p>
        )}
        {error && (
          <p className="resume-import-provider__issue" role="alert">
            {aiImportError(error)}
          </p>
        )}
        <p
          className="resume-import-method__transmission"
          id="ai-import-transmission"
        >
          {status?.provider === "codex"
            ? "Pages are sent to Codex using your ChatGPT plan."
            : "Sent to your AI provider. Charges may apply."}
        </p>
      </div>
      <div className="resume-import-method__action">
        <p>
          {status?.provider === "codex"
            ? "PDF up to 10 MB / 10 pages"
            : "PDF up to 10 MB"}
        </p>
        <button
          type="button"
          className="resume-import-file-button"
          aria-label="Choose PDF to import with AI"
          aria-describedby="ai-import-transmission"
          ref={importButton}
          disabled={disabled || pending || !ready}
          onClick={(event) => onImport(event.currentTarget)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v1M3 7h16a2 2 0 0 1 2 2l-2 10H5L3 7Z" />
          </svg>
          Choose PDF
        </button>
      </div>
    </section>
  );
}
