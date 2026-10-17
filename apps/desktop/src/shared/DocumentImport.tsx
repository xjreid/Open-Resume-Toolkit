import { useEffect, useRef, useState } from "react";
import type { DocumentStyle } from "@ort/contracts/export";
import type { ImportReviewSnapshot } from "@ort/contracts/import";
import type { VersionedResume } from "@ort/contracts/resume";
import {
  beginDocumentImport,
  beginAiDocumentImport,
  cancelDocumentImport,
  documentImportAvailable,
} from "./import-client";
import { AiImportPanel } from "./AiImportPanel";
import { aiImportError } from "./ai-import";
import { ImportReviewFlow } from "./ImportReviewFlow";

export function DocumentImport({
  active = true,
  disabled,
  reviewDisabled = false,
  disabledReason,
  style,
  revision,
  onBusyChange,
  onOperationChange,
  onSaved,
  onReloadRequired,
  onReload,
}: {
  active?: boolean;
  disabled: boolean;
  reviewDisabled?: boolean;
  disabledReason?: string;
  style?: DocumentStyle;
  revision: number | null;
  onBusyChange: (busy: boolean) => void;
  onOperationChange?: (busy: boolean) => void;
  onSaved: (saved: VersionedResume) => void;
  onReloadRequired?: () => void;
  onReload?: () => void;
}) {
  const [available, setAvailable] = useState<boolean | null>(null);
  const [pending, setPending] = useState(false);
  const [method, setMethod] = useState<"local" | "ai">("local");
  const [review, setReview] = useState<ImportReviewSnapshot | null>(null);
  const reviewId = review?.id ?? null;
  const [error, setError] = useState<string | null>(null);
  const launch = useRef<HTMLButtonElement>(null);
  const errorPanel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (error) errorPanel.current?.scrollIntoView?.({ block: "nearest" });
  }, [error]);
  const restoreFocus = useRef(false);
  useEffect(() => {
    if (reviewId === null && restoreFocus.current && method === "local") {
      restoreFocus.current = false;
      launch.current?.focus();
    }
  }, [reviewId, method]);
  const inFlight = useRef(false);
  const reloadNeeded = useRef(false);
  const mounted = useRef(true);
  const cancelled = useRef(false);
  useEffect(() => {
    mounted.current = true;
    void documentImportAvailable().then((value) => {
      if (mounted.current) setAvailable(value);
    });
    return () => {
      mounted.current = false;
    };
  }, []);
  async function begin(method: "local" | "ai", button: HTMLButtonElement) {
    if (
      disabled ||
      (method === "local" && !available) ||
      inFlight.current ||
      reviewId
    )
      return;
    launch.current = button;
    setMethod(method);
    inFlight.current = true;
    cancelled.current = false;
    reloadNeeded.current = false;
    setPending(true);
    setError(null);
    onBusyChange(true);
    onOperationChange?.(true);
    const result = await (method === "ai"
      ? beginAiDocumentImport(revision)
      : beginDocumentImport(revision));
    inFlight.current = false;
    if (!mounted.current) return;
    setPending(false);
    onOperationChange?.(false);
    if (result.ok && result.value) {
      // A cancellation may race successful native completion. Retain the
      // native-created session so its explicit Cancel action can retire it.
      setReview(result.value);
    } else {
      onBusyChange(false);
      if (!result.ok && !cancelled.current) {
        setError(
          method === "ai"
            ? aiImportError(result.error.code)
            : importFailureMessage(result.error.code),
        );
      }
    }
  }
  async function cancel() {
    cancelled.current = true;
    const result = await cancelDocumentImport();
    if (mounted.current && !result.ok)
      setError(
        "Cancellation could not be confirmed. Wait for import to finish before continuing.",
      );
  }
  function finished() {
    restoreFocus.current = true;
    setReview(null);
    onBusyChange(false);
  }
  return (
    <section className="resume-import" aria-label="Resume file importer">
      {review ? (
        <ImportReviewFlow
          reviewId={review.id}
          initialSnapshot={review}
          style={style}
          disabled={reviewDisabled}
          currentRevision={revision ?? 0}
          onOperationChange={onOperationChange}
          onReloadRequired={() => {
            reloadNeeded.current = true;
            onReloadRequired?.();
          }}
          onCancelled={() => {
            finished();
            if (reloadNeeded.current) onReload?.();
          }}
          onSaved={(saved) => {
            finished();
            onSaved(saved);
          }}
        />
      ) : (
        <div className="resume-import-start">
          <div className="resume-import-start__intro">
            <svg
              className="resume-import-start__icon"
              viewBox="0 0 32 32"
              aria-hidden="true"
            >
              <path d="M17 4H8a2 2 0 0 0-2 2v20a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-7M17 4v7h7M17 4l7 7" />
              <path d="M11 17h5M11 22h9M26 16V5m-4 4 4-4 4 4" />
            </svg>
            <div>
              <h2>Import your resume</h2>
            </div>
          </div>
          {error ? (
            <div className="resume-import-error" role="alert" ref={errorPanel}>
              <p>{error}</p>
            </div>
          ) : null}
          {disabledReason && !pending ? (
            <p className="resume-import-notice" role="status">
              {disabledReason}
            </p>
          ) : null}
          {pending ? (
            <div className="resume-import-progress" role="status">
              <p>
                {method === "ai"
                  ? "Reading your PDF with AI… This may take a few minutes."
                  : "Reading resume…"}
              </p>
              <button
                type="button"
                className="button--secondary"
                onClick={() => void cancel()}
              >
                Cancel import
              </button>
            </div>
          ) : null}
          <div className="resume-import-methods">
            <section
              className="resume-import-method"
              aria-labelledby="local-import-heading"
            >
              <h3 id="local-import-heading">Parse Locally</h3>
              <div className="resume-import-method__local-note">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <rect x="5" y="10" width="14" height="11" rx="2" />
                  <path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3" />
                </svg>
                <div>
                  <strong>Stays on this device</strong>
                  <p>No AI connection needed.</p>
                </div>
              </div>
              <div className="resume-import-method__action">
                <p>Text-based PDF or DOCX</p>
                <button
                  type="button"
                  className="resume-import-file-button"
                  aria-label="Choose file to parse locally"
                  ref={launch}
                  disabled={disabled || !available || pending}
                  onClick={(event) => void begin("local", event.currentTarget)}
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v1M3 7h16a2 2 0 0 1 2 2l-2 10H5L3 7Z" />
                  </svg>
                  Choose file
                </button>
              </div>
              {available === false ? (
                <p role="status">Local import is unavailable in this build.</p>
              ) : null}
            </section>
            <AiImportPanel
              active={active}
              disabled={disabled || pending}
              pending={pending && method === "ai"}
              onImport={(button) => void begin("ai", button)}
              onReady={(button) => {
                if (restoreFocus.current && method === "ai") {
                  restoreFocus.current = false;
                  button.focus();
                }
              }}
            />
          </div>
          <p className="resume-import-start__note">
            Review before saving. Your master resume stays unchanged until you
            confirm.
          </p>
        </div>
      )}
    </section>
  );
}

function importFailureMessage(code: string): string {
  switch (code) {
    case "LOCAL_DATA_OPERATION_BUSY":
      return "Another file operation is in progress; try again when it finishes.";
    case "REVISION_CONFLICT":
      return "Your saved resume changed; reload it before importing.";
    case "IMPORT_INVALID_SOURCE":
      return "This document could not be read; try an unencrypted PDF or DOCX.";
    case "IMPORT_DISABLED":
      return "Import is unavailable in this build.";
    default:
      return "Import failed; try exporting a new PDF or DOCX copy.";
  }
}
