import { expect, it, vi } from "vitest";
import {
  applyImportReview,
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
