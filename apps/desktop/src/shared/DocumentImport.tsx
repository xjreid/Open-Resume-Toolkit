import { useEffect, useRef, useState } from "react";
import type { DocumentStyle } from "@ort/contracts/export";
import type { ImportReviewSnapshot } from "@ort/contracts/import";
import type { VersionedResume } from "@ort/contracts/resume";
import {
  beginDocumentImport,
  cancelDocumentImport,
  documentImportAvailable,
} from "./import-client";
import { ImportReviewFlow } from "./ImportReviewFlow";

export function DocumentImport({
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
  const [review, setReview] = useState<ImportReviewSnapshot | null>(null);
  const reviewId = review?.id ?? null;
  const [error, setError] = useState<string | null>(null);
  const launch = useRef<HTMLButtonElement>(null);
  const restoreFocus = useRef(false);
  useEffect(() => {
    if (reviewId === null && restoreFocus.current) {
      restoreFocus.current = false;
      launch.current?.focus();
    }
  }, [reviewId]);
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
  async function begin() {
    if (disabled || !available || inFlight.current || reviewId) return;
    inFlight.current = true;
    cancelled.current = false;
    reloadNeeded.current = false;
    setPending(true);
    setError(null);
    onBusyChange(true);
    onOperationChange?.(true);
    const result = await beginDocumentImport(revision);
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
      if (!result.ok && !cancelled.current)
        setError(
          "Could not read this resume. Choose a text-based PDF or DOCX and try again.",
        );
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
    <section aria-label="Resume file importer">
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
        <>
          <h2>Import a resume</h2>
          <p>Text-based PDF or DOCX</p>
          <button
            type="button"
            ref={launch}
            className="button--secondary"
            disabled={disabled || !available || pending}
            onClick={() => void begin()}
          >
            Import resume
          </button>
          {available === false ? (
            <p role="status">Import is unavailable in this build.</p>
          ) : null}
          {disabledReason && available && !pending ? (
            <p role="status">{disabledReason}</p>
          ) : null}
          {pending ? (
            <div role="status">
              <p>Reading resume…</p>
              <button type="button" onClick={() => void cancel()}>
                Cancel import
              </button>
            </div>
          ) : null}
        </>
      )}
      {error ? <p role="alert">{error}</p> : null}
    </section>
  );
}
