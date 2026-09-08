import { useEffect, useRef, useState } from "react";
import type { VersionedResume } from "@ort/contracts/resume";
import {
  beginDocumentImport,
  cancelDocumentImport,
  documentImportAvailable,
} from "./import-client";
import { ImportReviewFlow } from "./ImportReviewFlow";

export function DocumentImport({
  disabled,
  revision,
  onBusyChange,
  onSaved,
}: {
  disabled: boolean;
  revision: number | null;
  onBusyChange: (busy: boolean) => void;
  onSaved: (saved: VersionedResume) => void;
}) {
  const [available, setAvailable] = useState(false);
  const [pending, setPending] = useState(false);
  const [reviewId, setReviewId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const inFlight = useRef(false);
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
    setPending(true);
    setError(null);
    onBusyChange(true);
    const result = await beginDocumentImport(revision);
    inFlight.current = false;
    if (!mounted.current) return;
    setPending(false);
    if (result.ok && result.value) {
      // A cancellation may race successful native completion. Retain the
      // native-created session so its explicit Cancel action can retire it.
      setReviewId(result.value.id);
    } else {
      onBusyChange(false);
      if (!result.ok && !cancelled.current)
        setError(
          "Import could not finish. Your resume has not been changed. Choose a supported text-based PDF or DOCX and try again.",
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
    setReviewId(null);
    onBusyChange(false);
  }
  return (
    <section aria-label="Import a resume">
      <h2>Import an existing resume</h2>
      <p>
        Choose a PDF or DOCX, compare the extracted text, and accept or reject
        every proposed change. Scans require OCR and are not supported.
      </p>
      {reviewId ? (
        <ImportReviewFlow
          reviewId={reviewId}
          currentRevision={revision ?? 0}
          onCancelled={finished}
          onSaved={(saved) => {
            finished();
            onSaved(saved);
          }}
        />
      ) : (
        <>
          <button
            type="button"
            className="button--secondary"
            disabled={disabled || !available || pending}
            onClick={() => void begin()}
          >
            Import an existing resume
          </button>
          {!available ? (
            <p>Import is unavailable in this development build.</p>
          ) : null}
          {pending ? (
            <div role="status">
              <p>
                Preparing your import. If the file picker is open, use its
                Cancel button to close it.
              </p>
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
