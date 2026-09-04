import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type FormEvent,
} from "react";
import {
  DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE,
  type DeleteAllLocalDataCommandResponse,
  type StorageUsage,
} from "@ort/contracts/storage";
import { deleteAllLocalData, requestStorageUsage } from "./command-client";

type UsageState =
  | { kind: "loading" }
  | { kind: "ready"; usage: StorageUsage }
  | { kind: "error" };

export function StoragePanel({
  enabled,
  onDeleteBegin,
  onDeleteFinish,
}: {
  enabled: boolean;
  onDeleteBegin: () => boolean;
  onDeleteFinish: (committed: boolean, freshProfileReady: boolean) => void;
}) {
  const [state, setState] = useState<UsageState>({ kind: "loading" });
  const [confirmation, setConfirmation] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [deletionCommitted, setDeletionCommitted] = useState(false);
  const [deletionMessage, setDeletionMessage] = useState<string | null>(null);
  const deletionResult = useRef<HTMLParagraphElement>(null);

  const refresh = useCallback(async (isCurrent: () => boolean = () => true) => {
    setState({ kind: "loading" });
    const result = await requestStorageUsage();
    if (!isCurrent()) return;
    setState(
      result.ok ? { kind: "ready", usage: result.value } : { kind: "error" },
    );
  }, []);

  useEffect(() => {
    if (!enabled) return;
    let current = true;
    void refresh(() => current);
    return () => {
      current = false;
    };
  }, [enabled, refresh]);

  const usage = state.kind === "ready" ? state.usage : null;
  const canDelete =
    enabled &&
    !deleting &&
    !deletionCommitted &&
    confirmation === DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE;
  const confirmationInvalid =
    confirmation.length > 0 &&
    confirmation !== DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE;

  useEffect(() => {
    if (deletionMessage) deletionResult.current?.focus();
  }, [deletionMessage]);

  async function removeAllLocalData(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canDelete || !onDeleteBegin()) return;
    setDeleting(true);
    const submittedConfirmation = confirmation;
    setConfirmation("");
    const result = await deleteAllLocalData(submittedConfirmation);
    const committed =
      result.ok &&
      (result.value.status === "deleted" ||
        result.value.status === "cleanup_pending");
    const freshProfileReady =
      result.ok &&
      result.value.status === "deleted" &&
      result.value.freshProfileReady;
    setDeletionCommitted(committed);
    setDeleting(false);
    const message = deleteAllLocalDataFeedback(result);
    setDeletionMessage(message);
    onDeleteFinish(committed, freshProfileReady);
  }

  return (
    <section
      className="editor-panel storage-panel"
      aria-labelledby="storage-heading"
    >
      <div className="section-heading">
        <div>
          <p className="eyebrow">Local data</p>
          <h2 id="storage-heading">Encrypted profile storage</h2>
        </div>
        <button
          type="button"
          className="button--secondary button--compact"
          disabled={!enabled || state.kind === "loading"}
          onClick={() => void refresh()}
        >
          Refresh usage
        </button>
      </div>
      <p className="description">
        This content-free inventory counts the active SQLCipher profile and its
        known journal/manifest files. External exports and backups, OS-vault
        items, and in-memory PDF preview bytes are not included.
      </p>
      {state.kind === "loading" ? (
        <p role="status">Reading encrypted profile usage…</p>
      ) : null}
      {state.kind === "error" ? (
        <p className="notice" role="alert">
          Storage usage is unavailable. The profile was not changed.
        </p>
      ) : null}
      {usage ? (
        <>
          <p className="storage-total" role="status">
            <strong>{formatBytes(usage.totalProfileBytes)}</strong> in the
            active profile · database schema {usage.databaseSchema}
          </p>
          <div className="storage-columns">
            <StorageList
              heading="Saved records"
              values={[
                ["Master drafts", usage.drafts],
                ["Published snapshots", usage.publishedSnapshots],
                ["Portable settings", usage.settings],
                ["PDF render manifests", usage.renderManifests],
                ["Diagnostic events", usage.diagnosticEvents],
              ]}
            />
            <StorageList
              heading="Known local files"
              values={[
                ["Encrypted database", formatBytes(usage.databaseBytes)],
                ["Encrypted write-ahead log", formatBytes(usage.walBytes)],
                ["SQLite shared memory", formatBytes(usage.sharedMemoryBytes)],
                [
                  "Non-secret profile manifest",
                  formatBytes(usage.manifestBytes),
                ],
                ["Recovery metadata", formatBytes(usage.recoveryMetadataBytes)],
              ]}
            />
          </div>
          <p className="description storage-guidance">
            File totals can change after saves and SQLite maintenance.
            Diagnostic events are local troubleshooting metadata and are
            excluded from the current portable backup format.
          </p>
        </>
      ) : null}
      <section
        className="storage-danger"
        aria-labelledby="delete-all-local-data-heading"
      >
        <h3 id="delete-all-local-data-heading">Delete all local ORT data</h3>
        <p className="description" id="delete-all-local-data-guidance">
          This permanently deletes the active encrypted profile, draft,
          published snapshot, settings, render history, diagnostics, local
          recovery copies, pending restore data, and their database keys. It
          also discards unsaved edits currently visible in this window.
        </p>
        <p className="description">
          ORT cannot recover this data afterward. Backups and exported PDF,
          DOCX, or text files that you saved outside ORT are not deleted. The
          application itself is not uninstalled.
        </p>
        <form
          className="backup-form"
          aria-busy={deleting}
          onSubmit={(event) => void removeAllLocalData(event)}
        >
          <label className="field">
            Type {DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE} to confirm
            <input
              type="text"
              value={confirmation}
              autoComplete="off"
              aria-describedby={`delete-all-local-data-guidance${
                confirmationInvalid ? " delete-all-local-data-confirmation" : ""
              }`}
              aria-invalid={confirmationInvalid || undefined}
              disabled={!enabled || deleting || deletionCommitted}
              onChange={(event) => setConfirmation(event.target.value)}
            />
          </label>
          <button
            className="button--danger"
            type="submit"
            disabled={!canDelete}
          >
            {deleting
              ? "Deleting all local data…"
              : deletionCommitted
                ? "Local data deletion committed"
                : "Permanently delete all local data"}
          </button>
          {confirmationInvalid ? (
            <p className="field-error" id="delete-all-local-data-confirmation">
              Enter the complete confirmation phrase exactly as shown.
            </p>
          ) : null}
          {deleting ? (
            <p role="status">
              Closing the encrypted profile and deleting exact local records…
            </p>
          ) : null}
          {deletionMessage ? (
            <p
              ref={deletionResult}
              role={deletionCommitted ? "status" : "alert"}
              tabIndex={-1}
            >
              {deletionMessage}
            </p>
          ) : null}
        </form>
      </section>
    </section>
  );
}

export function deleteAllLocalDataFeedback(
  result: DeleteAllLocalDataCommandResponse,
): string {
  if (result.ok) {
    if (result.value.status === "cleanup_pending") {
      return "Deletion was committed, but cleanup is incomplete. Do not enter new data; restart ORT to resume exact local cleanup before a fresh profile is created.";
    }
    return result.value.freshProfileReady
      ? "All local ORT profile and recovery data was deleted. A new empty encrypted profile is ready. External exports and backups were not changed."
      : "All local ORT profile and recovery data was deleted, but a new empty profile could not be opened. Restart ORT before entering new data.";
  }
  switch (result.error.code) {
    case "DELETE_ALL_CONFIRMATION_REQUIRED":
      return "Type the complete deletion confirmation phrase.";
    case "LOCAL_DATA_OPERATION_BUSY":
      return "Another file or recovery operation is active. Finish or cancel it before deleting local data.";
    case "LOCAL_DATA_DELETE_OUTCOME_UNKNOWN":
      return "The deletion outcome is unknown. Do not retry or enter new data; restart ORT so any committed cleanup can resume safely.";
    case "LOCAL_DATA_DELETE_UNSAFE":
      return "Deletion was not started because the local profile boundary is inconsistent or unsafe. No external exports or backups were changed.";
    default:
      return "Local data deletion could not start. The existing profile was reopened when possible; no external exports or backups were changed.";
  }
}

function StorageList({
  heading,
  values,
}: {
  heading: string;
  values: ReadonlyArray<readonly [string, number | string]>;
}) {
  return (
    <section aria-label={heading}>
      <h3>{heading}</h3>
      <dl className="storage-list">
        {values.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

export function formatBytes(bytes: number): string {
  if (bytes < 1_024) return `${bytes} ${bytes === 1 ? "byte" : "bytes"}`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let unit = -1;
  do {
    value /= 1_024;
    unit += 1;
  } while (value >= 1_024 && unit < units.length - 1);
  const precision = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(precision)} ${units[unit]} (${bytes} bytes)`;
}
