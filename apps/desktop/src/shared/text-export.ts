import type { ExportTextCommandResponse } from "@ort/contracts/export";

export function exportFeedback(result: ExportTextCommandResponse): string {
  if (!result.ok) {
    switch (result.error.code) {
      case "EXPORT_ALREADY_EXISTS":
        return "Nothing was overwritten. Export again with a new filename.";
      case "EXPORT_INVALID_DESTINATION":
        return "Choose a new regular .txt filename; special names are not supported.";
      case "EXPORT_INVALID_CONTENT":
        return "There is no exportable content, or the saved content contains unsupported control characters. Review the resume before exporting.";
      case "REVISION_CONFLICT":
        return "The saved revision changed. Reload the workspace before exporting; keep any unsaved edits first.";
      case "EXPORT_BUSY":
        return "Another export is still active. Finish or cancel its Save dialog first.";
      case "DRAFT_NOT_FOUND":
        return "That saved draft or published snapshot is no longer available. Reload the workspace.";
      default:
        return "Export could not be confirmed. Check your chosen folder before retrying; this filesystem may not support safe export. A hidden .ort-export-* staging folder may remain after an interrupted write.";
    }
  }
  if (result.value.status === "cancelled")
    return "Export canceled. No file was written.";
  const value = result.value;
  const source =
    value.source === "saved_draft"
      ? "saved draft revision"
      : "published snapshot";
  return (
    `Exported ${source} ${value.revision} as unencrypted UTF-8 text (${value.byteCount} bytes).` +
    (value.cleanupPending
      ? " A hidden .ort-export-* staging folder remains in the chosen folder; it contains the same unencrypted text."
      : "") +
    (value.durabilityUnconfirmed
      ? " File written, but this filesystem could not confirm directory durability against power loss."
      : "")
  );
}
