import { beforeEach, describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { invoke } from "@tauri-apps/api/core";
import { BackupPanel } from "./BackupPanel";
import {
  exportPortableBackup,
  deleteSafetyCopy,
  requestBackupRecoveryStatus,
  restorePortableBackup,
  rollbackSafetyCopy,
  validatePortableBackup,
} from "./command-client";
import {
  backupFeedback,
  backupRestoreFeedback,
  backupValidationFeedback,
  deleteSafetyFeedback,
  rollbackSafetyFeedback,
} from "./backup-export";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
beforeEach(() => vi.clearAllMocks());

const receipt = {
  status: "exported" as const,
  byteCount: 500,
  formatMajor: 1 as const,
  formatMinor: 1 as const,
  cleanupPending: false,
  durabilityUnconfirmed: false,
};

describe("portable backup export", () => {
  it("renders an accessible encrypted-backup form with truthful recovery limits", () => {
    const html = renderToStaticMarkup(
      createElement(BackupPanel, {
        blocked: false,
        dirty: false,
        onBegin: () => true,
        onFinish: () => {},
      }),
    );
    expect(html.match(/type="password"/g)).toHaveLength(4);
    expect(html).toContain("Encrypted portable backup");
    expect(html).toContain("passphrase cannot be recovered");
    expect(html).toContain("credentials are excluded");
    expect(html).toContain("activated only after restart");
    expect(html).toContain("Check an existing backup");
    expect(html).toContain("does not replace or write to the active profile");
    expect(html).toContain("selected path is never returned");
    expect(html).toContain("REPLACE SAVED PROFILE");
    expect(html).toContain("ROLL BACK SAVED PROFILE");
    expect(html).toContain("DELETE SAFETY COPY");
    expect(html).toContain("external exports and backups");
    expect(html).toContain('type="text"');
  });

  it("sends only the bounded passphrase and validates the fixed format receipt", async () => {
    vi.mocked(invoke).mockResolvedValue({ ok: true, value: receipt });
    const result = await exportPortableBackup("synthetic backup phrase");
    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("export_portable_backup", {
      request: {
        contractVersion: 2,
        requestId: expect.any(String),
        payload: { passphrase: "synthetic backup phrase" },
      },
    });
    const request = vi.mocked(invoke).mock.calls[0]?.[1] as unknown as Record<
      string,
      unknown
    >;
    expect(JSON.stringify(request)).not.toContain("path");
    expect(JSON.stringify(request)).not.toContain("overwrite");
    expect(backupFeedback(result)).toContain("format 1.1");
    expect(backupFeedback(result)).toContain("unrecoverable passphrase");
  });

  it("does not retry or expose native failure details", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("private path detail"));
    const result = await exportPortableBackup("synthetic backup phrase");
    expect(result.ok).toBe(false);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(backupFeedback(result)).not.toContain("private path detail");
  });

  it("reports cancellation and publication warnings precisely", () => {
    expect(
      backupFeedback({ ok: true, value: { status: "cancelled" } }),
    ).toContain("No file was written");
    const message = backupFeedback({
      ok: true,
      value: { ...receipt, cleanupPending: true, durabilityUnconfirmed: true },
    });
    expect(message).toContain("encrypted backup bytes");
    expect(message).toContain("power loss");
  });

  it("validates a selected backup with only a passphrase request", async () => {
    const validated = {
      status: "validated" as const,
      byteCount: 500,
      formatMajor: 1 as const,
      formatMinor: 1 as const,
      appVersion: "0.0.0-dev",
      databaseSchema: 2 as const,
      documentSchema: 1 as const,
      createdAt: "2026-09-03T12:00:00Z",
      masterDrafts: 1,
      publishedResumes: 2,
      settings: 3,
      renderManifests: 4,
    };
    vi.mocked(invoke).mockResolvedValue({ ok: true, value: validated });
    const result = await validatePortableBackup("synthetic backup phrase");
    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("validate_portable_backup", {
      request: {
        contractVersion: 2,
        requestId: expect.any(String),
        payload: { passphrase: "synthetic backup phrase" },
      },
    });
    expect(JSON.stringify(vi.mocked(invoke).mock.calls[0]?.[1])).not.toContain(
      "path",
    );
    expect(backupValidationFeedback(result)).toContain(
      "active profile was not changed",
    );
  });

  it("keeps invalid files and wrong passphrases deliberately indistinguishable", () => {
    const message = backupValidationFeedback({
      ok: false,
      error: {
        code: "BACKUP_INVALID_OR_PASSPHRASE",
        messageKey: "errors.backupValidation",
        retryable: false,
        details: {},
      },
    });
    expect(message).toContain("passphrase may be incorrect");
    expect(message).toContain("file may be damaged or unsupported");
    expect(message).not.toContain("path");
  });

  it("sends only passphrase and exact confirmation for staged replacement", async () => {
    vi.mocked(invoke).mockResolvedValue({
      ok: true,
      value: {
        status: "staged",
        restartRequired: true,
        safetyCopyRetained: true,
      },
    });
    const result = await restorePortableBackup(
      "synthetic restore phrase",
      "REPLACE SAVED PROFILE",
    );
    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("restore_portable_backup", {
      request: {
        contractVersion: 2,
        requestId: expect.any(String),
        payload: {
          passphrase: "synthetic restore phrase",
          confirmation: "REPLACE SAVED PROFILE",
        },
      },
    });
    expect(JSON.stringify(vi.mocked(invoke).mock.calls[0]?.[1])).not.toContain(
      "path",
    );
    expect(backupRestoreFeedback(result)).toContain("Restart ORT");
    expect(backupRestoreFeedback(result)).toContain("safety copy");
  });

  it("uses content-free status and exact confirmations for safety management", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        ok: true,
        value: {
          safetyCopyAvailable: true,
          restartOperationPending: false,
          safetyCleanupPending: false,
        },
      })
      .mockResolvedValueOnce({
        ok: true,
        value: { restartRequired: true, currentProfileRetained: true },
      })
      .mockResolvedValueOnce({ ok: true, value: { deleted: true } });

    expect((await requestBackupRecoveryStatus()).ok).toBe(true);
    expect(await rollbackSafetyCopy("ROLL BACK SAVED PROFILE")).toEqual({
      ok: true,
      value: { restartRequired: true, currentProfileRetained: true },
    });
    const deleted = await deleteSafetyCopy("DELETE SAFETY COPY");
    expect(deleted.ok).toBe(true);
    expect(vi.mocked(invoke).mock.calls).toEqual([
      [
        "load_backup_recovery_status",
        {
          request: {
            contractVersion: 2,
            requestId: expect.any(String),
            payload: {},
          },
        },
      ],
      [
        "rollback_safety_copy",
        {
          request: {
            contractVersion: 2,
            requestId: expect.any(String),
            payload: { confirmation: "ROLL BACK SAVED PROFILE" },
          },
        },
      ],
      [
        "delete_safety_copy",
        {
          request: {
            contractVersion: 2,
            requestId: expect.any(String),
            payload: { confirmation: "DELETE SAFETY COPY" },
          },
        },
      ],
    ]);
    expect(
      rollbackSafetyFeedback({
        ok: true,
        value: { restartRequired: true, currentProfileRetained: true },
      }),
    ).toContain("current profile");
    expect(deleteSafetyFeedback(deleted)).toContain(
      "external exports or backups",
    );
  });
});
