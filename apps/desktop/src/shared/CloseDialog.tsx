import { useEffect, useRef } from "react";

export function CloseDialog({
  open,
  busy,
  resolving,
  canSave,
  error,
  saveError,
  onCancel,
  onSave,
  onDiscard,
  onRetry,
}: {
  open: boolean;
  busy: boolean;
  resolving: boolean;
  canSave: boolean;
  error: string | null;
  saveError: string | null;
  onCancel: () => void;
  onSave: () => void;
  onDiscard: () => void;
  onRetry: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const keep = useRef<HTMLButtonElement>(null);
  const restoreFocus = useRef<HTMLElement | null>(null);
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) {
      restoreFocus.current =
        document.activeElement instanceof HTMLElement &&
        document.activeElement !== document.body
          ? document.activeElement
          : null;
      element.showModal();
      keep.current?.focus();
    } else if (!open && element.open) {
      element.close();
      if (restoreFocus.current?.isConnected) restoreFocus.current.focus();
      restoreFocus.current = null;
    }
  }, [open]);

  useEffect(
    () => () => {
      if (restoreFocus.current?.isConnected) restoreFocus.current.focus();
    },
    [],
  );

  return (
    <dialog
      ref={dialog}
      className="close-dialog"
      aria-labelledby="close-title"
      aria-describedby="close-description"
      onCancel={(event) => {
        event.preventDefault();
        if (!resolving) onCancel();
      }}
    >
      <h2 id="close-title">Quit Open Resume Toolkit?</h2>
      <p id="close-description">
        Unsaved edits will be lost if you discard them. Published snapshots and
        previously saved drafts are kept.
      </p>
      {busy ? (
        <p role="status">
          Waiting for the current operation to finish. The app will stay open if
          saving fails.
        </p>
      ) : null}
      {!canSave && !busy ? (
        <p>
          To save, keep editing and correct any validation or storage errors
          first.
        </p>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}
      {saveError ? (
        <p role="alert">
          {saveError} The app has not quit. Keep editing to recover, or
          explicitly discard unsaved edits.
        </p>
      ) : null}
      <div className="editor-tools">
        <button
          ref={keep}
          type="button"
          disabled={resolving}
          onClick={onCancel}
        >
          Keep editing
        </button>
        <button
          type="button"
          className="button--secondary"
          disabled={!canSave || busy || resolving || !!error}
          onClick={onSave}
        >
          Save and quit
        </button>
        <button
          type="button"
          className="button--danger"
          disabled={busy || resolving || !!error}
          onClick={onDiscard}
        >
          Discard unsaved edits and quit
        </button>
        {error ? (
          <button type="button" disabled={resolving} onClick={onRetry}>
            Retry quit connection
          </button>
        ) : null}
      </div>
      {resolving ? <p role="status">Confirming with the desktop…</p> : null}
    </dialog>
  );
}
