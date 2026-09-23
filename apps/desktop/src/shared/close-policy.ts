import { isDirty, type EditorState } from "./editor-state";

export function closeDisposition(
  editor: EditorState,
  otherUnsavedWork = false,
): "wait" | "confirm" | "quit" {
  if (editor.status !== "idle") return "wait";
  if (otherUnsavedWork) return "confirm";
  // An untouched new template is not user work. Failed loads contain no edits.
  return isDirty(editor) && editor.editEpoch > 0 ? "confirm" : "quit";
}
