import type {
  ExportBackupCommandResponse,
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
  return `Authenticated backup format ${result.value.formatMajor}.${result.value.formatMinor} (${result.value.byteCount} bytes). The active profile was not changed; replace-restore remains disabled.`;
}
