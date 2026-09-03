import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { requestStorageUsage } from "./command-client";
import { formatBytes, StoragePanel } from "./StoragePanel";

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
      createElement(StoragePanel, { enabled: false }),
    );
    expect(html).toContain("Encrypted profile storage");
    expect(html).toContain("External exports and backups");
    expect(html).toContain("OS-vault");
    expect(html).not.toContain("profileId");
    expect(html).not.toContain("path");
  });
});
