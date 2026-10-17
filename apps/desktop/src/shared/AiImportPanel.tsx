import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  aiImportError,
  aiImportStatus,
  importProviderLabel,
  type AiImportStatus,
} from "./ai-import";
import { AiFailureDetailsView } from "./AiFailureDetailsView";

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
  const [error, setError] = useState<{
    code: string;
    details: Record<string, unknown>;
  } | null>(null);
  const refresh = useRef<() => void>(() => {});
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
          setError(result.error);
        }
      });
    };
    refresh.current = update;
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
      <div>
        <h3 id="ai-import-heading">Import with AI</h3>
        <p>
          Let your selected AI read the PDF and organize its content into resume
          sections.
        </p>
      </div>
      <div className="resume-import-provider" aria-label="AI import provider">
        <strong
          className={`resume-import-provider__status${ready ? " is-ready" : ""}`}
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
            <dd>{status?.model ?? "No model selected"}</dd>
          </div>
          {status?.reasoning && (
            <div>
              <dt>Reasoning</dt>
              <dd>{status.reasoning}</dd>
            </div>
          )}
        </dl>
        {status?.issue && <p>{aiImportError(status.issue)}</p>}
        {error && (
          <div role="alert">
            <p>{aiImportError(error.code)}</p>
            <AiFailureDetailsView
              details={{
                ...error.details,
                diagnostic: {
                  code: error.code,
                  ...((error.details.diagnostic as object) ?? {}),
                },
              }}
            />
          </div>
        )}
        <button
          type="button"
          className="button--secondary"
          disabled={pending || checking}
          onClick={() => {
            setChecking(true);
            refresh.current();
          }}
        >
          Check connection
        </button>
      </div>
      <div className="resume-import-method__action">
        <p>
          {status?.provider === "codex"
            ? "PDF · up to 10 MB and 10 pages. Pages are sent to Codex as images for extraction. Uses your connected ChatGPT plan."
            : "PDF · up to 10 MB. The selected file is sent to your AI provider. Provider charges may apply."}
        </p>
        <button
          type="button"
          ref={importButton}
          disabled={disabled || pending || !ready}
          onClick={(event) => onImport(event.currentTarget)}
        >
          Import with AI
        </button>
      </div>
    </section>
  );
}
