import { useEffect, useRef } from "react";
import type { AiAttemptFailure } from "@ort/contracts/wire";
import {
  AiFailureDetailsContent,
  aiFailureName,
  aiFailureReason,
} from "./AiFailureDetailsView";

export function AiRecentFailuresDialog({
  open,
  failures,
  period,
  selection,
  onClose,
}: {
  open: boolean;
  failures: AiAttemptFailure[];
  period: string;
  selection: string;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const element = dialog.current;
    if (!element || !open) return;
    const previousFocus = document.activeElement;
    element.showModal();
    close.current?.focus();
    return () => {
      element.close();
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected)
        previousFocus.focus();
    };
  }, [open]);

  const sorted = [...failures].sort(
    (left, right) =>
      right.startedAtUnixMs - left.startedAtUnixMs ||
      right.attemptId.localeCompare(left.attemptId),
  );

  return (
    <dialog
      ref={dialog}
      className="ai-recent-failures-dialog"
      aria-labelledby="ai-recent-failures-title"
      aria-describedby="ai-recent-failures-description"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="ai-recent-failures-heading">
        <div>
          <h3 id="ai-recent-failures-title">Recent failures</h3>
          <p>
            {selection} · {period}
          </p>
        </div>
        <button
          ref={close}
          type="button"
          className="button--quiet button--compact"
          onClick={onClose}
        >
          Close
        </button>
      </div>
      <div className="ai-recent-failures-body">
        <p id="ai-recent-failures-description" className="ai-help">
          Latest 10 failures, newest first. Select a row to view its details.
        </p>
        {sorted.length === 0 ? (
          <p className="ai-failure-empty">
            No recorded failures for this period and key selection.
          </p>
        ) : (
          <ol className="ai-failure-history">
            {sorted.map((failure) => (
              <li key={failure.attemptId}>
                <details className="ai-failure-row" key={`${open}`}>
                  <summary>
                    <span className="ai-failure-row-copy">
                      <strong>
                        {aiFailureName(failure.details?.code, failure.category)}
                      </strong>
                      <span>
                        {failure.requestedModel} · call {failure.callNumber}
                      </span>
                      <time
                        dateTime={new Date(
                          failure.startedAtUnixMs,
                        ).toISOString()}
                      >
                        {new Date(failure.startedAtUnixMs).toLocaleString()}
                      </time>
                    </span>
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                      <path d="m6 8 4 4 4-4" />
                    </svg>
                  </summary>
                  <div className="ai-failure-row-body">
                    <p>
                      {aiFailureReason(failure.details?.code, failure.category)}
                    </p>
                    <AiFailureDetailsContent
                      details={{
                        diagnostic: failure.details,
                        category: failure.category,
                        provider: failure.provider,
                        model: failure.requestedModel,
                        effectiveModel: failure.effectiveModel,
                        operationType: failure.operationType.replaceAll(
                          "_",
                          " ",
                        ),
                        callNumber: failure.callNumber,
                        maximumCalls: [
                          "tailor_resume",
                          "refine_resume",
                        ].includes(failure.operationType)
                          ? 4
                          : undefined,
                        durationMs: failure.durationMs,
                        usageComplete: failure.usageComplete,
                        usage: failure.usage,
                        operationId: failure.operationId,
                        attemptId: failure.attemptId,
                      }}
                    />
                  </div>
                </details>
              </li>
            ))}
          </ol>
        )}
        <p className="ai-help">
          Keys, prompts, and response text are excluded.
        </p>
      </div>
    </dialog>
  );
}
