import type {
  ResumeDocument,
  ResumeWorkspace,
  VersionedResume,
} from "@ort/contracts/resume";
import { normalizeDocument } from "./resume-editor";

export interface EditorState {
  document: ResumeDocument | null;
  saved: VersionedResume | null;
  published: VersionedResume | null;
  status:
    | "loading"
    | "idle"
    | "saving"
    | "publishing"
    | "exporting"
    | "rendering"
    | "deleting";
  editEpoch: number;
  errorCode: string | null;
  autosavePaused: boolean;
  notice: string | null;
  undo: ResumeDocument[];
  redo: ResumeDocument[];
}

export const initialEditorState: EditorState = {
  document: null,
  saved: null,
  published: null,
  status: "loading",
  editEpoch: 0,
  errorCode: null,
  autosavePaused: false,
  notice: null,
  undo: [],
  redo: [],
};

type EditorAction =
  | { type: "loading" }
  | { type: "loaded"; workspace: ResumeWorkspace; empty: ResumeDocument }
  | { type: "edit"; update: (document: ResumeDocument) => ResumeDocument }
  | { type: "undo" }
  | { type: "redo" }
  | { type: "saving" }
  | { type: "saved"; value: VersionedResume; submittedEpoch: number }
  | { type: "publishing" }
  | { type: "published"; value: VersionedResume }
  | { type: "exporting" }
  | { type: "rendering" }
  | { type: "deleting" }
  | { type: "delete-finished" }
  | { type: "data-deleted" }
  | { type: "export-finished"; notice: string }
  | { type: "failed"; code: string };

export function isDirty(state: EditorState): boolean {
  return (
    state.document !== null &&
    (state.saved === null ||
      JSON.stringify(state.document) !== JSON.stringify(state.saved.document))
  );
}

// A transport error may follow a successful commit. Do not blindly repeat a
// create/publish or overwrite a conflicting revision: explicitly reload first.
export function requiresReload(state: EditorState): boolean {
  return [
    "REVISION_CONFLICT",
    "COMMAND_UNAVAILABLE",
    "INVALID_RESPONSE",
  ].includes(state.errorCode ?? "");
}

export function editorReducer(
  state: EditorState,
  action: EditorAction,
): EditorState {
  switch (action.type) {
    case "loading":
      return { ...state, status: "loading", errorCode: null, notice: null };
    case "loaded":
      return {
        ...initialEditorState,
        status: "idle",
        document: action.workspace.draft?.document ?? action.empty,
        saved: action.workspace.draft,
        published: action.workspace.latestPublished,
      };
    case "edit":
      return state.document
        ? {
            ...state,
            document: normalizeDocument(action.update(state.document)),
            editEpoch: state.editEpoch + 1,
            undo: [...state.undo.slice(-29), state.document],
            redo: [],
            notice: null,
          }
        : state;
    case "undo": {
      const previous = state.undo.at(-1);
      return previous && state.document
        ? {
            ...state,
            document: previous,
            undo: state.undo.slice(0, -1),
            redo: [...state.redo, state.document],
            editEpoch: state.editEpoch + 1,
            notice: null,
          }
        : state;
    }
    case "redo": {
      const next = state.redo.at(-1);
      return next && state.document
        ? {
            ...state,
            document: next,
            redo: state.redo.slice(0, -1),
            undo: [...state.undo.slice(-29), state.document],
            editEpoch: state.editEpoch + 1,
            notice: null,
          }
        : state;
    }
    case "saving":
      return { ...state, status: "saving", errorCode: null, notice: null };
    case "saved": {
      const newerEdits = state.editEpoch !== action.submittedEpoch;
      return {
        ...state,
        status: "idle",
        saved: action.value,
        document: newerEdits ? state.document : action.value.document,
        errorCode: null,
        autosavePaused: false,
        notice: newerEdits
          ? "Earlier changes saved; newer edits are still pending."
          : `Draft revision ${action.value.revision} saved securely.`,
      };
    }
    case "publishing":
      return { ...state, status: "publishing", errorCode: null, notice: null };
    case "published":
      return {
        ...state,
        status: "idle",
        published: action.value,
        notice: `Published immutable snapshot ${action.value.revision}.`,
      };
    case "exporting":
      return { ...state, status: "exporting", notice: null };
    case "rendering":
      return { ...state, status: "rendering", notice: null };
    case "deleting":
      return { ...state, status: "deleting", notice: null };
    case "delete-finished":
      return { ...state, status: "idle", notice: null };
    case "data-deleted":
      return { ...initialEditorState, status: "loading" };
    case "export-finished":
      // Export never mutates stored revisions or the editor. In particular,
      // an uncertain file result must not pause autosave or clear save errors.
      return { ...state, status: "idle", notice: action.notice };
    case "failed":
      return {
        ...state,
        status: "idle",
        errorCode: action.code,
        autosavePaused: true,
        notice: null,
      };
  }
}
