import { createHash } from "node:crypto";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import type { PdfPreview } from "@ort/contracts/pdf";
import { PdfPreviewPanel } from "./PdfPreview";
import {
  previewExpired,
  previewIsStale,
  verifiedPdfBytes,
  pdfFailure,
} from "./pdf-preview";
import { createResumeDocument } from "./resume-editor";
import { editorReducer, initialEditorState } from "./editor-state";
import { closeDisposition } from "./close-policy";
import {
  renderResumePdf,
  exportResumePdf,
  requestPdfRenderHistory,
} from "./command-client";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const preview: PdfPreview = {
  renderId: "019a0000-0000-7000-8000-000000000001",
  source: "saved_draft",
  revision: 1,
  generatedAtUnixMs: 1000,
  pdfBase64: "JVBERi0=",
  receipt: {
    documentSha256: "a".repeat(64),
    documentSchemaVersion: 1,
    pdfSha256: createHash("sha256").update("%PDF-").digest("hex"),
    rendererVersion: "typst-0.15.1/ort-1",
    templateId: "plain_pdf_v1",
    templateSha256: "c".repeat(64),
    fontBundleId: "libertinus-serif/typst-assets-0.15.1",
    fontBundleSha256: "d".repeat(64),
    pageCount: 1,
    byteCount: 5,
  },
};
it("binds previews to source revisions and expiry", () => {
  expect(previewExpired(preview, 600999)).toBe(false);
  expect(previewExpired(preview, 601000)).toBe(true);
  expect(previewIsStale(preview, 1, false)).toBe(false);
  expect(previewIsStale(preview, 1, true)).toBe(true);
  expect(previewIsStale(preview, 2, false)).toBe(true);
  expect(
    previewIsStale({ ...preview, source: "published_snapshot" }, 1, true),
  ).toBe(false);
});
it("verifies exact bytes before sending them to PDF.js", async () => {
  expect(await verifiedPdfBytes(preview)).toEqual(
    new TextEncoder().encode("%PDF-"),
  );
  await expect(
    verifiedPdfBytes({ ...preview, pdfBase64: "JVBEUi0=" }),
  ).rejects.toThrow();
  await expect(
    verifiedPdfBytes({
      ...preview,
      receipt: { ...preview.receipt, byteCount: 6 },
    }),
  ).rejects.toThrow();
});
it("native requests contain no path, document, PDF bytes or template", async () => {
  vi.mocked(invoke).mockResolvedValueOnce({ ok: true, value: preview });
  expect((await renderResumePdf("saved_draft", 1)).ok).toBe(true);
  expect(invoke).toHaveBeenLastCalledWith("render_resume_pdf", {
    request: expect.objectContaining({
      payload: { source: "saved_draft", expectedRevision: 1 },
    }),
  });
  vi.mocked(invoke).mockResolvedValueOnce({
    ok: true,
    value: { status: "cancelled" },
  });
  expect((await exportResumePdf(preview)).ok).toBe(true);
  expect(invoke).toHaveBeenLastCalledWith("export_resume_pdf", {
    request: expect.objectContaining({
      payload: { renderId: preview.renderId },
    }),
  });
  vi.mocked(invoke).mockResolvedValueOnce({
    ok: true,
    value: { manifests: [] },
  });
  expect((await requestPdfRenderHistory()).ok).toBe(true);
  expect(invoke).toHaveBeenLastCalledWith("load_pdf_render_history", {
    request: expect.objectContaining({ payload: {} }),
  });
  vi.mocked(invoke).mockResolvedValueOnce({
    ok: true,
    value: { ...preview, revision: 2 },
  });
  expect((await renderResumePdf("saved_draft", 1)).ok).toBe(false);
  vi.mocked(invoke).mockResolvedValueOnce({
    ok: true,
    value: {
      status: "exported",
      renderId: preview.renderId,
      pdfSha256: "e".repeat(64),
      byteCount: 5,
      cleanupPending: false,
      durabilityUnconfirmed: false,
    },
  });
  expect((await exportResumePdf(preview)).ok).toBe(false);
});
it("rendering blocks quit and failures do not discard edits or pause autosave", () => {
  const state = {
    ...initialEditorState,
    status: "idle" as const,
    document: createResumeDocument(),
    editEpoch: 1,
  };
  const rendering = editorReducer(state, { type: "rendering" });
  expect(closeDisposition(rendering)).toBe("wait");
  const finished = editorReducer(rendering, {
    type: "export-finished",
    notice: pdfFailure("PDF_LAYOUT_LIMIT"),
  });
  expect(finished.document).toBe(state.document);
  expect(finished.autosavePaused).toBe(false);
  expect(closeDisposition(finished)).toBe("confirm");
});
it("offers explicit saved-source controls, privacy warning and local license notices", () => {
  const html = renderToStaticMarkup(
    <PdfPreviewPanel
      saved={null}
      published={null}
      dirty={false}
      blocked={false}
      onBegin={() => true}
      onFinish={() => {}}
      semantic={() => null}
    />,
  );
  expect(html).toContain("Preview saved draft");
  expect(html).toContain("Preview published snapshot");
  expect(html).toContain("Stored render history");
  expect(html).toContain("encrypted profile retains at most 100");
  expect(html).toContain("unencrypted");
  expect(html).toContain("SIL OPEN FONT LICENSE");
  expect(html).not.toContain("<iframe");
});
