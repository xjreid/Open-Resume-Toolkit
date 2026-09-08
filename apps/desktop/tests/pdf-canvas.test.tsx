import { createHash } from "node:crypto";
import { JSDOM } from "jsdom";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { PdfCanvas } from "../src/shared/PdfPreview";
import type { PdfPreview } from "@ort/contracts/pdf";

const engine = vi.hoisted(() => ({ getDocument: vi.fn(), create: vi.fn() }));
vi.mock("pdfjs-dist", () => ({
  getDocument: engine.getDocument,
  PDFWorker: { create: engine.create },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
let dom: JSDOM, root: Root;
let terminate: ReturnType<typeof vi.fn>, destroy: ReturnType<typeof vi.fn>;
let getPage: ReturnType<typeof vi.fn>, renderPage: ReturnType<typeof vi.fn>;
let ready: ReturnType<typeof vi.fn>,
  error: ReturnType<typeof vi.fn>,
  pending: ReturnType<typeof vi.fn>;
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
    templateId: "technical_pdf_v1",
    templateSha256: "c".repeat(64),
    fontBundleId: "libertinus-serif/typst-assets-0.15.1",
    fontBundleSha256: "d".repeat(64),
    pageCount: 3,
    byteCount: 5,
  },
};
async function settle() {
  await act(async () => {
    for (let i = 0; i < 8; i++)
      await new Promise((resolve) => setTimeout(resolve, 0));
  });
}
async function mount() {
  await act(async () =>
    root.render(
      <PdfCanvas
        preview={preview}
        onReady={ready}
        onError={error}
        onPending={pending}
      />,
    ),
  );
  await settle();
}
beforeEach(() => {
  dom = new JSDOM('<div id="root"></div>', { url: "http://localhost" });
  vi.stubGlobal("window", dom.window);
  vi.stubGlobal("document", dom.window.document);
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  terminate = vi.fn();
  destroy = vi.fn(async () => {});
  vi.stubGlobal(
    "Worker",
    class {
      terminate = terminate;
    },
  );
  engine.create.mockReturnValue({ destroy: vi.fn() });
  renderPage = vi.fn(() => ({ promise: Promise.resolve(), cancel: vi.fn() }));
  getPage = vi.fn(async () => ({
    getViewport: ({ scale }: { scale: number }) => ({
      width: 612 * scale,
      height: 792 * scale,
    }),
    render: renderPage,
  }));
  engine.getDocument.mockReturnValue({
    promise: Promise.resolve({ numPages: 3, getPage }),
    destroy,
  });
  ready = vi.fn();
  error = vi.fn();
  pending = vi.fn();
  root = createRoot(document.getElementById("root")!);
});
afterEach(async () => {
  await act(async () => root.unmount());
  dom.window.close();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
it("renders all pages sequentially and zooms without loading a second PDF", async () => {
  await mount();
  expect(getPage.mock.calls.map(([page]) => page)).toEqual([1, 2, 3]);
  expect(document.querySelectorAll("canvas")).toHaveLength(3);
  expect(ready).toHaveBeenCalledTimes(1);
  expect(error).not.toHaveBeenCalled();
  const select = document.querySelector("select")!;
  await act(async () => {
    select.value = "2";
    select.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  await settle();
  expect(engine.getDocument).toHaveBeenCalledTimes(1);
  expect(ready).toHaveBeenCalledTimes(2);
  expect(pending).toHaveBeenCalled();
  for (const canvas of document.querySelectorAll("canvas"))
    expect(canvas.width).toBe(1224);
  const canvases = [...document.querySelectorAll("canvas")];
  await act(async () => root.unmount());
  expect(destroy).toHaveBeenCalled();
  expect(terminate).toHaveBeenCalled();
  expect(
    canvases.every((canvas) => canvas.width === 0 && canvas.height === 0),
  ).toBe(true);
});
it("does not mark a partial document ready and clears pages after a later-page failure", async () => {
  renderPage
    .mockImplementationOnce(() => ({
      promise: Promise.resolve(),
      cancel: vi.fn(),
    }))
    .mockImplementationOnce(() => ({
      promise: Promise.reject(new Error("synthetic")),
      cancel: vi.fn(),
    }));
  await mount();
  expect(ready).not.toHaveBeenCalled();
  expect(error).toHaveBeenCalledTimes(1);
  expect(getPage).toHaveBeenCalledTimes(2);
  expect(
    [...document.querySelectorAll("canvas")].every(
      (canvas) => canvas.width === 0,
    ),
  ).toBe(true);
});
it("rejects oversized canvas geometry before allocating or rendering", async () => {
  getPage.mockResolvedValue({
    getViewport: () => ({ width: 2000, height: 2000 }),
    render: renderPage,
  });
  await mount();
  expect(renderPage).not.toHaveBeenCalled();
  expect(ready).not.toHaveBeenCalled();
  expect(error).toHaveBeenCalledTimes(1);
});
it("cancels pending page rendering on unmount and ignores its late completion", async () => {
  let resolve: (() => void) | undefined;
  const cancel = vi.fn();
  renderPage.mockReturnValue({
    promise: new Promise<void>((done) => {
      resolve = done;
    }),
    cancel,
  });
  await mount();
  expect(ready).not.toHaveBeenCalled();
  await act(async () => root.unmount());
  expect(cancel).toHaveBeenCalled();
  await act(async () => resolve?.());
  expect(ready).not.toHaveBeenCalled();
  expect(error).not.toHaveBeenCalled();
  expect(getPage).toHaveBeenCalledTimes(1);
});

it("fits resized panes without reloading bytes and disconnects its observer", async () => {
  let resize:
    | ((entries: Array<{ contentRect: { width: number } }>) => void)
    | undefined;
  const disconnect = vi.fn();
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(callback: typeof resize) {
        resize = callback;
      }
      observe() {
        resize?.([{ contentRect: { width: 306 } }]);
      }
      disconnect = disconnect;
    },
  );
  await mount();
  for (const canvas of document.querySelectorAll("canvas"))
    expect(canvas.width).toBe(306);
  await act(async () => resize?.([{ contentRect: { width: 459 } }]));
  await settle();
  for (const canvas of document.querySelectorAll("canvas"))
    expect(canvas.width).toBe(459);
  expect(engine.getDocument).toHaveBeenCalledTimes(1);
  expect(error).not.toHaveBeenCalled();
  await act(async () => root.unmount());
  expect(disconnect).toHaveBeenCalledTimes(1);
});

it("does not let completion from a cancelled zoom restore readiness", async () => {
  let finishOld: (() => void) | undefined;
  const cancel = vi.fn();
  renderPage.mockImplementationOnce(() => ({
    promise: new Promise<void>((resolve) => {
      finishOld = resolve;
    }),
    cancel,
  }));
  await mount();
  const select = document.querySelector("select")!;
  await act(async () => {
    select.value = "1.5";
    select.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  await settle();
  expect(cancel).toHaveBeenCalledTimes(1);
  expect(ready).toHaveBeenCalledTimes(1);
  await act(async () => finishOld?.());
  await settle();
  expect(ready).toHaveBeenCalledTimes(1);
  expect(error).not.toHaveBeenCalled();
  for (const canvas of document.querySelectorAll("canvas"))
    expect(canvas.width).toBe(918);
});
