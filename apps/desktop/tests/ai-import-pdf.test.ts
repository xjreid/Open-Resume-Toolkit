// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  listenForImportRendering,
  renderImportPages,
} from "../src/shared/ai-import-pdf";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  stop: vi.fn(),
  getDocument: vi.fn(),
  destroy: vi.fn(),
  bridgeDestroy: vi.fn(),
  terminate: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("pdfjs-dist", () => ({
  PDFWorker: { create: () => ({ destroy: mocks.bridgeDestroy }) },
  getDocument: mocks.getDocument,
}));
const pdfData = btoa("%PDF-1.7\nsynthetic");
let event: (event: { payload: unknown }) => void;
let getPage: ReturnType<typeof vi.fn>;
let render: ReturnType<typeof vi.fn>;
let annotations: ReturnType<typeof vi.fn>;
let toDataURL: ReturnType<typeof vi.spyOn>;
let workerMessage: (event: MessageEvent<unknown>) => void;

beforeEach(() => {
  vi.clearAllMocks();
  mocks.invoke.mockResolvedValue({ ok: true, value: true });
  mocks.listen.mockImplementation(async (_name, callback) => {
    event = callback;
    return mocks.stop;
  });
  mocks.destroy.mockResolvedValue(undefined);
  vi.stubGlobal(
    "Worker",
    class {
      terminate = mocks.terminate;
      addEventListener(_name: string, callback: typeof workerMessage) {
        workerMessage = callback;
      }
      removeEventListener() {}
    },
  );
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
    {} as CanvasRenderingContext2D,
  );
  toDataURL = vi
    .spyOn(HTMLCanvasElement.prototype, "toDataURL")
    .mockReturnValue("data:image/png;base64,cGFnZQ==");
  render = vi.fn().mockReturnValue({ promise: Promise.resolve() });
  annotations = vi.fn().mockResolvedValue([]);
  getPage = vi.fn().mockImplementation(async () => ({
    getViewport: ({ scale }: { scale: number }) => ({
      width: 612 * scale,
      height: 792 * scale,
      convertToViewportPoint: (x: number, y: number) => [
        x * scale,
        (792 - y) * scale,
      ],
    }),
    render,
    cleanup: vi.fn(),
    getAnnotations: annotations,
    getTextContent: () => {
      throw new Error("Must not extract text locally");
    },
  }));
  mocks.getDocument.mockReturnValue({
    promise: Promise.resolve({ numPages: 2, getPage }),
    destroy: mocks.destroy,
  });
});

it("preserves hidden URI destinations and their positions alongside Codex page images", async () => {
  annotations.mockResolvedValueOnce([
    {
      subtype: "Link",
      url: "https://github.com/example/project?tab=readme#details",
      rect: [54, 624, 93, 638],
    },
    { subtype: "Link", url: "javascript:alert(1)", rect: [54, 624, 93, 638] },
    {
      subtype: "Link",
      unsafeUrl: "https://example.com",
      rect: [54, 624, 93, 638],
    },
    { subtype: "Widget", url: "https://example.com", rect: [54, 624, 93, 638] },
  ]);
  const result = await renderImportPages(pdfData, new AbortController().signal);
  expect(result.pages).toHaveLength(2);
  expect(result.links).toEqual([
    {
      page: 1,
      rect: [88, 194, 152, 212],
      url: "https://github.com/example/project?tab=readme#details",
    },
  ]);
  expect(annotations).toHaveBeenCalledWith({ intent: "display" });
});

it("also supplies annotations to API imports without rendering images or applying the Codex page limit", async () => {
  mocks.getDocument.mockReturnValueOnce({
    promise: Promise.resolve({ numPages: 11, getPage }),
    destroy: mocks.destroy,
  });
  annotations.mockResolvedValueOnce([
    {
      subtype: "Link",
      url: "https://linkedin.com/in/example",
      rect: [54, 711, 98, 725],
    },
  ]);
  const result = await renderImportPages(
    pdfData,
    new AbortController().signal,
    false,
  );
  expect(result.pages).toEqual([]);
  expect(result.pageCount).toBe(11);
  expect(result.links[0].url).toBe("https://linkedin.com/in/example");
  expect(render).not.toHaveBeenCalled();
  expect(toDataURL).not.toHaveBeenCalled();
});

it("fails the whole preparation if annotations cannot be read or exceed their bounded sidecar", async () => {
  annotations.mockRejectedValueOnce(new Error("annotation read failed"));
  await expect(
    renderImportPages(pdfData, new AbortController().signal),
  ).rejects.toThrow("annotation read failed");
  annotations.mockResolvedValueOnce(
    Array.from({ length: 201 }, () => ({
      subtype: "Link",
      url: "https://example.com",
      rect: [54, 624, 93, 638],
    })),
  );
  await expect(
    renderImportPages(pdfData, new AbortController().signal, false),
  ).rejects.toThrow("AI_PDF_LINK_LIMIT");
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

it("renders every page in order with bounded canvases and releases the worker without extracting text", async () => {
  toDataURL
    .mockReturnValueOnce("data:image/png;base64,cGFnZTE=")
    .mockReturnValueOnce("data:image/png;base64,cGFnZTI=");
  expect(
    await renderImportPages(pdfData, new AbortController().signal),
  ).toEqual({ pages: ["cGFnZTE=", "cGFnZTI="], links: [], pageCount: 2 });
  expect(getPage.mock.calls).toEqual([[1], [2]]);
  expect(render.mock.calls[0][0].viewport.height).toBeCloseTo(1800);
  expect(mocks.getDocument.mock.calls[0][0]).toMatchObject({
    disableFontFace: true,
    useSystemFonts: false,
    useWorkerFetch: false,
    enableXfa: false,
  });
  expect(mocks.destroy).toHaveBeenCalled();
  expect(mocks.terminate).toHaveBeenCalled();
});

it("rejects excess pages before rendering and never returns partial pages on failure", async () => {
  mocks.getDocument.mockReturnValueOnce({
    promise: Promise.resolve({ numPages: 11, getPage }),
    destroy: mocks.destroy,
  });
  await expect(
    renderImportPages(pdfData, new AbortController().signal),
  ).rejects.toThrow("AI_PDF_PAGE_LIMIT");
  expect(getPage).not.toHaveBeenCalled();
  render
    .mockReturnValueOnce({ promise: Promise.resolve() })
    .mockReturnValueOnce({ promise: Promise.reject(new Error("bad page")) });
  await expect(
    renderImportPages(pdfData, new AbortController().signal),
  ).rejects.toThrow("bad page");
  expect(mocks.terminate).toHaveBeenCalledTimes(2);
});

it("rejects oversized image data and cancels pending PDF loading", async () => {
  toDataURL.mockReturnValue(
    "data:image/png;base64," + "A".repeat(4 * 1024 * 1024 + 4),
  );
  await expect(
    renderImportPages(pdfData, new AbortController().signal),
  ).rejects.toThrow("AI_PDF_IMAGES_TOO_LARGE");
  let reject!: (error: Error) => void;
  const pending = new Promise((_resolve, fail) => {
    reject = fail;
  });
  mocks.destroy.mockImplementation(async () => {
    reject(new Error("cancelled"));
  });
  mocks.getDocument.mockReturnValue({
    promise: pending,
    destroy: mocks.destroy,
  });
  const controller = new AbortController();
  const result = renderImportPages(pdfData, controller.signal);
  const assertion = expect(result).rejects.toThrow("cancelled");
  await vi.waitFor(() => expect(mocks.getDocument).toHaveBeenCalledTimes(2));
  controller.abort();
  await assertion;
  expect(mocks.terminate).toHaveBeenCalled();
});

it("answers only one native render request and stops listening after the import", async () => {
  const stop = await listenForImportRendering();
  const payload = {
    id: "01900000-0000-7000-8000-000000000001",
    base64: pdfData,
    images: true,
  };
  event({ payload });
  event({ payload });
  await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledOnce());
  expect(mocks.invoke).toHaveBeenCalledWith("complete_ai_import_render", {
    reply: {
      id: payload.id,
      pages: ["cGFnZQ==", "cGFnZQ=="],
      links: [],
      pageCount: 2,
      error: null,
    },
  });
  stop();
  expect(mocks.stop).toHaveBeenCalledOnce();
});

it("does not submit a late result after cancellation and sanitizes renderer errors", async () => {
  let finish!: (value: unknown) => void;
  mocks.getDocument.mockReturnValueOnce({
    promise: new Promise((resolve) => {
      finish = resolve;
    }),
    destroy: mocks.destroy,
  });
  const stop = await listenForImportRendering();
  const payload = {
    id: "01900000-0000-7000-8000-000000000001",
    base64: pdfData,
    images: true,
  };
  event({ payload });
  await vi.waitFor(() => expect(mocks.getDocument).toHaveBeenCalled());
  stop();
  finish({ numPages: 2, getPage });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(mocks.invoke).not.toHaveBeenCalled();
  mocks.getDocument.mockImplementationOnce(() => {
    throw new Error("PRIVATE_DOCUMENT_CONTENT");
  });
  const stopAgain = await listenForImportRendering();
  event({ payload });
  await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledOnce());
  expect(mocks.invoke.mock.calls[0][1].reply).toEqual({
    id: payload.id,
    pages: [],
    links: [],
    pageCount: 0,
    error: "AI_PDF_RENDER_FAILED",
  });
  stopAgain();
});

it("reports a preparation timeout and destroys the stalled worker", async () => {
  vi.useFakeTimers();
  let reject!: (error: Error) => void;
  mocks.getDocument.mockReturnValue({
    promise: new Promise((_resolve, fail) => {
      reject = fail;
    }),
    destroy: async () => {
      reject(new Error("loading aborted"));
    },
  });
  const stop = await listenForImportRendering();
  const id = "01900000-0000-7000-8000-000000000001";
  event({ payload: { id, base64: pdfData, images: true } });
  await vi.advanceTimersByTimeAsync(25_000);
  expect(mocks.terminate).toHaveBeenCalled();
  expect(mocks.invoke).toHaveBeenCalledWith("complete_ai_import_render", {
    reply: {
      id,
      pages: [],
      links: [],
      pageCount: 0,
      error: "AI_PDF_RENDER_TIMEOUT",
    },
  });
  stop();
});

it("rejects a page when PDF.js suppresses a missing font error", async () => {
  render.mockImplementation(() => ({
    promise: (async () => {
      const Factory = mocks.getDocument.mock.calls[0][0].BinaryDataFactory;
      // PDF.js can catch font fetch errors internally and continue drawing.
      await new Factory()
        .fetch({ kind: "standardFontDataUrl", filename: "missing.ttf" })
        .catch(() => {});
    })(),
  }));
  await expect(
    renderImportPages(pdfData, new AbortController().signal),
  ).rejects.toThrow("AI_PDF_RENDER_FAILED");
  expect(toDataURL).not.toHaveBeenCalled();
});

it.each([
  { action: "obj", data: ["image", 0, "Image", null] },
  { action: "commonobj", data: ["image", "Image", null] },
  { action: "commonobj", data: ["font", "Font", { error: "broken font" }] },
])("rejects worker-reported content omissions: $action", async (data) => {
  render.mockImplementation(() => {
    workerMessage(new MessageEvent("message", { data }));
    return { promise: Promise.resolve() };
  });
  await expect(
    renderImportPages(pdfData, new AbortController().signal),
  ).rejects.toThrow("AI_PDF_RENDER_FAILED");
  expect(toDataURL).not.toHaveBeenCalled();
});
