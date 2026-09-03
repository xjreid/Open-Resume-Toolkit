import { describe, expect, it } from "vitest";
import {
  isExportTextCommandResponse,
  isExportDocxCommandResponse,
} from "../generated/export";

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

describe("DOCX receipt boundary", () => {
  it("keeps format-specific limits and rejects unrecognized metadata", () => {
    const wrapped = (value: unknown) => ({ ok: true, value });
    expect(
      isExportDocxCommandResponse(wrapped({ ...receipt, byteCount: 2097152 })),
    ).toBe(true);
    expect(
      isExportTextCommandResponse(wrapped({ ...receipt, byteCount: 2097152 })),
    ).toBe(false);
    expect(isExportDocxCommandResponse(wrapped({ status: "cancelled" }))).toBe(
      true,
    );
    for (const value of [
      { ...receipt, byteCount: 2097153 },
      { ...receipt, byteCount: 0 },
      { ...receipt, byteCount: NaN },
      { ...receipt, formatVersion: 2 },
      { ...receipt, path: "/private" },
      { ...receipt, bytes: "not permitted" },
      { ...receipt, source: "unsaved" },
      { ...receipt, revision: 0 },
      { ...receipt, cleanupPending: "false" },
      { status: "cancelled", path: "/private" },
    ])
      expect(isExportDocxCommandResponse(wrapped(value))).toBe(false);
  });
});
