// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { PdfPreview } from "@ort/contracts/pdf";
import { ApplicationCoverPreview } from "./ApplicationCoverPreview";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("./PdfPreview", () => ({
  PdfCanvas: ({
    preview,
    onReady,
    onError,
  }: {
    preview: { pdfBase64: string };
    onReady: () => void;
    onError: () => void;
  }) => (
    <div data-pdf={preview.pdfBase64}>
      <button onClick={onReady}>Ready</button>
      <button onClick={onError}>Render error</button>
    </div>
  ),
}));
const preview: PdfPreview = {
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
const cleanups: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const cleanup of cleanups.splice(0)) await cleanup();
  vi.resetAllMocks();
});
async function mount(revision?: number) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () =>
    root.render(
      <ApplicationCoverPreview
        revision={revision}
        text="Current cover letter"
      />,
    ),
  );
  cleanups.push(async () => {
    await act(async () => root.unmount());
    host.remove();
  });
  return { host, root };
}

it("loads the exact cover revision for the PDF canvas with readable text", async () => {
  vi.mocked(invoke).mockResolvedValue({
    ok: true,
    value: { ...preview, revision: 9, pdfBase64: "PDF-bytes" },
  });
  const { host } = await mount(9);
  expect(invoke).toHaveBeenCalledWith("preview_application_pdf", {
    expectedRevision: 9,
    kind: "cover_letter",
  });
  expect(host.querySelector('[data-pdf="PDF-bytes"]')).toBeTruthy();
  expect(host.querySelector(".visually-hidden")?.textContent).toContain(
    "Current cover letter",
  );
  expect(host.querySelector("textarea")).toBeNull();
  await act(async () =>
    host.querySelector<HTMLButtonElement>("button")!.click(),
  );
  expect(host.querySelector('[role="status"]')).toBeNull();
  expect(host.querySelector("details")).toBeNull();
});

it.each([
  { ok: false, error: { code: "EXPORT_NOT_PREPARED" } },
  { ok: true, value: { ...preview, revision: 8, pdfBase64: "Old PDF" } },
])("does not display a failed or mismatched saved revision", async (result) => {
  vi.mocked(invoke).mockResolvedValue(result);
  const { host } = await mount(9);
  expect(host.querySelector("[data-pdf]")).toBeNull();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain(
    "could not be loaded",
  );
});

it("requires a saved revision and hides a PDF if canvas verification fails", async () => {
  const { host, root } = await mount();
  expect(invoke).not.toHaveBeenCalled();
  vi.mocked(invoke).mockResolvedValue({
    ok: true,
    value: { ...preview, revision: 9, pdfBase64: "PDF" },
  });
  await act(async () =>
    root.render(<ApplicationCoverPreview revision={9} text="Current" />),
  );
  await act(async () =>
    host.querySelectorAll<HTMLButtonElement>("button")[1]!.click(),
  );
  expect(host.querySelector("[data-pdf]")).toBeNull();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain(
    "could not be displayed",
  );
});

it("ignores a late response from a previous saved revision", async () => {
  let finish!: (value: unknown) => void;
  vi.mocked(invoke).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const { host, root } = await mount(9);
  vi.mocked(invoke).mockResolvedValue({
    ok: true,
    value: { ...preview, revision: 10, pdfBase64: "New PDF" },
  });
  await act(async () =>
    root.render(<ApplicationCoverPreview revision={10} text="New text" />),
  );
  await act(async () =>
    finish({
      ok: true,
      value: { ...preview, revision: 9, pdfBase64: "Old PDF" },
    }),
  );
  expect(host.querySelector('[data-pdf="New PDF"]')).toBeTruthy();
  expect(host.querySelector('[data-pdf="Old PDF"]')).toBeNull();
});
