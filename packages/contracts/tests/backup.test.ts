import { describe, expect, it } from "vitest";
import {
  MAX_BACKUP_BYTES,
  MAX_BACKUP_PASSPHRASE_BYTES,
  isExportBackupCommandResponse,
  isValidateBackupCommandResponse,
} from "../generated/backup";

describe("portable backup contract", () => {
  it("accepts cancellation and bounded format-1.1 receipts", () => {
    expect(
      isExportBackupCommandResponse({
        ok: true,
        value: { status: "cancelled" },
      }),
    ).toBe(true);
    expect(
      isExportBackupCommandResponse({
        ok: true,
        value: {
          status: "exported",
          byteCount: MAX_BACKUP_BYTES,
          formatMajor: 1,
          formatMinor: 1,
          cleanupPending: false,
          durabilityUnconfirmed: false,
        },
      }),
    ).toBe(true);
    expect(MAX_BACKUP_PASSPHRASE_BYTES).toBe(1024);
  });

  it("rejects malformed, oversized and version-drifted receipts", () => {
    const base = {
      status: "exported",
      byteCount: 100,
      formatMajor: 1,
      formatMinor: 1,
      cleanupPending: false,
      durabilityUnconfirmed: false,
    };
    for (const value of [
      { ...base, byteCount: MAX_BACKUP_BYTES + 1 },
      { ...base, byteCount: 0 },
      { ...base, formatMinor: 0 },
      { ...base, passphrase: "must not return" },
    ]) {
      expect(isExportBackupCommandResponse({ ok: true, value })).toBe(false);
    }
  });

  it("accepts only exact authenticated-inventory validation responses", () => {
    const current = {
      status: "validated",
      byteCount: 500,
      formatMajor: 1,
      formatMinor: 1,
      appVersion: "0.0.0-dev",
      databaseSchema: 2,
      documentSchema: 1,
      createdAt: "2026-09-03T12:00:00Z",
      masterDrafts: 1,
      publishedResumes: 2,
      settings: 3,
      renderManifests: 4,
    };
    expect(isValidateBackupCommandResponse({ ok: true, value: current })).toBe(
      true,
    );
    expect(
      isValidateBackupCommandResponse({
        ok: true,
        value: {
          ...current,
          formatMinor: 0,
          databaseSchema: 1,
          renderManifests: 0,
        },
      }),
    ).toBe(true);
    expect(
      isValidateBackupCommandResponse({
        ok: true,
        value: { status: "cancelled" },
      }),
    ).toBe(true);

    for (const value of [
      { ...current, path: "/private/backup" },
      { ...current, byteCount: MAX_BACKUP_BYTES + 1 },
      { ...current, appVersion: "<script>" },
      { ...current, formatMinor: 0, databaseSchema: 2 },
      { ...current, masterDrafts: 2 },
      { ...current, publishedResumes: 101 },
      { ...current, formatMinor: 0, databaseSchema: 1 },
    ]) {
      expect(isValidateBackupCommandResponse({ ok: true, value })).toBe(false);
    }
  });
});
