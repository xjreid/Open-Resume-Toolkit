import { useState, type FormEvent } from "react";
import {
  MAX_BACKUP_PASSPHRASE_BYTES,
  type ValidatedBackup,
} from "@ort/contracts/backup";
import { exportPortableBackup, validatePortableBackup } from "./command-client";
import { backupFeedback, backupValidationFeedback } from "./backup-export";

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
  const [operation, setOperation] = useState<"export" | "validate" | null>(
    null,
  );
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
        Restore into a clean replacement profile is not enabled in this
        checkpoint. Keep this backup and its passphrase safe for that flow.
      </p>
      <form className="backup-form" onSubmit={(event) => void submit(event)}>
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
        current structural checks. Replace-restore is still disabled and the
        active profile remains unchanged.
      </p>
    </section>
  );
}
