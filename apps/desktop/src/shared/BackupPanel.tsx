import { useEffect, useState, type FormEvent } from "react";
import {
  DELETE_SAFETY_CONFIRMATION_PHRASE,
  MAX_BACKUP_PASSPHRASE_BYTES,
  RESTORE_CONFIRMATION_PHRASE,
  ROLLBACK_CONFIRMATION_PHRASE,
  type BackupRecoveryStatus,
  type ValidatedBackup,
} from "@ort/contracts/backup";
import {
  exportPortableBackup,
  deleteSafetyCopy,
  requestBackupRecoveryStatus,
  restorePortableBackup,
  rollbackSafetyCopy,
  validatePortableBackup,
} from "./command-client";
import {
  backupFeedback,
  backupRestoreFeedback,
  backupValidationFeedback,
  deleteSafetyFeedback,
  recoveryStatusFeedback,
  rollbackSafetyFeedback,
} from "./backup-export";

export function BackupPanel({
  blocked,
  dirty,
  onBegin,
  onFinish,
}: {
  blocked: boolean;
  dirty: boolean;
  onBegin: () => boolean;
  onFinish: (message: string) => void;
}) {
  const [passphrase, setPassphrase] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [validationPassphrase, setValidationPassphrase] = useState("");
  const [validated, setValidated] = useState<ValidatedBackup | null>(null);
  const [restorePassphrase, setRestorePassphrase] = useState("");
  const [restoreConfirmation, setRestoreConfirmation] = useState("");
  const [rollbackConfirmation, setRollbackConfirmation] = useState("");
  const [deleteConfirmation, setDeleteConfirmation] = useState("");
  const [restoreStaged, setRestoreStaged] = useState(false);
  const [recovery, setRecovery] = useState<BackupRecoveryStatus | null>(null);
  const [recoveryError, setRecoveryError] = useState<string | null>(null);
  const [operation, setOperation] = useState<
    "export" | "validate" | "restore" | "rollback" | "delete-safety" | null
  >(null);
  const running = operation !== null;
  const byteCount = new TextEncoder().encode(passphrase).byteLength;
  const confirmationMatches = passphrase === confirmation;
  const passphraseValid =
    byteCount > 0 && byteCount <= MAX_BACKUP_PASSPHRASE_BYTES;
  const canSubmit =
    !blocked && !running && passphraseValid && confirmationMatches;
  const validationByteCount = new TextEncoder().encode(
    validationPassphrase,
  ).byteLength;
  const canValidate =
    !blocked &&
    !running &&
    validationByteCount > 0 &&
    validationByteCount <= MAX_BACKUP_PASSPHRASE_BYTES;
  const restoreByteCount = new TextEncoder().encode(
    restorePassphrase,
  ).byteLength;
  const canRestore =
    !blocked &&
    !running &&
    !restoreStaged &&
    recovery !== null &&
    !recovery.safetyCopyAvailable &&
    !recovery.restartOperationPending &&
    !recovery.safetyCleanupPending &&
    restoreByteCount > 0 &&
    restoreByteCount <= MAX_BACKUP_PASSPHRASE_BYTES &&
    restoreConfirmation === RESTORE_CONFIRMATION_PHRASE;
  const canRollback =
    !blocked &&
    !running &&
    recovery?.safetyCopyAvailable === true &&
    !recovery.restartOperationPending &&
    !recovery.safetyCleanupPending &&
    rollbackConfirmation === ROLLBACK_CONFIRMATION_PHRASE;
  const canDeleteSafety =
    !blocked &&
    !running &&
    recovery?.safetyCopyAvailable === true &&
    !recovery.restartOperationPending &&
    !recovery.safetyCleanupPending &&
    deleteConfirmation === DELETE_SAFETY_CONFIRMATION_PHRASE;
  const rollbackConfirmationInvalid =
    rollbackConfirmation.length > 0 &&
    rollbackConfirmation !== ROLLBACK_CONFIRMATION_PHRASE;
  const deleteConfirmationInvalid =
    deleteConfirmation.length > 0 &&
    deleteConfirmation !== DELETE_SAFETY_CONFIRMATION_PHRASE;
  const restoreConfirmationInvalid =
    restoreConfirmation.length > 0 &&
    restoreConfirmation !== RESTORE_CONFIRMATION_PHRASE;

  async function refreshRecovery() {
    const result = await requestBackupRecoveryStatus();
    if (result.ok) {
      setRecovery(result.value);
      setRecoveryError(null);
    } else {
      setRecovery(null);
      setRecoveryError(recoveryStatusFeedback(result));
    }
  }

  useEffect(() => {
    if (!blocked) void refreshRecovery();
  }, [blocked]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || !onBegin()) return;
    setOperation("export");
    const submittedPassphrase = passphrase;
    // Clear visible controlled fields immediately. The native command owns and
    // zeroizes its copy after the one requested operation.
    setPassphrase("");
    setConfirmation("");
    const result = await exportPortableBackup(submittedPassphrase);
    setOperation(null);
    onFinish(backupFeedback(result));
  }

  async function validate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canValidate || !onBegin()) return;
    setOperation("validate");
    setValidated(null);
    const submittedPassphrase = validationPassphrase;
    setValidationPassphrase("");
    const result = await validatePortableBackup(submittedPassphrase);
    if (result.ok && result.value.status === "validated") {
      setValidated(result.value);
    }
    setOperation(null);
    onFinish(backupValidationFeedback(result));
  }

  async function restore(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canRestore || !onBegin()) return;
    setOperation("restore");
    const submittedPassphrase = restorePassphrase;
    const submittedConfirmation = restoreConfirmation;
    setRestorePassphrase("");
    setRestoreConfirmation("");
    const result = await restorePortableBackup(
      submittedPassphrase,
      submittedConfirmation,
    );
    if (result.ok && result.value.status === "staged") {
      setRestoreStaged(true);
      setRecovery((current) =>
        current ? { ...current, restartOperationPending: true } : current,
      );
    }
    setOperation(null);
    onFinish(backupRestoreFeedback(result));
  }

  async function rollback(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canRollback || !onBegin()) return;
    setOperation("rollback");
    const submittedConfirmation = rollbackConfirmation;
    setRollbackConfirmation("");
    const result = await rollbackSafetyCopy(submittedConfirmation);
    if (result.ok) {
      setRecovery((current) =>
        current ? { ...current, restartOperationPending: true } : current,
      );
    }
    setOperation(null);
    onFinish(rollbackSafetyFeedback(result));
  }

  async function removeSafety(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canDeleteSafety || !onBegin()) return;
    setOperation("delete-safety");
    const submittedConfirmation = deleteConfirmation;
    setDeleteConfirmation("");
    const result = await deleteSafetyCopy(submittedConfirmation);
    if (result.ok) {
      setRecovery((current) =>
        current
          ? {
              ...current,
              safetyCopyAvailable: false,
              safetyCleanupPending: false,
            }
          : current,
      );
    }
    setOperation(null);
    onFinish(deleteSafetyFeedback(result));
  }

  const validation =
    byteCount > MAX_BACKUP_PASSPHRASE_BYTES
      ? "The passphrase is over the 1,024-byte limit."
      : confirmation && !confirmationMatches
        ? "The passphrases do not match."
        : null;

  return (
    <section
      className="editor-panel backup-panel"
      aria-labelledby="backup-heading"
    >
      <div className="section-heading">
        <div>
          <p className="eyebrow">Data recovery</p>
          <h2 id="backup-heading">Encrypted portable backup</h2>
        </div>
      </div>
      <p className="description" id="backup-description">
        This packages saved resume data, settings, published snapshots, and PDF
        render history. Device keys and future provider credentials are
        excluded. The passphrase cannot be recovered by ORT. A synced
        destination receives the encrypted archive, and existing files are never
        replaced.
      </p>
      <p className="description">
        A restore is prepared in a fresh encrypted profile and activated only
        after restart. The replaced encrypted profile remains on this device as
        a safety copy; external exports and backups are never changed.
      </p>
      <form
        className="backup-form"
        aria-busy={operation === "export"}
        onSubmit={(event) => void submit(event)}
      >
        <label className="field">
          Backup passphrase
          <input
            type="password"
            value={passphrase}
            autoComplete="new-password"
            aria-describedby="backup-description backup-guidance"
            disabled={blocked || running}
            onChange={(event) => setPassphrase(event.target.value)}
          />
        </label>
        <label className="field">
          Confirm passphrase
          <input
            type="password"
            value={confirmation}
            autoComplete="new-password"
            aria-describedby={validation ? "backup-validation" : undefined}
            aria-invalid={validation ? true : undefined}
            disabled={blocked || running}
            onChange={(event) => setConfirmation(event.target.value)}
          />
        </label>
        <p className="description backup-guidance" id="backup-guidance">
          Use a long, unique passphrase and store it separately from the backup.
          {dirty ? " Save the current edits before creating the backup." : ""}
        </p>
        {validation ? (
          <p className="field-error" id="backup-validation" role="alert">
            {validation}
          </p>
        ) : null}
        <button type="submit" disabled={!canSubmit}>
          {operation === "export"
            ? "Creating encrypted backup…"
            : "Create encrypted backup"}
        </button>
        {operation === "export" ? (
          <p role="status">
            Finish or cancel the native Save dialog. Backup encryption may take
            a moment.
          </p>
        ) : null}
      </form>

      <section
        className="backup-validation"
        aria-labelledby="backup-check-heading"
      >
        <h3 id="backup-check-heading">Check an existing backup</h3>
        <p className="description" id="backup-check-description">
          Select a backup through the native file dialog. ORT reads at most the
          fixed backup limit, authenticates and validates the complete encrypted
          archive, and returns only its content-free inventory. This does not
          replace or write to the active profile.
        </p>
        <form
          className="backup-form"
          aria-busy={operation === "validate"}
          onSubmit={(event) => void validate(event)}
        >
          <label className="field backup-validation__passphrase">
            Existing backup passphrase
            <input
              type="password"
              value={validationPassphrase}
              autoComplete="current-password"
              aria-describedby="backup-check-description backup-check-guidance"
              aria-invalid={validationByteCount > MAX_BACKUP_PASSPHRASE_BYTES}
              disabled={blocked || running}
              onChange={(event) => setValidationPassphrase(event.target.value)}
            />
          </label>
          <p className="description backup-guidance" id="backup-check-guidance">
            Wrong passphrases, damaged files, and unsupported backup contents
            share the same result. The selected path is never returned to this
            interface.
          </p>
          {validationByteCount > MAX_BACKUP_PASSPHRASE_BYTES ? (
            <p className="field-error" role="alert">
              The passphrase is over the 1,024-byte limit.
            </p>
          ) : null}
          <button type="submit" disabled={!canValidate}>
            {operation === "validate"
              ? "Checking encrypted backup…"
              : "Select and check encrypted backup"}
          </button>
          {operation === "validate" ? (
            <p role="status">
              Finish or cancel the native file dialog. Authentication may take a
              moment.
            </p>
          ) : null}
        </form>
        {validated ? <ValidatedBackupSummary backup={validated} /> : null}
      </section>

      <section
        className="backup-validation"
        aria-labelledby="backup-recovery-heading"
      >
        <h3 id="backup-recovery-heading">Local recovery safety copy</h3>
        {recoveryError ? (
          <p role="alert" className="field-error">
            {recoveryError}
          </p>
        ) : recovery ? (
          <p role="status" className="description">
            {recovery.restartOperationPending
              ? "A verified replacement or rollback is waiting for restart."
              : recovery.safetyCleanupPending
                ? "Confirmed safety-copy cleanup will resume at startup."
                : recovery.safetyCopyAvailable
                  ? "One encrypted safety copy is retained on this device."
                  : "No retained safety copy is present."}
          </p>
        ) : (
          <p role="status" className="description">
            Checking local recovery state…
          </p>
        )}
        <p className="description" id="rollback-guidance">
          Rollback verifies and stages the retained profile for activation after
          restart. The current profile then becomes the new safety copy.
        </p>
        <form
          className="backup-form"
          aria-busy={operation === "rollback"}
          onSubmit={(event) => void rollback(event)}
        >
          <label className="field">
            Type {ROLLBACK_CONFIRMATION_PHRASE} to confirm rollback
            <input
              type="text"
              value={rollbackConfirmation}
              autoComplete="off"
              aria-describedby={`rollback-guidance${
                rollbackConfirmationInvalid ? " rollback-confirmation" : ""
              }`}
              aria-invalid={rollbackConfirmationInvalid || undefined}
              disabled={blocked || running || !recovery?.safetyCopyAvailable}
              onChange={(event) => setRollbackConfirmation(event.target.value)}
            />
          </label>
          <button type="submit" disabled={!canRollback}>
            {operation === "rollback"
              ? "Preparing rollback…"
              : "Roll back after restart"}
          </button>
          {rollbackConfirmationInvalid ? (
            <p className="field-error" id="rollback-confirmation">
              Enter the complete rollback phrase exactly as shown.
            </p>
          ) : null}
        </form>
        <p className="description" id="safety-delete-guidance">
          Deleting the safety copy is permanent and removes its exact encrypted
          profile directory and OS-vault key. It does not change the active
          profile or delete exports and backups saved elsewhere.
        </p>
        <form
          className="backup-form"
          aria-busy={operation === "delete-safety"}
          onSubmit={(event) => void removeSafety(event)}
        >
          <label className="field">
            Type {DELETE_SAFETY_CONFIRMATION_PHRASE} to delete it
            <input
              type="text"
              value={deleteConfirmation}
              autoComplete="off"
              aria-describedby={`safety-delete-guidance${
                deleteConfirmationInvalid ? " safety-delete-confirmation" : ""
              }`}
              aria-invalid={deleteConfirmationInvalid || undefined}
              disabled={blocked || running || !recovery?.safetyCopyAvailable}
              onChange={(event) => setDeleteConfirmation(event.target.value)}
            />
          </label>
          <button type="submit" disabled={!canDeleteSafety}>
            {operation === "delete-safety"
              ? "Deleting safety copy…"
              : "Permanently delete safety copy"}
          </button>
          {deleteConfirmationInvalid ? (
            <p className="field-error" id="safety-delete-confirmation">
              Enter the complete deletion phrase exactly as shown.
            </p>
          ) : null}
        </form>
      </section>

      <section
        className="backup-validation"
        aria-labelledby="backup-restore-heading"
      >
        <h3 id="backup-restore-heading">Replace saved profile from backup</h3>
        <p className="description" id="backup-restore-description">
          This replaces the draft, published snapshots, settings, and render
          history after restart. Save current edits first. ORT authenticates the
          selected archive and imports it into a separately keyed encrypted
          staging profile before scheduling any replacement.
        </p>
        <form
          className="backup-form"
          aria-busy={operation === "restore"}
          onSubmit={(event) => void restore(event)}
        >
          <label className="field backup-validation__passphrase">
            Backup passphrase
            <input
              type="password"
              value={restorePassphrase}
              autoComplete="current-password"
              aria-describedby="backup-restore-description backup-restore-guidance"
              aria-invalid={restoreByteCount > MAX_BACKUP_PASSPHRASE_BYTES}
              disabled={blocked || running || restoreStaged}
              onChange={(event) => setRestorePassphrase(event.target.value)}
            />
          </label>
          <label className="field">
            Type {RESTORE_CONFIRMATION_PHRASE} to confirm
            <input
              type="text"
              value={restoreConfirmation}
              autoComplete="off"
              aria-describedby={
                restoreConfirmationInvalid
                  ? "backup-restore-guidance restore-confirmation"
                  : "backup-restore-guidance"
              }
              aria-invalid={restoreConfirmationInvalid || undefined}
              disabled={blocked || running || restoreStaged}
              onChange={(event) => setRestoreConfirmation(event.target.value)}
            />
          </label>
          <p
            className="description backup-guidance"
            id="backup-restore-guidance"
          >
            Merge restore is not supported. Restart promptly after staging; the
            current profile remains active until then.
          </p>
          {restoreConfirmationInvalid ? (
            <p className="field-error" id="restore-confirmation">
              Enter the complete replacement phrase exactly as shown.
            </p>
          ) : null}
          {restoreByteCount > MAX_BACKUP_PASSPHRASE_BYTES ? (
            <p className="field-error" role="alert">
              The passphrase is over the 1,024-byte limit.
            </p>
          ) : null}
          <button type="submit" disabled={!canRestore}>
            {operation === "restore"
              ? "Preparing encrypted replacement…"
              : restoreStaged
                ? "Restart ORT to finish restore"
                : "Select backup and replace after restart"}
          </button>
          {operation === "restore" ? (
            <p role="status">
              Finish or cancel the native file dialog. Authentication and
              encrypted staging may take a moment.
            </p>
          ) : null}
        </form>
      </section>
    </section>
  );
}

function ValidatedBackupSummary({ backup }: { backup: ValidatedBackup }) {
  return (
    <section
      className="backup-summary"
      aria-labelledby="validated-backup-heading"
    >
      <h3 id="validated-backup-heading">Authenticated backup summary</h3>
      <dl className="storage-list">
        <div>
          <dt>Container</dt>
          <dd>
            Format {backup.formatMajor}.{backup.formatMinor} ·{" "}
            {backup.byteCount} bytes
          </dd>
        </div>
        <div>
          <dt>Created</dt>
          <dd>{backup.createdAt}</dd>
        </div>
        <div>
          <dt>Application version</dt>
          <dd>{backup.appVersion}</dd>
        </div>
        <div>
          <dt>Schema</dt>
          <dd>
            Database {backup.databaseSchema} · document {backup.documentSchema}
          </dd>
        </div>
        <div>
          <dt>Saved records</dt>
          <dd>
            {backup.masterDrafts} draft · {backup.publishedResumes} published ·{" "}
            {backup.settings} settings · {backup.renderManifests} render records
          </dd>
        </div>
      </dl>
      <p className="description">
        This summary proves that the selected archive authenticated and passed
        current structural checks. The active profile remains unchanged unless
        you separately confirm and stage a replacement below.
      </p>
    </section>
  );
}
