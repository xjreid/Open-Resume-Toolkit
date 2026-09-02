import { describe, expect, it } from "vitest";
import {
  editorReducer,
  initialEditorState,
  isDirty,
  requiresReload,
} from "./editor-state";
import { createResumeDocument } from "./resume-editor";

function loaded() {
  const document = createResumeDocument();
  return editorReducer(initialEditorState, {
    type: "loaded",
    empty: document,
    workspace: { draft: { revision: 2, document }, latestPublished: null },
  });
}

describe("editor persistence state", () => {
  it("keeps a bounded undo/redo history without resetting the saved revision", () => {
    let state = loaded();
    for (let index = 0; index < 35; index += 1) {
      state = editorReducer(state, {
        type: "edit",
        update: (d) => ({ ...d, title: `Edit ${index}` }),
      });
    }
    expect(state.undo).toHaveLength(30);
    state = editorReducer(state, { type: "undo" });
    expect(state.document?.title).toBe("Edit 33");
    expect(state.saved?.revision).toBe(2);
    state = editorReducer(state, { type: "redo" });
    expect(state.document?.title).toBe("Edit 34");
  });
  it("keeps edits made while a save response is in flight", () => {
    let state = loaded();
    state = editorReducer(state, {
      type: "edit",
      update: (d) => ({ ...d, title: "First edit" }),
    });
    const submitted = state.document!;
    const submittedEpoch = state.editEpoch;
    state = editorReducer(state, { type: "saving" });
    state = editorReducer(state, {
      type: "edit",
      update: (d) => ({ ...d, title: "Newer edit" }),
    });
    state = editorReducer(state, {
      type: "saved",
      value: { revision: 3, document: submitted },
      submittedEpoch,
    });
    expect(state.document?.title).toBe("Newer edit");
    expect(state.saved?.document.title).toBe("First edit");
    expect(state.saved?.revision).toBe(3);
    expect(isDirty(state)).toBe(true);
  });

  it("marks a matching save clean, but never an empty unsaved resume", () => {
    let state = loaded();
    state = editorReducer(state, {
      type: "saved",
      value: state.saved!,
      submittedEpoch: state.editEpoch,
    });
    expect(isDirty(state)).toBe(false);
    state = editorReducer(state, {
      type: "loaded",
      empty: createResumeDocument(),
      workspace: { draft: null, latestPublished: null },
    });
    expect(isDirty(state)).toBe(true);
  });

  it("keeps edits and pauses autosave on conflict or uncertain transport failure", () => {
    for (const code of [
      "REVISION_CONFLICT",
      "COMMAND_UNAVAILABLE",
      "INVALID_RESPONSE",
    ]) {
      const state = editorReducer(loaded(), { type: "failed", code });
      const edited = editorReducer(state, {
        type: "edit",
        update: (d) => ({ ...d, title: "Keep me" }),
      });
      expect(requiresReload(edited)).toBe(true);
      expect(edited.autosavePaused).toBe(true);
      expect(edited.document?.title).toBe("Keep me");
    }
  });

  it("publishing never replaces the editable document", () => {
    const state = loaded();
    const published = {
      revision: 1,
      document: { ...state.document!, title: "Earlier snapshot" },
    };
    const result = editorReducer(state, {
      type: "published",
      value: published,
    });
    expect(result.document).toBe(state.document);
    expect(result.published).toBe(published);
  });
});
