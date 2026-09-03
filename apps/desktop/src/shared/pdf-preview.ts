import { PDF_PREVIEW_TTL_SECONDS, type PdfPreview } from "@ort/contracts/pdf";

export function previewExpired(preview: PdfPreview, now: number): boolean {
  return now >= preview.generatedAtUnixMs + PDF_PREVIEW_TTL_SECONDS * 1000;
}

export function previewIsStale(
  preview: PdfPreview,
  revision: number | null,
  dirty: boolean,
): boolean {
  return (
    preview.revision !== revision || (preview.source === "saved_draft" && dirty)
  );
}

export async function verifiedPdfBytes(
  preview: PdfPreview,
): Promise<Uint8Array<ArrayBuffer>> {
  const bytes = Uint8Array.from(atob(preview.pdfBase64), (c) =>
    c.charCodeAt(0),
  );
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const hash = Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join(
    "",
  );
  if (
    bytes.length !== preview.receipt.byteCount ||
    hash !== preview.receipt.pdfSha256 ||
    String.fromCharCode(...bytes.subarray(0, 5)) !== "%PDF-"
  )
    throw new Error("PDF integrity check failed");
  return bytes;
}

export function pdfFailure(code: string): string {
  switch (code) {
    case "PDF_UNSUPPORTED_GLYPH":
      return "Some characters are not covered by the bundled PDF fonts. No PDF was produced. Text and DOCX export remain available.";
    case "PDF_LAYOUT_LIMIT":
      return "This content exceeds the five-page PDF or layout complexity limit. Nothing was truncated. Shorten the resume or use text/DOCX export.";
    case "PDF_BYTE_LIMIT":
      return "The PDF exceeds the 4 MiB development limit. No file was exported.";
    case "PDF_PREVIEW_EXPIRED":
    case "REVISION_CONFLICT":
      return "This preview has expired or its saved revision changed. Generate a fresh preview before exporting.";
    case "EXPORT_ALREADY_EXISTS":
      return "That filename already exists. It was not replaced; choose a new filename.";
    case "EXPORT_INVALID_DESTINATION":
      return "Choose a new local filename ending in .pdf. No file was replaced.";
    case "EXPORT_OUTCOME_UNKNOWN":
    case "COMMAND_UNAVAILABLE":
    case "INVALID_RESPONSE":
      return "The PDF operation could not be confirmed. If a Save dialog was completed, inspect the destination before retrying. Your saved resume is unchanged.";
    default:
      return "The PDF operation could not finish. Your saved resume is unchanged; try a fresh preview or use text/DOCX export.";
  }
}
