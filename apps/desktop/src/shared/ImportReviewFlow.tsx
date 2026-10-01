import { useEffect, useRef, useState } from "react";
import type { ImportReviewSnapshot } from "@ort/contracts/import";
import type { DocumentStyle } from "@ort/contracts/export";
import type { ResumeDocument, VersionedResume } from "@ort/contracts/resume";
import {
  readImportReview,
  mapImportReview,
  cancelImportReview,
} from "./import-client";
import { ImportedResumeEditor } from "./ImportedResumeEditor";

// Mount only with a native-created review ID. No file/parser initiation here.
export function ImportReviewFlow(props: {
  reviewId: string;
  currentRevision: number;
  initialSnapshot?: ImportReviewSnapshot;
  style?: DocumentStyle;
  disabled?: boolean;
  onSaved: (saved: VersionedResume) => void;
  onCancelled: () => void;
  onReloadRequired?: () => void;
  onOperationChange?: (busy: boolean) => void;
}) {
  return <ReviewFlow key={props.reviewId} {...props} />;
}
function ReviewFlow({
  reviewId,
  currentRevision,
  initialSnapshot,
  style,
  disabled = false,
  onSaved,
  onCancelled,
  onReloadRequired,
  onOperationChange,
}: Parameters<typeof ImportReviewFlow>[0]) {
  const inFlight = useRef(false);
  const mounted = useRef(true);
  const [snapshot, setSnapshot] = useState<ImportReviewSnapshot | null>(
    initialSnapshot ?? null,
  );
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  useEffect(() => {
    let active = true;
    mounted.current = true;
    if (initialSnapshot?.id !== reviewId)
      void readImportReview(reviewId).then((response) => {
        if (!active) return;
        if (response.ok && response.value.id === reviewId)
          setSnapshot(response.value);
        else
          setError(
            "This import review is unavailable. Cancel and start again.",
          );
      });
    return () => {
      active = false;
      mounted.current = false;
    };
  }, [reviewId, initialSnapshot]);
  async function cancel() {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    onOperationChange?.(true);
    const response = await cancelImportReview(reviewId);
    inFlight.current = false;
    if (!mounted.current) return;
    setBusy(false);
    onOperationChange?.(false);
    if (response.ok) onCancelled();
    else
      setError(
        "Cancellation could not be confirmed. Try Cancel again or close the app to discard this review.",
      );
  }
  async function apply(document: ResumeDocument) {
    if (
      inFlight.current ||
      uncertain ||
      disabled ||
      !snapshot ||
      snapshot.baseRevision !== currentRevision
    )
      return;
    inFlight.current = true;
    setBusy(true);
    onOperationChange?.(true);
    setError(undefined);
    const response = await mapImportReview(reviewId, document);
    inFlight.current = false;
    if (!mounted.current) return;
    setBusy(false);
    onOperationChange?.(false);
    if (response.ok) {
      onSaved(response.value);
    } else if (response.error.code === "IMPORT_REVIEW_INVALID") {
      setError(
        "Some fields could not be saved. Correct the imported resume and try again.",
      );
    } else if (response.error.code === "LOCAL_DATA_OPERATION_BUSY") {
      setError(
        "Another operation is running. Try mapping again when it finishes.",
      );
    } else if (
      response.error.code === "REVISION_CONFLICT" ||
      response.error.code === "IMPORT_REVIEW_UNAVAILABLE" ||
      response.error.code === "IMPORT_DISABLED"
    ) {
      setUncertain(true);
      onReloadRequired?.();
      setError("This review is no longer current. Cancel and import again.");
    } else {
      // A storage error can occur after commit. Never offer blind retry.
      setUncertain(true);
      onReloadRequired?.();
      setError(
        "The import save was not confirmed. Cancel to reload your saved resume before importing again.",
      );
    }
  }
  if (!snapshot)
    return (
      <section aria-label="Import review">
        <p role="status">{error ?? "Loading import review…"}</p>
        <button type="button" disabled={busy} onClick={() => void cancel()}>
          Cancel import
        </button>
      </section>
    );
  return (
    <ImportedResumeEditor
      initialDocument={snapshot.importedDocument}
      style={style}
      busy={busy}
      blocked={
        disabled || uncertain || snapshot.baseRevision !== currentRevision
      }
      error={
        snapshot.baseRevision !== currentRevision
          ? "Your saved resume changed. Cancel and import again."
          : error
      }
      onMap={(document) => void apply(document)}
      onCancel={() => void cancel()}
    />
  );
}
