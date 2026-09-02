import { describe, expect, it } from "vitest";
import { closeDisposition } from "../src/shared/close-policy";
import { editorReducer, initialEditorState } from "../src/shared/editor-state";
import { createResumeDocument } from "../src/shared/resume-editor";

function loaded() {
  const document = createResumeDocument();
  return editorReducer(initialEditorState, {
    type: "loaded",
    empty: document,
    workspace: { draft: { revision: 1, document }, latestPublished: null },
  });
}

describe("native quit policy", () => {
  it("allows saved and untouched new documents, but waits for startup", () => {
    expect(closeDisposition(initialEditorState)).toBe("wait");
    expect(closeDisposition(loaded())).toBe("quit");
    const state = editorReducer(initialEditorState, {
      type: "loaded",
      empty: createResumeDocument(),
      workspace: { draft: null, latestPublished: null },
    });
    expect(closeDisposition(state)).toBe("quit");
  });

  it("requires confirmation for valid or invalid unsaved content", () => {
    for (const title of ["Edited", " "]) {
      const state = editorReducer(loaded(), {
        type: "edit",
        update: (document) => ({ ...document, title }),
      });
      expect(closeDisposition(state)).toBe("confirm");
    }
  });

  it("waits for a save and does not quit on failure or a late older save", () => {
    let state = editorReducer(loaded(), {
      type: "edit",
      update: (document) => ({ ...document, title: "Save me" }),
    });
    const document = state.document!;
    const epoch = state.editEpoch;
    state = editorReducer(state, { type: "saving" });
    expect(closeDisposition(state)).toBe("wait");
    for (const code of [
      "STORAGE_UNAVAILABLE",
      "REVISION_CONFLICT",
      "COMMAND_UNAVAILABLE",
    ]) {
      expect(
        closeDisposition(editorReducer(state, { type: "failed", code })),
      ).toBe("confirm");
    }
    const saved = editorReducer(state, {
      type: "saved",
      value: { revision: 2, document },
      submittedEpoch: epoch,
    });
    expect(closeDisposition(saved)).toBe("quit");
    state = editorReducer(state, {
      type: "edit",
      update: (value) => ({ ...value, title: "Newer edit" }),
    });
    state = editorReducer(state, {
      type: "saved",
      value: { revision: 2, document },
      submittedEpoch: epoch,
    });
    expect(closeDisposition(state)).toBe("confirm");
  });

  it("waits for publication even when the draft is already saved", () => {
    expect(
      closeDisposition(editorReducer(loaded(), { type: "publishing" })),
    ).toBe("wait");
  });
});
