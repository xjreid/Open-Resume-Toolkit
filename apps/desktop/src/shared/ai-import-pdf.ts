import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { pdfBinaryData } from "./ai-import-pdf-assets";
import {
  pageHyperlinks,
  checkHyperlinkLimits,
  type PdfHyperlink,
} from "./ai-import-pdf-links";

export async function renderImportPages(
  base64: string,
  signal: AbortSignal,
  images = true,
): Promise<{ pages: string[]; links: PdfHyperlink[]; pageCount: number }> {
  if (base64.length > 14_000_000) throw new Error("AI_PDF_TOO_LARGE");
  const data = Uint8Array.from(atob(base64), (character) =>
    character.charCodeAt(0),
  );
  const engine = await import("pdfjs-dist");
  signal.throwIfAborted();
  const worker = new Worker(workerUrl, { type: "module" });
  let bridge: import("pdfjs-dist").PDFWorker | undefined;
  let task: import("pdfjs-dist").PDFDocumentLoadingTask | undefined;
  let assetFailed = false;
  let contentFailed = false;
  const inspectMessage = ({ data: message }: MessageEvent<unknown>) => {
    if (!message || typeof message !== "object") return;
    const { action, data } = message as { action?: unknown; data?: unknown };
    if (!Array.isArray(data)) return;
    // Pinned PDF.js 6.3 reports failed XObject decodes as null images, even
    // with stopAtErrors. Do not send pages with silently omitted content.
    const kind =
      action === "obj" ? data[2] : action === "commonobj" ? data[1] : null;
    const value = action === "obj" ? data[3] : data[2];
    if (
      (kind === "Image" && value === null) ||
      (kind === "Font" &&
        value &&
        typeof value === "object" &&
        "error" in value)
    )
      contentFailed = true;
  };
  worker.addEventListener("message", inspectMessage);
  class ImportBinaryDataFactory {
    async fetch({ kind, filename }: { kind: string; filename: string }) {
      try {
        return await pdfBinaryData(kind, filename);
      } catch {
        assetFailed = true;
        throw new Error("AI_PDF_RENDER_FAILED");
      }
    }
  }
  const dispose = () => {
    void task?.destroy().catch(() => {});
    bridge?.destroy();
    worker.removeEventListener("message", inspectMessage);
    worker.terminate();
  };
  signal.addEventListener("abort", dispose, { once: true });
  try {
    bridge = engine.PDFWorker.create({ port: worker });
    task = engine.getDocument({
      worker: bridge,
      data,
      BinaryDataFactory: ImportBinaryDataFactory,
      cMapPacked: true,
      disableFontFace: true,
      useSystemFonts: false,
      useWorkerFetch: false,
      useWasm: false,
      stopAtErrors: true,
      enableXfa: false,
      isOffscreenCanvasSupported: false,
      isImageDecoderSupported: false,
      // With stopAtErrors, PDF.js rejects oversized scans instead of omitting
      // them. This allows 600 dpi letter pages while bounding decoded pixels.
      maxImageSize: 40_000_000,
      canvasMaxAreaInBytes: 16_000_000,
    });
    const pdf = await task.promise;
    if (pdf.numPages < 1 || pdf.numPages > (images ? 10 : 1000))
      throw new Error("AI_PDF_PAGE_LIMIT");
    const pages: string[] = [];
    const links: PdfHyperlink[] = [];
    let total = 0;
    for (let index = 1; index <= pdf.numPages; index += 1) {
      signal.throwIfAborted();
      const page = await pdf.getPage(index);
      try {
        links.push(...(await pageHyperlinks(page, index, signal)));
        checkHyperlinkLimits(links);
      } catch (error) {
        page.cleanup();
        throw error;
      }
      if (!images) {
        page.cleanup();
        continue;
      }
      const original = page.getViewport({ scale: 1 });
      if (
        ![original.width, original.height].every(
          (size) => Number.isFinite(size) && size > 0,
        )
      )
        throw new Error("AI_PDF_RENDER_FAILED");
      const viewport = page.getViewport({
        scale: 1800 / Math.max(original.width, original.height),
      });
      const canvas = document.createElement("canvas");
      try {
        canvas.width = Math.min(1800, Math.ceil(viewport.width));
        canvas.height = Math.min(1800, Math.ceil(viewport.height));
        const context = canvas.getContext("2d");
        if (!context) throw new Error("AI_PDF_RENDER_FAILED");
        await page.render({
          canvas,
          canvasContext: context,
          viewport,
          background: "white",
        }).promise;
        signal.throwIfAborted();
        if (assetFailed || contentFailed)
          throw new Error("AI_PDF_RENDER_FAILED");
        const data = canvas.toDataURL("image/png");
        if (!data.startsWith("data:image/png;base64,"))
          throw new Error("AI_PDF_RENDER_FAILED");
        const encoded = data.slice("data:image/png;base64,".length);
        total += encoded.length;
        if (
          encoded.length > 4 * 1024 * 1024 ||
          total > Math.ceil((20 * 1024 * 1024 * 4) / 3)
        )
          throw new Error("AI_PDF_IMAGES_TOO_LARGE");
        pages.push(encoded);
      } finally {
        canvas.width = 0;
        canvas.height = 0;
        page.cleanup();
      }
    }
    return { pages, links, pageCount: pdf.numPages };
  } finally {
    signal.removeEventListener("abort", dispose);
    dispose();
  }
}

export async function listenForImportRendering(): Promise<() => void> {
  const controller = new AbortController();
  let rendering = false;
  let stopped = false;
  let timeout: number | undefined;
  const stop = await listen<unknown>("ort:ai-import-render-pdf", (event) => {
    const value = event.payload;
    if (
      controller.signal.aborted ||
      rendering ||
      typeof value !== "object" ||
      value === null
    )
      return;
    const payload = value as Record<string, unknown>;
    if (
      typeof payload.id !== "string" ||
      !/^[0-9a-f-]{36}$/i.test(payload.id) ||
      typeof payload.base64 !== "string" ||
      typeof payload.images !== "boolean"
    )
      return;
    rendering = true;
    const id = payload.id;
    timeout = window.setTimeout(() => controller.abort(), 25_000);
    void renderImportPages(payload.base64, controller.signal, payload.images)
      .then((prepared) => ({ id, ...prepared, error: null as string | null }))
      .catch((error: unknown) => ({
        id,
        pages: [] as string[],
        links: [] as PdfHyperlink[],
        pageCount: 0,
        error: controller.signal.aborted
          ? "AI_PDF_RENDER_TIMEOUT"
          : error instanceof Error &&
              [
                "AI_PDF_PAGE_LIMIT",
                "AI_PDF_IMAGES_TOO_LARGE",
                "AI_PDF_LINK_LIMIT",
              ].includes(error.message)
            ? error.message
            : "AI_PDF_RENDER_FAILED",
      }))
      .then(async (reply) => {
        window.clearTimeout(timeout);
        if (!stopped) await invoke("complete_ai_import_render", { reply });
      })
      .catch(() => {
        /* Native render deadline safely retires an unanswered request. */
      });
  });
  return () => {
    stopped = true;
    window.clearTimeout(timeout);
    controller.abort();
    stop();
  };
}
