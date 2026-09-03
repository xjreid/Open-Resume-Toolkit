import { useEffect, useRef, useState, type ReactNode } from "react";
import type {
  PDFDocumentProxy,
  PDFDocumentLoadingTask,
  PDFWorker,
  RenderTask,
} from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import {
  PDF_PREVIEW_TTL_SECONDS,
  type PdfPreview,
  type PdfRenderManifest,
} from "@ort/contracts/pdf";
import type { ExportSource } from "@ort/contracts/export";
import type { VersionedResume, ResumeDocument } from "@ort/contracts/resume";
import {
  renderResumePdf,
  exportResumePdf,
  releaseResumePdf,
  replayPdfRender,
  requestPdfRenderHistory,
} from "./command-client";
import {
  pdfFailure,
  previewExpired,
  previewIsStale,
  verifiedPdfBytes,
} from "./pdf-preview";
import { PdfNotices } from "./PdfNotices";

interface Props {
  saved: VersionedResume | null;
  published: VersionedResume | null;
  dirty: boolean;
  blocked: boolean;
  onBegin: (kind: "rendering" | "exporting") => boolean;
  onFinish: (notice: string) => void;
  semantic: (document: ResumeDocument) => ReactNode;
}

export function replayableDocument(
  manifest: PdfRenderManifest,
  saved: VersionedResume | null,
  published: VersionedResume | null,
): ResumeDocument | null {
  const current = manifest.source === "saved_draft" ? saved : published;
  return current?.revision === manifest.sourceRevision
    ? current.document
    : null;
}

export function PdfPreviewPanel({
  saved,
  published,
  dirty,
  blocked,
  onBegin,
  onFinish,
  semantic,
}: Props) {
  const [snapshot, setSnapshot] = useState<{
    preview: PdfPreview;
    document: ResumeDocument;
  } | null>(null);
  const [ready, setReady] = useState(false);
  const [history, setHistory] = useState<PdfRenderManifest[] | null>(null);
  const [historyError, setHistoryError] = useState(false);
  const [message, setMessage] = useState(
    "Generate a preview from a saved revision. Unsaved edits are not included.",
  );
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    void refreshHistory();
    return () => {
      mounted.current = false;
    };
  }, []);

  async function refreshHistory() {
    const result = await requestPdfRenderHistory();
    if (!mounted.current) return;
    if (result.ok) {
      setHistory(result.value.manifests);
      setHistoryError(false);
    } else {
      setHistoryError(true);
    }
  }
  useEffect(() => {
    if (!snapshot) return;
    const { preview } = snapshot;
    const remaining = Math.max(
      0,
      Math.min(
        PDF_PREVIEW_TTL_SECONDS * 1000,
        preview.generatedAtUnixMs + PDF_PREVIEW_TTL_SECONDS * 1000 - Date.now(),
      ),
    );
    const timer = window.setTimeout(() => {
      setSnapshot(null);
      setReady(false);
      setMessage("Preview expired. Generate a fresh preview to export.");
    }, remaining);
    return () => {
      window.clearTimeout(timer);
      void releaseResumePdf(preview.renderId);
    };
  }, [snapshot]);

  async function generate(source: ExportSource) {
    const selected = source === "saved_draft" ? saved : published;
    if (
      blocked ||
      !selected ||
      (source === "saved_draft" && dirty) ||
      !onBegin("rendering")
    )
      return;
    setSnapshot(null);
    setReady(false);
    setMessage("Rendering the saved revision locally…");
    const result = await renderResumePdf(source, selected.revision);
    if (!mounted.current) {
      if (result.ok) void releaseResumePdf(result.value.renderId);
      return;
    }
    if (result.ok) {
      setSnapshot({ preview: result.value, document: selected.document });
      setMessage("Loading the generated PDF…");
      onFinish("PDF generated locally. Review the preview before exporting.");
      void refreshHistory();
    } else {
      const message = pdfFailure(result.error.code);
      setMessage(message);
      onFinish(message);
    }
  }

  async function download() {
    if (blocked || !snapshot || !ready || stale || !onBegin("exporting"))
      return;
    if (previewExpired(snapshot.preview, Date.now())) {
      onFinish(pdfFailure("PDF_PREVIEW_EXPIRED"));
      setSnapshot(null);
      return;
    }
    const result = await exportResumePdf(snapshot.preview);
    const notice = !result.ok
      ? pdfFailure(result.error.code)
      : result.value.status === "cancelled"
        ? "PDF export cancelled. No file was created."
        : `The exact preview PDF was exported unencrypted.${result.value.cleanupPending ? " A private staging copy could not be removed; inspect the destination folder." : ""}${result.value.durabilityUnconfirmed ? " The filesystem could not confirm crash durability." : ""}`;
    if (mounted.current) {
      setMessage(notice);
      onFinish(notice);
    }
  }

  function replayDocument(manifest: PdfRenderManifest) {
    return replayableDocument(manifest, saved, published);
  }

  async function replay(manifest: PdfRenderManifest) {
    const document = replayDocument(manifest);
    if (
      blocked ||
      (manifest.source === "saved_draft" && dirty) ||
      !document ||
      !onBegin("rendering")
    )
      return;
    setSnapshot(null);
    setReady(false);
    setMessage("Verifying the retained receipt with the installed renderer…");
    const result = await replayPdfRender(manifest);
    if (!mounted.current) {
      if (result.ok) void releaseResumePdf(result.value.renderId);
      return;
    }
    if (result.ok) {
      setSnapshot({ preview: result.value, document });
      setMessage("Loading the verified replay PDF…");
      onFinish("The installed renderer reproduced the exact retained receipt.");
      void refreshHistory();
      return;
    }
    const notice =
      result.error.code === "PDF_REPLAY_SOURCE_UNAVAILABLE"
        ? "That exact source revision is no longer the current draft or published snapshot. No substitute was rendered."
        : result.error.code === "PDF_REPLAY_INCOMPATIBLE"
          ? "The installed renderer could not reproduce the exact historical receipt. No replay was exposed."
          : pdfFailure(result.error.code);
    setMessage(notice);
    onFinish(notice);
  }

  const preview = snapshot?.preview;
  const current = preview?.source === "saved_draft" ? saved : published;
  const stale = preview
    ? previewIsStale(preview, current?.revision ?? null, dirty)
    : false;
  return (
    <section className="editor-panel pdf-panel" aria-labelledby="pdf-title">
      <h2 id="pdf-title">PDF preview &amp; export</h2>
      <p>
        Plain layout v1 · US Letter · 11 pt Libertinus Serif · English layout ·
        five pages maximum. Unsupported characters are reported, not
        substituted. Export creates an unencrypted file; choose a private local
        folder and a new filename.
      </p>
      <div className="move-controls">
        <button
          type="button"
          disabled={blocked || !saved || dirty}
          onClick={() => void generate("saved_draft")}
        >
          Preview saved draft
        </button>
        <button
          type="button"
          disabled={blocked || !published}
          onClick={() => void generate("published_snapshot")}
        >
          Preview published snapshot
        </button>
        {snapshot ? (
          <button
            type="button"
            className="button--secondary"
            disabled={blocked}
            onClick={() => {
              setSnapshot(null);
              setReady(false);
              setMessage("Preview cleared.");
            }}
          >
            Clear preview
          </button>
        ) : null}
      </div>
      <p role="status">{message}</p>
      {snapshot && preview ? (
        <>
          <p>
            {preview.source === "saved_draft"
              ? "Saved draft"
              : "Published snapshot"}{" "}
            revision {preview.revision} · {preview.receipt.pageCount} page(s).
            This preview expires after ten minutes.
          </p>
          {stale ? (
            <p role="status">
              This preview is out of date. Save pending edits and generate a
              fresh preview before exporting.
            </p>
          ) : null}
          <PdfCanvas
            key={preview.renderId}
            preview={preview}
            onReady={() => {
              setReady(true);
              setMessage("Preview ready. Export uses these exact PDF bytes.");
            }}
            onError={() => {
              setReady(false);
              setMessage(
                "The PDF could not be displayed. Export is disabled; the accessible content is available below.",
              );
            }}
          />
          <button
            type="button"
            disabled={blocked || stale || !ready}
            onClick={() => void download()}
          >
            Export this preview (.pdf)
          </button>
          <details>
            <summary>Accessible text for this preview</summary>
            {semantic(snapshot.document)}
          </details>
          <details>
            <summary>Render receipt</summary>
            <dl className="pdf-receipt">
              <dt>Renderer</dt>
              <dd>{preview.receipt.rendererVersion}</dd>
              <dt>Template</dt>
              <dd>
                {preview.receipt.templateId} · {preview.receipt.templateSha256}
              </dd>
              <dt>Fonts</dt>
              <dd>
                {preview.receipt.fontBundleId} ·{" "}
                {preview.receipt.fontBundleSha256}
              </dd>
              <dt>PDF SHA-256</dt>
              <dd>{preview.receipt.pdfSha256}</dd>
              <dt>
                Document SHA-256 (schema {preview.receipt.documentSchemaVersion}
                )
              </dt>
              <dd>{preview.receipt.documentSha256}</dd>
              <dt>Generated</dt>
              <dd>
                {new Date(preview.generatedAtUnixMs).toISOString()} ·{" "}
                {preview.receipt.byteCount} bytes
              </dd>
            </dl>
            <p>
              This content-free receipt is stored in the encrypted profile.
              Historical renderer replay is not implemented.
            </p>
          </details>
        </>
      ) : null}
      <details>
        <summary>
          Stored render history{history ? ` (${history.length})` : ""}
        </summary>
        {historyError ? (
          <p role="status">Stored render history is temporarily unavailable.</p>
        ) : history === null ? (
          <p role="status">Loading encrypted render history…</p>
        ) : history.length === 0 ? (
          <p>No PDF render receipts are stored yet.</p>
        ) : (
          <ol className="pdf-history">
            {history.map((manifest) => (
              <li key={manifest.manifestId}>
                <strong>
                  {manifest.source === "saved_draft"
                    ? "Saved draft"
                    : "Published snapshot"}{" "}
                  revision {manifest.sourceRevision}
                </strong>
                <span>
                  {new Date(manifest.lastGeneratedAtUnixMs).toISOString()} ·{" "}
                  {manifest.receipt.pageCount} page(s) · rendered{" "}
                  {manifest.renderCount} time(s)
                </span>
                <code>{manifest.receipt.pdfSha256}</code>
                <button
                  type="button"
                  className="button--secondary button--compact"
                  disabled={
                    blocked ||
                    (manifest.source === "saved_draft" && dirty) ||
                    replayDocument(manifest) === null
                  }
                  title={
                    replayDocument(manifest) === null
                      ? "Replay requires this exact revision to be the current draft or latest published snapshot."
                      : "Regenerate and expose a preview only if every retained receipt field matches."
                  }
                  onClick={() => void replay(manifest)}
                >
                  Verify &amp; replay
                </button>
              </li>
            ))}
          </ol>
        )}
        <p>
          The encrypted profile retains at most 100 distinct receipt identities;
          this view shows the newest 20. PDF bytes and resume text are not
          stored in this history. Replay never substitutes a newer source or a
          different renderer result; unavailable revisions remain
          inspection-only.
        </p>
      </details>
      <PdfNotices />
    </section>
  );
}

function PdfCanvas({
  preview,
  onReady,
  onError,
}: {
  preview: PdfPreview;
  onReady: () => void;
  onError: () => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [pdf, setPdf] = useState<PDFDocumentProxy | null>(null);
  const [page, setPage] = useState(1);
  const [zoom, setZoom] = useState(1);
  const callbacks = useRef({ onReady, onError });
  callbacks.current = { onReady, onError };
  useEffect(() => {
    let disposed = false;
    let task: PDFDocumentLoadingTask | undefined;
    let worker: Worker | undefined;
    let bridge: PDFWorker | undefined;
    const dispose = () => {
      void task?.destroy().catch(() => {});
      bridge?.destroy();
      worker?.terminate();
    };
    const timer = window.setTimeout(() => {
      if (!disposed) {
        disposed = true;
        callbacks.current.onError();
        dispose();
      }
    }, 10_000);
    void (async () => {
      const data = await verifiedPdfBytes(preview);
      const engine = await import("pdfjs-dist");
      if (disposed) return;
      // Explicit local worker: no PDF.js CDN/blob wrapper or fake-worker fallback,
      // including on Tauri custom-scheme origins. CSP permits only local workers.
      worker = new Worker(workerUrl, { type: "module" });
      bridge = engine.PDFWorker.create({ port: worker });
      task = engine.getDocument({
        worker: bridge,
        data,
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
      const document = await task.promise;
      if (document.numPages !== preview.receipt.pageCount)
        throw new Error("PDF page count mismatch");
      if (!disposed) setPdf(document);
    })()
      .catch(() => {
        if (!disposed) {
          callbacks.current.onError();
          dispose();
        }
      })
      .finally(() => window.clearTimeout(timer));
    return () => {
      disposed = true;
      window.clearTimeout(timer);
      dispose();
    };
  }, [preview]);
  useEffect(() => {
    if (!pdf || !canvas.current) return;
    let disposed = false;
    let task: RenderTask | undefined;
    const element = canvas.current;
    const timer = window.setTimeout(() => {
      if (!disposed) {
        disposed = true;
        task?.cancel();
        callbacks.current.onError();
      }
    }, 10_000);
    void (async () => {
      const current = await pdf.getPage(page);
      if (disposed) return;
      const viewport = current.getViewport({ scale: zoom });
      if (viewport.width * viewport.height > 2_000_000)
        throw new Error("PDF canvas limit");
      element.width = Math.ceil(viewport.width);
      element.height = Math.ceil(viewport.height);
      task = current.render({ canvas: element, viewport });
      await task.promise;
      if (!disposed) callbacks.current.onReady();
    })()
      .catch(() => {
        if (!disposed) callbacks.current.onError();
      })
      .finally(() => window.clearTimeout(timer));
    return () => {
      disposed = true;
      window.clearTimeout(timer);
      task?.cancel();
      element.width = 0;
      element.height = 0;
    };
  }, [pdf, page, zoom]);
  return (
    <div>
      <div className="move-controls" aria-label="PDF navigation">
        <button
          type="button"
          disabled={!pdf || page <= 1}
          onClick={() => setPage(page - 1)}
        >
          Previous page
        </button>
        <span aria-live="polite">
          Page {page} of {preview.receipt.pageCount}
        </span>
        <button
          type="button"
          disabled={!pdf || page >= preview.receipt.pageCount}
          onClick={() => setPage(page + 1)}
        >
          Next page
        </button>
        <label>
          PDF zoom{" "}
          <select
            value={zoom}
            onChange={(e) => setZoom(Number(e.target.value))}
          >
            <option value={1}>100%</option>
            <option value={1.5}>150%</option>
            <option value={2}>200%</option>
          </select>
        </label>
      </div>
      <div
        className="pdf-canvas-scroll"
        tabIndex={0}
        role="region"
        aria-label="Scrollable PDF page"
      >
        <canvas
          ref={canvas}
          role="img"
          aria-label={`PDF page ${page}. Equivalent accessible text follows the preview.`}
        />
      </div>
    </div>
  );
}
