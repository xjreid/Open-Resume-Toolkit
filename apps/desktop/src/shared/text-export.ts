import type {
  ExportFormat,
  ExportDocxCommandResponse,
} from "@ort/contracts/export";

export function exportFeedback(
  result: ExportDocxCommandResponse,
  format: ExportFormat = "txt",
): string {
  if (!result.ok) {
    switch (result.error.code) {
      case "EXPORT_ALREADY_EXISTS":
        return "Nothing was overwritten. Export again with a new filename.";
      case "EXPORT_INVALID_DESTINATION":
        return `Choose a new regular .${format} filename; special names are not supported.`;
      case "EXPORT_INVALID_CONTENT":
        return "The saved content is empty, contains unsupported characters or links, or is too large for this format. Review the resume before exporting.";
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
  const docxLayout = value.templateId
    ? {
        technical_docx_v1: "Technical / Engineering v1",
        professional_docx_v1: "Professional / Business v1",
        modern_docx_v1: "Modern / Marketing & Sales v1",
      }[value.templateId]
    : "plain layout v1";
  return (
    `Exported ${source} ${value.revision} as unencrypted ${format === "docx" ? `DOCX (${docxLayout})` : "UTF-8 text"} (${value.byteCount} bytes).` +
    (value.cleanupPending
      ? " A hidden .ort-export-* staging folder remains in the chosen folder; it contains the same unencrypted document."
      : "") +
    (value.durabilityUnconfirmed
      ? " File written, but this filesystem could not confirm directory durability against power loss."
      : "")
  );
}
