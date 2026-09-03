import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { deleteAllLocalData, requestStorageUsage } from "./command-client";
import {
  deleteAllLocalDataFeedback,
  formatBytes,
  StoragePanel,
} from "./StoragePanel";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
beforeEach(() => vi.clearAllMocks());

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

describe("storage usage", () => {
  it("uses an empty path-free request and rejects malformed responses", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ ok: true, value: usage });
    expect((await requestStorageUsage()).ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("load_storage_usage", {
      request: {
        contractVersion: 2,
        requestId: expect.any(String),
        payload: {},
      },
    });
    vi.mocked(invoke).mockResolvedValueOnce({
      ok: true,
      value: { ...usage, totalProfileBytes: 999 },
    });
    expect((await requestStorageUsage()).ok).toBe(false);
  });

  it("formats exact bytes with readable binary units", () => {
    expect(formatBytes(0)).toBe("0 bytes");
    expect(formatBytes(1)).toBe("1 byte");
    expect(formatBytes(1_024)).toBe("1.00 KiB (1024 bytes)");
    expect(formatBytes(10 * 1_024)).toBe("10.0 KiB (10240 bytes)");
    expect(formatBytes(2 * 1_024 * 1_024)).toBe("2.00 MiB (2097152 bytes)");
  });

  it("renders content-free scope and exclusions before data is available", () => {
    const html = renderToStaticMarkup(
      createElement(StoragePanel, {
        enabled: false,
        onDeleteBegin: () => true,
        onDeleteFinish: () => {},
      }),
    );
    expect(html).toContain("Encrypted profile storage");
    expect(html).toContain("External exports and backups");
    expect(html).toContain("OS-vault");
    expect(html).not.toContain("profileId");
    expect(html).not.toContain("path");
    expect(html).toContain("DELETE ALL LOCAL ORT DATA");
    expect(html).toContain("unsaved edits");
    expect(html).toContain("are not deleted");
    expect(html).toContain('class="button--danger"');
  });

  it("sends only the exact confirmation and validates deletion outcomes", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      ok: true,
      value: { status: "deleted", freshProfileReady: true },
    });
    const result = await deleteAllLocalData("DELETE ALL LOCAL ORT DATA");
    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("delete_all_local_data", {
      request: {
        contractVersion: 2,
        requestId: expect.any(String),
        payload: { confirmation: "DELETE ALL LOCAL ORT DATA" },
      },
    });
    const serialized = JSON.stringify(vi.mocked(invoke).mock.calls[0]?.[1]);
    expect(serialized).not.toContain("path");
    expect(serialized).not.toContain("profileId");
    expect(deleteAllLocalDataFeedback(result)).toContain(
      "new empty encrypted profile",
    );
  });

  it("distinguishes committed cleanup from an unstarted or unknown result", () => {
    expect(
      deleteAllLocalDataFeedback({
        ok: true,
        value: { status: "cleanup_pending", restartRequired: true },
      }),
    ).toContain("Deletion was committed");
    expect(
      deleteAllLocalDataFeedback({
        ok: false,
        error: {
          code: "LOCAL_DATA_DELETE_UNSAFE",
          messageKey: "errors.localDataDelete",
          retryable: false,
          details: {},
        },
      }),
    ).toContain("Deletion was not started");
    expect(
      deleteAllLocalDataFeedback({
        ok: false,
        error: {
          code: "LOCAL_DATA_DELETE_OUTCOME_UNKNOWN",
          messageKey: "errors.localDataDelete",
          retryable: true,
          details: {},
        },
      }),
    ).toContain("outcome is unknown");
  });
});
