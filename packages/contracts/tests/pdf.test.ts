import { expect, it } from "vitest";
import {
  isPdfPreviewCommandResponse,
  isPdfExportCommandResponse,
  MAX_PDF_BYTES,
} from "../generated/pdf";

const preview = {
  renderId: "019a0000-0000-7000-8000-000000000001",
  source: "saved_draft",
  revision: 1,
  generatedAtUnixMs: 1000,
  pdfBase64: "JVBERi0=",
  receipt: {
    documentSha256: "a".repeat(64),
    documentSchemaVersion: 1,
    pdfSha256: "b".repeat(64),
    rendererVersion: "typst-0.15.1/ort-1",
    templateId: "plain_pdf_v1",
    templateSha256: "c".repeat(64),
    fontBundleId: "libertinus-serif/typst-assets-0.15.1",
    fontBundleSha256: "d".repeat(64),
    pageCount: 1,
    byteCount: 5,
  },
};
const wrap = (value: unknown) => ({ ok: true, value });
it("accepts only fixed bounded PDF metadata and encoded byte lengths", () => {
  expect(isPdfPreviewCommandResponse(wrap(preview))).toBe(true);
  for (const value of [
    { ...preview, path: "private" },
    { ...preview, renderId: "not-a-ticket" },
    { ...preview, revision: 0 },
    { ...preview, source: "unsaved" },
    { ...preview, generatedAtUnixMs: Infinity },
    { ...preview, pdfBase64: "JVBE=i0=" },
    { ...preview, pdfBase64: "JVBERi0===?" },
    { ...preview, pdfBase64: "JVBERi0-" },
    ...[
      { pageCount: 6 },
      { pageCount: 0 },
      { byteCount: MAX_PDF_BYTES + 1 },
      { byteCount: 4 },
      { byteCount: NaN },
      { rendererVersion: "untrusted" },
      { templateId: "user_template" },
      { fontBundleId: "system" },
      { pdfSha256: "short" },
      { documentSchemaVersion: 2 },
      { path: "private" },
    ].map((changes) => ({
      ...preview,
      receipt: { ...preview.receipt, ...changes },
    })),
  ])
    expect(isPdfPreviewCommandResponse(wrap(value))).toBe(false);
});
it("accepts path-free PDF export receipts and rejects unexpected data", () => {
  const receipt = {
    status: "exported",
    renderId: preview.renderId,
    pdfSha256: preview.receipt.pdfSha256,
    byteCount: 5,
    cleanupPending: false,
    durabilityUnconfirmed: true,
  };
  expect(isPdfExportCommandResponse(wrap(receipt))).toBe(true);
  expect(isPdfExportCommandResponse(wrap({ status: "cancelled" }))).toBe(true);
  for (const value of [
    { ...receipt, byteCount: MAX_PDF_BYTES + 1 },
    { ...receipt, pdfBase64: "data" },
    { ...receipt, renderId: "bad" },
    { ...receipt, pdfSha256: "bad" },
    { ...receipt, cleanupPending: 0 },
    { status: "cancelled", path: "private" },
  ])
    expect(isPdfExportCommandResponse(wrap(value))).toBe(false);
});
