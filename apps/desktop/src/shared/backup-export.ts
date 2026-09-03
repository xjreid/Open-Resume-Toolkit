import type {
  BackupRecoveryStatusCommandResponse,
  DeleteSafetyCopyCommandResponse,
  ExportBackupCommandResponse,
  RestoreBackupCommandResponse,
  RollbackSafetyCopyCommandResponse,
  ValidateBackupCommandResponse,
} from "@ort/contracts/backup";

export function backupFeedback(result: ExportBackupCommandResponse): string {
  if (!result.ok) {
    switch (result.error.code) {
      case "BACKUP_ALREADY_EXISTS":
        return "Nothing was overwritten. Create the backup again with a new filename.";
      case "BACKUP_INVALID_DESTINATION":
        return "Choose a new regular .ort-backup filename; special names are not supported.";
      case "INVALID_BACKUP_PASSPHRASE":
        return "Enter a nonempty backup passphrase of at most 1,024 UTF-8 bytes.";
      case "BACKUP_BUSY":
        return "Another file operation is active. Finish or cancel its native dialog first.";
      case "BACKUP_INVALID_CONTENT":
        return "The saved profile could not be packaged safely. The existing profile was not changed.";
      default:
        return "Backup creation could not be confirmed. Check the chosen folder before retrying; a hidden .ort-export-* staging folder may remain after an interrupted write.";
    }
  }
  if (result.value.status === "cancelled")
    return "Backup canceled. No file was written.";
  const value = result.value;
  return (
    `Created encrypted portable backup format ${value.formatMajor}.${value.formatMinor} (${value.byteCount} bytes). Keep the file and its unrecoverable passphrase separately.` +
    (value.cleanupPending
      ? " A hidden .ort-export-* staging folder remains in the chosen folder; it contains the same encrypted backup bytes."
      : "") +
    (value.durabilityUnconfirmed
      ? " File written, but this filesystem could not confirm directory durability against power loss."
      : "")
  );
}

export function recoveryStatusFeedback(
  result: BackupRecoveryStatusCommandResponse,
): string | null {
  if (result.ok) return null;
  return "Local recovery status could not be verified. No recovery data was changed.";
}

export function rollbackSafetyFeedback(
  result: RollbackSafetyCopyCommandResponse,
): string {
  if (!result.ok) {
    switch (result.error.code) {
      case "SAFETY_COPY_CONFIRMATION_REQUIRED":
        return "Type the complete rollback confirmation phrase.";
      case "SAFETY_COPY_NOT_FOUND":
        return "No retained safety copy is available to roll back.";
      case "RECOVERY_BUSY":
      case "BACKUP_BUSY":
        return "Another recovery or file operation is active. Finish it or restart ORT first.";
      case "SAFETY_COPY_UNAVAILABLE":
        return "The retained safety copy could not be opened and verified. The active profile was not changed.";
      default:
        return "Rollback could not be prepared. The active profile was not changed.";
    }
  }
  return "Rollback is verified and ready. Restart ORT to activate the retained profile; the current profile will become the new safety copy.";
}

export function deleteSafetyFeedback(
  result: DeleteSafetyCopyCommandResponse,
): string {
  if (!result.ok) {
    switch (result.error.code) {
      case "SAFETY_COPY_CONFIRMATION_REQUIRED":
        return "Type the complete safety-copy deletion phrase.";
      case "RECOVERY_BUSY":
      case "BACKUP_BUSY":
        return "Another recovery or file operation is active. Finish it or restart ORT first.";
      case "SAFETY_COPY_UNAVAILABLE":
        return "The retained safety copy or its vault key could not be removed safely. The active profile was not changed.";
      default:
        return "Safety-copy deletion could not be confirmed. The active profile was not changed.";
    }
  }
  return result.value.deleted
    ? "The retained encrypted safety copy and its vault key were deleted. The active profile and external exports or backups were not changed."
    : "No retained safety copy was present. Nothing was deleted.";
}

export function backupRestoreFeedback(
  result: RestoreBackupCommandResponse,
): string {
  if (!result.ok) {
    switch (result.error.code) {
      case "INVALID_BACKUP_PASSPHRASE":
        return "Enter the backup passphrase, up to 1,024 UTF-8 bytes.";
      case "RESTORE_CONFIRMATION_REQUIRED":
        return "Type the complete confirmation phrase before replacing saved data.";
      case "BACKUP_BUSY":
        return "Another file operation is active. Finish or cancel it first.";
      case "BACKUP_INVALID_OR_PASSPHRASE":
        return "The backup could not be authenticated. The passphrase may be incorrect, or the file may be damaged or unsupported. The active profile was not changed.";
      case "BACKUP_READ_UNAVAILABLE":
        return "The selected backup could not be read. The active profile was not changed.";
      case "RESTORE_RECOVERY_PENDING":
        return "A staged operation or retained safety copy already exists. Restart a pending operation, or use the recovery controls before another restore.";
      default:
        return "The replacement could not be staged. The active profile was not changed; select the file again before retrying.";
    }
  }
  if (result.value.status === "cancelled")
    return "Restore canceled. The active profile was not changed.";
  return "Backup authenticated and imported into a fresh encrypted profile. Restart ORT to activate it. The current encrypted profile will be retained as a local safety copy.";
}

export function backupValidationFeedback(
  result: ValidateBackupCommandResponse,
): string {
  if (!result.ok) {
    switch (result.error.code) {
      case "INVALID_BACKUP_PASSPHRASE":
        return "Enter the backup passphrase, up to 1,024 UTF-8 bytes.";
      case "BACKUP_BUSY":
        return "Another file operation is active. Finish or cancel its native dialog first.";
      case "BACKUP_INVALID_OR_PASSPHRASE":
        return "The backup could not be authenticated. The passphrase may be incorrect, or the file may be damaged or unsupported. The active profile was not changed.";
      case "BACKUP_READ_UNAVAILABLE":
        return "The selected backup could not be read. The active profile was not changed.";
      default:
        return "Backup validation could not be confirmed. The active profile was not changed; select the file again before retrying.";
    }
  }
  if (result.value.status === "cancelled")
    return "Backup validation canceled. The active profile was not changed.";
  return `Authenticated backup format ${result.value.formatMajor}.${result.value.formatMinor} (${result.value.byteCount} bytes). The active profile was not changed; replacement requires the separate confirmation below.`;
}
