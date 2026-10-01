import {
  createResumeDocument,
  upgradeDocumentV2,
} from "../src/shared/resume-editor";
import { expect, it, vi } from "vitest";
import {
  applyImportReview,
  mapImportReview,
  cancelImportReview,
  readImportReview,
} from "../src/shared/import-client";
const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
it("rejects malformed native replies and never retries a potentially committed apply", async () => {
  native.invoke.mockReset();
  native.invoke.mockResolvedValue({ ok: true, value: { owner: "forged" } });
  expect((await readImportReview("synthetic-id")).ok).toBe(false);
  native.invoke.mockRejectedValueOnce(new Error("synthetic native failure"));
  const start = native.invoke.mock.calls.length;
  expect(
    (await applyImportReview("synthetic-id", { choices: [{ kind: "reject" }] }))
      .ok,
  ).toBe(false);
  expect(native.invoke.mock.calls.length - start).toBe(1);
  const [command, { request }] = native.invoke.mock.calls.at(-1)!;
  expect(command).toBe("apply_import_review");
  expect(request.payload).toEqual({
    reviewId: "synthetic-id",
    decisionsJson: '{"choices":[{"kind":"reject"}]}',
  });
  native.invoke.mockResolvedValueOnce({ ok: true, value: false });
  expect((await cancelImportReview("synthetic-id")).ok).toBe(false);
  native.invoke.mockResolvedValueOnce({ ok: true, value: true });
  expect((await cancelImportReview("synthetic-id")).ok).toBe(true);
});

it("sends only the edited document and review identity through the replacement command without retry", async () => {
  native.invoke.mockReset().mockRejectedValue(new Error("native failure"));
  const document = upgradeDocumentV2(createResumeDocument());
  expect((await mapImportReview("review-id", document)).ok).toBe(false);
  expect(native.invoke).toHaveBeenCalledOnce();
  const [command, { request }] = native.invoke.mock.calls[0];
  expect(command).toBe("map_import_review");
  expect(request.payload).toEqual({
    reviewId: "review-id",
    documentJson: JSON.stringify(document),
  });
});
