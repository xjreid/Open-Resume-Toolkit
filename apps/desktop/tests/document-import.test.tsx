// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { DocumentImport } from "../src/shared/DocumentImport";
const client = vi.hoisted(() => ({
  documentImportAvailable: vi.fn(),
  beginDocumentImport: vi.fn(),
  cancelDocumentImport: vi.fn(),
}));
vi.mock("../src/shared/import-client", () => client);
vi.mock("../src/shared/ImportReviewFlow", () => ({
  ImportReviewFlow: ({ onCancelled }: { onCancelled: () => void }) => (
    <button onClick={onCancelled}>Cancel review</button>
  ),
}));
it("does not launch unavailable import or mutate an empty profile on picker cancellation", async () => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  client.documentImportAvailable.mockResolvedValue(false);
  client.beginDocumentImport.mockReset();
  const host = document.createElement("div");
  const root = createRoot(host);
  const saved = vi.fn();
  const busy = vi.fn();
  try {
    await act(async () =>
      root.render(
        <DocumentImport
          disabled={false}
          revision={null}
          onSaved={saved}
          onBusyChange={busy}
        />,
      ),
    );
    expect(host.querySelector("button")!.disabled).toBe(true);
    await act(async () => host.querySelector("button")!.click());
    expect(client.beginDocumentImport).not.toHaveBeenCalled();
  } finally {
    await act(async () => root.unmount());
  }
  const active = createRoot(host);
  client.documentImportAvailable.mockResolvedValue(true);
  client.beginDocumentImport.mockResolvedValue({ ok: true, value: null });
  try {
    await act(async () =>
      active.render(
        <DocumentImport
          disabled={false}
          revision={null}
          onSaved={saved}
          onBusyChange={busy}
        />,
      ),
    );
    await act(async () => host.querySelector("button")!.click());
    expect(client.beginDocumentImport).toHaveBeenCalledWith(null);
    expect(saved).not.toHaveBeenCalled();
    expect(busy).toHaveBeenLastCalledWith(false);
  } finally {
    await act(async () => active.unmount());
  }
});
it("owns one pending request and retains a review if cancellation races completion", async () => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  client.documentImportAvailable.mockResolvedValue(true);
  client.cancelDocumentImport.mockResolvedValue({ ok: true, value: true });
  let finish!: (value: unknown) => void;
  client.beginDocumentImport.mockReset().mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const host = document.createElement("div");
  const root = createRoot(host);
  const saved = vi.fn();
  const busy = vi.fn();
  try {
    await act(async () =>
      root.render(
        <DocumentImport
          disabled={false}
          revision={3}
          onSaved={saved}
          onBusyChange={busy}
        />,
      ),
    );
    await act(async () => {
      host.querySelector("button")!.click();
      host.querySelector("button")!.click();
    });
    expect(client.beginDocumentImport).toHaveBeenCalledOnce();
    await act(async () => {
      Array.from(host.querySelectorAll("button"))
        .find((button) => button.textContent === "Cancel import")!
        .click();
    });
    await act(async () => finish({ ok: true, value: { id: "native-review" } }));
    expect(host.textContent).toContain("Cancel review");
    expect(saved).not.toHaveBeenCalled();
    await act(async () => host.querySelector("button")!.click());
    expect(busy).toHaveBeenLastCalledWith(false);
  } finally {
    await act(async () => root.unmount());
  }
});
