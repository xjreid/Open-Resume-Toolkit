// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { ImportReviewFlow } from "../src/shared/ImportReviewFlow";
const client = vi.hoisted(() => ({
  readImportReview: vi.fn(),
  applyImportReview: vi.fn(),
  cancelImportReview: vi.fn(),
}));
vi.mock("../src/shared/import-client", () => client);
it("blocks resubmission after an uncertain save while keeping cancellation available", async () => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  client.readImportReview.mockResolvedValue({
    ok: true,
    value: {
      id: "synthetic",
      baseRevision: 1,
      mappingVersion: 1,
      blocks: [
        {
          source: "Projects",
          page: 1,
          explanation: "Heading",
          suggestedTarget: "section",
          suggestedValue: "Projects",
          proposedSection: null,
        },
      ],
      sections: [],
      contacts: { fullName: "", email: "", phone: "", location: "" },
    },
  });
  client.applyImportReview.mockResolvedValue({
    ok: false,
    error: { code: "OUTCOME_UNKNOWN" },
  });
  client.cancelImportReview.mockResolvedValue({ ok: true, value: true });
  const saved = vi.fn();
  const cancelled = vi.fn();
  try {
    await act(async () =>
      root.render(
        <ImportReviewFlow
          reviewId="synthetic"
          currentRevision={1}
          onSaved={saved}
          onCancelled={cancelled}
        />,
      ),
    );
    const decision = host.querySelector("select")!;
    await act(async () => {
      decision.value = "accept";
      decision.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () =>
      (
        host.querySelector('button[type="submit"]') as HTMLButtonElement
      ).click(),
    );
    expect(client.applyImportReview).toHaveBeenCalledOnce();
    expect(saved).not.toHaveBeenCalled();
    expect(
      (host.querySelector('button[type="submit"]') as HTMLButtonElement)
        .disabled,
    ).toBe(true);
    await act(async () =>
      host
        .querySelector("form")!
        .dispatchEvent(
          new Event("submit", { bubbles: true, cancelable: true }),
        ),
    );
    expect(client.applyImportReview).toHaveBeenCalledOnce();
    await act(async () =>
      (
        host.querySelector('button[type="button"]') as HTMLButtonElement
      ).click(),
    );
    expect(cancelled).toHaveBeenCalledOnce();
  } finally {
    await act(async () => root.unmount());
    host.remove();
    vi.unstubAllGlobals();
  }
});
