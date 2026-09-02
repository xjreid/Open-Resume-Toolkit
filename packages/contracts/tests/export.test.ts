import { describe, expect, it } from "vitest";
import { isExportTextCommandResponse } from "../generated/export";

const receipt = {
  status: "exported",
  source: "saved_draft",
  revision: 2,
  byteCount: 123,
  formatVersion: 1,
  cleanupPending: false,
  durabilityUnconfirmed: false,
};
describe("text-export response boundary", () => {
  it("accepts cancellation and bounded receipts, including committed-file warnings", () => {
    for (const value of [
      { status: "cancelled" },
      receipt,
      {
        ...receipt,
        source: "published_snapshot",
        cleanupPending: true,
        durabilityUnconfirmed: true,
      },
    ])
      expect(isExportTextCommandResponse({ ok: true, value })).toBe(true);
  });
  it("rejects path leakage, wrong versions, malformed results and unbounded numbers", () => {
    for (const value of [
      { status: "cancelled", path: "sensitive" },
      { ...receipt, path: "sensitive" },
      { ...receipt, source: "editor" },
      { ...receipt, revision: 0 },
      { ...receipt, revision: Number.MAX_SAFE_INTEGER + 1 },
      { ...receipt, byteCount: 262145 },
      { ...receipt, byteCount: 1.1 },
      { ...receipt, byteCount: 0 },
      { ...receipt, formatVersion: 2 },
      { ...receipt, cleanupPending: "false" },
    ])
      expect(isExportTextCommandResponse({ ok: true, value })).toBe(false);
  });
});
