import { describe, expect, it, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { exportResumeText, exportResumeDocument } from "./command-client";
import { exportFeedback } from "./text-export";
import {
  editorReducer,
  initialEditorState,
  isDirty,
  requiresReload,
} from "./editor-state";
import { closeDisposition } from "./close-policy";
import { createResumeDocument } from "./resume-editor";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const receipt = {
  status: "exported" as const,
  source: "saved_draft" as const,
  revision: 2,
  byteCount: 123,
  formatVersion: 1 as const,
  cleanupPending: false,
  durabilityUnconfirmed: false,
};
beforeEach(() => vi.clearAllMocks());

describe("text export command", () => {
  it("dispatches DOCX through its fixed command without passing format, content or path", async () => {
    vi.mocked(invoke).mockResolvedValue({
      ok: true,
      value: { ...receipt, byteCount: 300000 },
    });
    const result = await exportResumeDocument("saved_draft", 2, "docx");
    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("export_resume_docx", {
      request: {
        contractVersion: 2,
        requestId: expect.any(String),
        payload: { source: "saved_draft", expectedRevision: 2 },
      },
    });
    expect(exportFeedback(result, "docx")).toContain("DOCX (plain layout v1)");
    expect((await exportResumeDocument("saved_draft", 3, "docx")).ok).toBe(
      false,
    );
    expect(
      (await exportResumeDocument("published_snapshot", 2, "docx")).ok,
    ).toBe(false);
    expect((await exportResumeText("saved_draft", 2)).ok).toBe(false);
  });
  it("does not retry failed DOCX operations or expose native details", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("private OS detail"));
    const result = await exportResumeDocument("published_snapshot", 2, "docx");
    expect(result.ok).toBe(false);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(exportFeedback(result, "docx")).not.toContain("private OS detail");
  });
  it("sends only the exact source and revision and validates the returned identity", async () => {
    vi.mocked(invoke).mockResolvedValue({ ok: true, value: receipt });
    expect((await exportResumeText("saved_draft", 2)).ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("export_resume_text", {
      request: {
        contractVersion: 2,
        requestId: expect.any(String),
        payload: { source: "saved_draft", expectedRevision: 2 },
      },
    });
    expect((await exportResumeText("published_snapshot", 2)).ok).toBe(false);
    expect((await exportResumeText("saved_draft", 3)).ok).toBe(false);
  });
  it("never retries uncertain outcomes and reports cancellation accurately", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("private OS detail"));
    const result = await exportResumeText("saved_draft", 2);
    expect(result.ok).toBe(false);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(exportFeedback(result)).toContain("Check your chosen folder");
    expect(exportFeedback(result)).not.toContain("private OS detail");
    vi.mocked(invoke).mockResolvedValueOnce({
      ok: true,
      value: { status: "cancelled" },
    });
    expect(exportFeedback(await exportResumeText("saved_draft", 2))).toContain(
      "No file was written",
    );
  });
  it("does not hide cleanup/durability warnings after a committed export", () => {
    const message = exportFeedback({
      ok: true,
      value: { ...receipt, cleanupPending: true, durabilityUnconfirmed: true },
    });
    expect(message).toContain("Exported saved draft revision 2");
    expect(message).toContain("unencrypted");
    expect(message).toContain("staging folder remains");
    expect(message).toContain("power loss");
  });
});

describe("export and editor lifecycle isolation", () => {
  it("waits during export and preserves edits, revisions and autosave after any export outcome", () => {
    const document = createResumeDocument();
    const loaded = editorReducer(initialEditorState, {
      type: "loaded",
      empty: document,
      workspace: { draft: { revision: 2, document }, latestPublished: null },
    });
    for (const notice of [
      "Exported",
      "Canceled",
      "COMMAND_UNAVAILABLE",
      "REVISION_CONFLICT",
    ]) {
      let state = editorReducer(loaded, { type: "exporting" });
      expect(closeDisposition(state)).toBe("wait");
      state = editorReducer(state, {
        type: "edit",
        update: (d) => ({ ...d, title: "Newer edits" }),
      });
      state = editorReducer(state, { type: "export-finished", notice });
      expect(state.saved).toBe(loaded.saved);
      expect(state.document?.title).toBe("Newer edits");
      expect(isDirty(state)).toBe(true);
      expect(requiresReload(state)).toBe(false);
      expect(state.autosavePaused).toBe(false);
      expect(closeDisposition(state)).toBe("confirm");
    }
  });
  it("does not clear a pre-existing save failure", () => {
    const failed = editorReducer(initialEditorState, {
      type: "failed",
      code: "REVISION_CONFLICT",
    });
    const state = editorReducer(failed, {
      type: "export-finished",
      notice: "Export canceled",
    });
    expect(requiresReload(state)).toBe(true);
    expect(state.autosavePaused).toBe(true);
  });
});
