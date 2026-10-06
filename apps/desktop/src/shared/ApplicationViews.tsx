import { useEffect, useRef } from "react";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

export function PdfCanvas({ base64 }: { base64: string }) {
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let cancelled = false;
    let worker: Worker | null = null;
    let pdfWorker: import("pdfjs-dist").PDFWorker | null = null;
    let loading: import("pdfjs-dist").PDFDocumentLoadingTask | null = null;
    const host = container.current;
    if (!host) return;
    host.replaceChildren();
    void (async () => {
      try {
        const engine = await import("pdfjs-dist");
        if (cancelled) return;
        worker = new Worker(workerUrl, { type: "module" });
        pdfWorker = engine.PDFWorker.create({ port: worker });
        const binary = atob(base64);
        const bytes = Uint8Array.from(binary, (character) =>
          character.charCodeAt(0),
        );
        loading = engine.getDocument({
          worker: pdfWorker,
          data: bytes,
          disableFontFace: true,
          useSystemFonts: false,
          useWorkerFetch: false,
          useWasm: false,
          stopAtErrors: true,
          enableXfa: false,
          isOffscreenCanvasSupported: false,
          isImageDecoderSupported: false,
          maxImageSize: 2_000_000,
          canvasMaxAreaInBytes: 8_000_000,
        });
        const pdf = await loading.promise;
        for (let index = 1; index <= pdf.numPages && !cancelled; index += 1) {
          const page = await pdf.getPage(index);
          const viewport = page.getViewport({ scale: 1.2 });
          const canvas = document.createElement("canvas");
          canvas.width = Math.ceil(viewport.width);
          canvas.height = Math.ceil(viewport.height);
          canvas.setAttribute("aria-label", `PDF page ${index}`);
          host.append(canvas);
          const context = canvas.getContext("2d");
          if (context)
            await page.render({ canvas, canvasContext: context, viewport })
              .promise;
        }
      } catch {
        if (!cancelled)
          host.textContent = "Preview unavailable. Download remains available.";
      }
    })();
    return () => {
      cancelled = true;
      void loading?.destroy();
      pdfWorker?.destroy();
      worker?.terminate();
      host.replaceChildren();
    };
  }, [base64]);
  return (
    <div
      className="application-pdf-pages"
      ref={container}
      aria-label="PDF preview"
    />
  );
}
