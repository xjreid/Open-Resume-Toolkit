import { describe, expect, it } from "vitest";
import { isStorageUsageCommandResponse } from "../generated/storage";

const usage = {
  databaseSchema: 2,
  drafts: 1,
  publishedSnapshots: 2,
  settings: 1,
  renderManifests: 3,
  diagnosticEvents: 4,
  databaseBytes: 100,
  walBytes: 20,
  sharedMemoryBytes: 10,
  manifestBytes: 5,
  recoveryMetadataBytes: 0,
  totalProfileBytes: 135,
};

describe("storage usage contract", () => {
  it("accepts a content-free exact inventory", () => {
    expect(isStorageUsageCommandResponse({ ok: true, value: usage })).toBe(
      true,
    );
  });

  it("rejects content, invalid counts, unsafe bytes and inconsistent totals", () => {
    for (const value of [
      { ...usage, title: "private content" },
      { ...usage, drafts: 2 },
      { ...usage, settings: -1 },
      { ...usage, databaseBytes: Number.MAX_SAFE_INTEGER + 1 },
      { ...usage, totalProfileBytes: 136 },
    ]) {
      expect(isStorageUsageCommandResponse({ ok: true, value })).toBe(false);
    }
  });
});
