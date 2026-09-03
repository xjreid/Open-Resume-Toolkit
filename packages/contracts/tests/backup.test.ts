import { describe, expect, it } from "vitest";
import {
  MAX_BACKUP_BYTES,
  MAX_BACKUP_PASSPHRASE_BYTES,
  DELETE_SAFETY_CONFIRMATION_PHRASE,
  RESTORE_CONFIRMATION_PHRASE,
  ROLLBACK_CONFIRMATION_PHRASE,
  isBackupRecoveryStatusCommandResponse,
  isDeleteSafetyCopyCommandResponse,
  isExportBackupCommandResponse,
  isRestoreBackupCommandResponse,
  isRollbackSafetyCopyCommandResponse,
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

  it("accepts only exact staged-replacement receipts", () => {
    expect(RESTORE_CONFIRMATION_PHRASE).toBe("REPLACE SAVED PROFILE");
    expect(
      isRestoreBackupCommandResponse({
        ok: true,
        value: {
          status: "staged",
          restartRequired: true,
          safetyCopyRetained: true,
        },
      }),
    ).toBe(true);
    expect(
      isRestoreBackupCommandResponse({
        ok: true,
        value: { status: "cancelled" },
      }),
    ).toBe(true);
    for (const value of [
      { status: "staged", restartRequired: false, safetyCopyRetained: true },
      { status: "staged", restartRequired: true, safetyCopyRetained: false },
      {
        status: "staged",
        restartRequired: true,
        safetyCopyRetained: true,
        path: "/private/profile",
      },
    ]) {
      expect(isRestoreBackupCommandResponse({ ok: true, value })).toBe(false);
    }
  });

  it("validates content-free safety-copy status and exact action receipts", () => {
    expect(ROLLBACK_CONFIRMATION_PHRASE).toBe("ROLL BACK SAVED PROFILE");
    expect(DELETE_SAFETY_CONFIRMATION_PHRASE).toBe("DELETE SAFETY COPY");
    expect(
      isBackupRecoveryStatusCommandResponse({
        ok: true,
        value: {
          safetyCopyAvailable: true,
          restartOperationPending: false,
          safetyCleanupPending: false,
        },
      }),
    ).toBe(true);
    expect(
      isBackupRecoveryStatusCommandResponse({
        ok: true,
        value: {
          safetyCopyAvailable: true,
          restartOperationPending: false,
          safetyCleanupPending: false,
          path: "/private/safety",
        },
      }),
    ).toBe(false);
    expect(
      isRollbackSafetyCopyCommandResponse({
        ok: true,
        value: { restartRequired: true, currentProfileRetained: true },
      }),
    ).toBe(true);
    expect(
      isRollbackSafetyCopyCommandResponse({
        ok: true,
        value: { restartRequired: false, currentProfileRetained: true },
      }),
    ).toBe(false);
    expect(
      isDeleteSafetyCopyCommandResponse({
        ok: true,
        value: { deleted: true },
      }),
    ).toBe(true);
    expect(
      isDeleteSafetyCopyCommandResponse({
        ok: true,
        value: { deleted: true, externalDeleted: true },
      }),
    ).toBe(false);
  });
});
