import { useEffect, useRef, useState } from "react";
import type {
  ImportChoices,
  ImportReviewSnapshot,
} from "@ort/contracts/import";
import type { VersionedResume } from "@ort/contracts/resume";
import {
  readImportReview,
  applyImportReview,
  cancelImportReview,
} from "./import-client";
import { ImportReviewPanel } from "./ImportReviewPanel";

// Mount only with a native-created review ID. No file/parser initiation here.
export function ImportReviewFlow(props: {
  reviewId: string;
  currentRevision: number;
  onSaved: (saved: VersionedResume) => void;
  onCancelled: () => void;
}) {
  return <ReviewFlow key={props.reviewId} {...props} />;
}
function ReviewFlow({
  reviewId,
  currentRevision,
  onSaved,
  onCancelled,
}: Parameters<typeof ImportReviewFlow>[0]) {
  const inFlight = useRef(false);
  const mounted = useRef(true);
  const [snapshot, setSnapshot] = useState<ImportReviewSnapshot | null>(null);
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  useEffect(() => {
    let active = true;
    mounted.current = true;
    void readImportReview(reviewId).then((response) => {
      if (!active) return;
      if (response.ok && response.value.id === reviewId)
        setSnapshot(response.value);
      else
        setError("This import review is unavailable. Cancel and start again.");
    });
    return () => {
      active = false;
      mounted.current = false;
    };
  }, [reviewId]);
  async function cancel() {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    const response = await cancelImportReview(reviewId);
    inFlight.current = false;
    if (!mounted.current) return;
    setBusy(false);
    if (response.ok) onCancelled();
    else
      setError(
        "Cancellation could not be confirmed. Close the app to discard its in-memory review.",
      );
  }
  async function apply(choices: ImportChoices) {
    if (
      inFlight.current ||
      uncertain ||
      !snapshot ||
      snapshot.baseRevision !== currentRevision
    )
      return;
    inFlight.current = true;
    setBusy(true);
    setError(undefined);
    const response = await applyImportReview(reviewId, choices);
    inFlight.current = false;
    if (!mounted.current) return;
    setBusy(false);
    if (response.ok) {
      onSaved(response.value);
    } else {
      // A storage error can occur after commit. Never offer blind retry.
      setUncertain(true);
      setError(
        "The import save was not confirmed. Check the current saved draft before starting another import. This review will not submit again.",
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
    <>
      {uncertain && <p role="alert">{error}</p>}
      <ImportReviewPanel
        session={snapshot}
        currentRevision={currentRevision}
        busy={busy}
        error={uncertain ? undefined : error}
        submissionBlocked={uncertain}
        onSubmit={(choices) => void apply(choices)}
        onCancel={() => void cancel()}
      />
    </>
  );
}
