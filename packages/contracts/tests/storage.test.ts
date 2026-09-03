import { describe, expect, it } from "vitest";
import {
  DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE,
  isDeleteAllLocalDataCommandResponse,
  isStorageUsageCommandResponse,
} from "../generated/storage";

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

  it("accepts only exact all-local-data deletion outcomes", () => {
    expect(DELETE_ALL_LOCAL_DATA_CONFIRMATION_PHRASE).toBe(
      "DELETE ALL LOCAL ORT DATA",
    );
    expect(
      isDeleteAllLocalDataCommandResponse({
        ok: true,
        value: { status: "deleted", freshProfileReady: true },
      }),
    ).toBe(true);
    expect(
      isDeleteAllLocalDataCommandResponse({
        ok: true,
        value: { status: "cleanup_pending", restartRequired: true },
      }),
    ).toBe(true);
    for (const value of [
      { status: "deleted", freshProfileReady: true, path: "/private/profile" },
      { status: "deleted", freshProfileReady: "yes" },
      { status: "cleanup_pending", restartRequired: false },
      { status: "cleanup_pending", restartRequired: true, deleted: false },
    ]) {
      expect(isDeleteAllLocalDataCommandResponse({ ok: true, value })).toBe(
        false,
      );
    }
  });
});
