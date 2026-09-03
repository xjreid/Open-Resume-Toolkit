import { beforeEach, describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { invoke } from "@tauri-apps/api/core";
import { BackupPanel } from "./BackupPanel";
import { exportPortableBackup, validatePortableBackup } from "./command-client";
import { backupFeedback, backupValidationFeedback } from "./backup-export";

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
    expect(html.match(/type="password"/g)).toHaveLength(3);
    expect(html).toContain("Encrypted portable backup");
    expect(html).toContain("passphrase cannot be recovered");
    expect(html).toContain("credentials are excluded");
    expect(html).toContain(
      "Restore into a clean replacement profile is not enabled",
    );
    expect(html).toContain("Check an existing backup");
    expect(html).toContain("does not replace or write to the active profile");
    expect(html).toContain("selected path is never returned");
    expect(html).not.toContain('type="text"');
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
});
