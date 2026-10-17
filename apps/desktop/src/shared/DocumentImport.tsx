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
import { AiFailureDetailsView } from "./AiFailureDetailsView";
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
  const [errorDetails, setErrorDetails] = useState<Record<
    string,
    unknown
  > | null>(null);
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
    setErrorDetails(null);
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
        setErrorDetails({
          ...result.error.details,
          diagnostic: {
            code: result.error.code,
            ...((result.error.details?.diagnostic as object) ?? {}),
          },
        });
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
              <path d="M9 3h10l6 6v20H9V3Z" />
              <path d="M19 3v7h6M13 15h8M13 19h8M13 23h5" />
            </svg>
            <div>
              <h2>Import a resume</h2>
              <p className="resume-import-start__description">
                Bring your existing resume into the editor, then review and
                refine it.
              </p>
            </div>
          </div>
          {error ? (
            <div className="resume-import-error" role="alert" ref={errorPanel}>
              <p>{error}</p>
              {errorDetails && (
                <AiFailureDetailsView details={errorDetails} open />
              )}
            </div>
          ) : null}
          {disabledReason && !pending ? (
            <p role="status">{disabledReason}</p>
          ) : null}
          {pending ? (
            <div className="resume-import-progress" role="status">
              <p>
                {method === "ai"
                  ? "Reading your PDF with AI… Page preparation may take 30 seconds, followed by up to 2 minutes for AI."
                  : "Reading resume…"}
              </p>
              <button type="button" onClick={() => void cancel()}>
                Cancel import
              </button>
            </div>
          ) : null}
          <div className="resume-import-methods">
            <section
              className="resume-import-method"
              aria-labelledby="local-import-heading"
            >
              <div>
                <h3 id="local-import-heading">Import locally</h3>
                <p>
                  Extract text from your resume on this device, then organize
                  and refine it.
                </p>
              </div>
              <div className="resume-import-method__local-note">
                <strong>No AI connection needed</strong>
                <p>
                  Your file stays on this device. Works with text-based PDFs and
                  DOCX files.
                </p>
              </div>
              <div className="resume-import-method__action">
                <p>Text-based PDF or DOCX</p>
                <button
                  type="button"
                  ref={launch}
                  disabled={disabled || !available || pending}
                  onClick={(event) => void begin("local", event.currentTarget)}
                >
                  Import resume
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
          <ol className="resume-import-start__steps" aria-label="Import steps">
            <li>
              <strong>Choose a file</strong>
              <span>
                Choose local or AI import and select your resume file.
              </span>
            </li>
            <li>
              <strong>Review your resume</strong>
              <span>
                Edit the content and organize its sections in the resume editor.
              </span>
            </li>
            <li>
              <strong>Save when ready</strong>
              <span>Map the reviewed content to your saved master resume.</span>
            </li>
          </ol>
          <p className="resume-import-start__note">
            Your saved resume stays unchanged until you confirm the import.
          </p>
        </div>
      )}
    </section>
  );
}

function importFailureMessage(code: string): string {
  switch (code) {
    case "LOCAL_DATA_OPERATION_BUSY":
      return "Another file operation is in progress. Try again when it finishes.";
    case "REVISION_CONFLICT":
      return "Your saved resume changed. Reload it before importing.";
    case "IMPORT_INVALID_SOURCE":
      return "This file is unsupported, encrypted, or too large to import.";
    case "IMPORT_DISABLED":
      return "Import is unavailable in this build.";
    default:
      return "This resume could not be imported. Try exporting a new PDF or DOCX copy.";
  }
}
