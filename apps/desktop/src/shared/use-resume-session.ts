import {
  useCallback,
  useEffect,
  useReducer,
  useRef,
  useState,
  useMemo,
} from "react";
import type { DocumentStyle, ExportSource } from "@ort/contracts/export";
import type { HealthState } from "./HealthBadge";
import {
  editorReducer,
  initialEditorState,
  isDirty,
  requiresReload,
} from "./editor-state";
import { validateEditorDocument } from "./resume-validation";
import { useCloseGuard } from "./use-close-guard";
import {
  publishResume,
  requestHealth,
  requestResumeWorkspace,
  saveResume,
  exportResumeDocument,
  exportResumePdf,
  releaseResumePdf,
  renderResumePdf,
} from "./command-client";
import {
  createResumeDocument,
  normalizeDocument,
  upgradeDocumentV2,
} from "./resume-editor";
import { exportFeedback } from "./text-export";
import { pdfFailure } from "./pdf-preview";

type SessionOptions = {
  importActive: boolean;
  importWorking: boolean;
  trackerDirty: boolean;
  confirmReload: boolean;
  documentStyle: DocumentStyle;
  exportSource: ExportSource;
};

/** Owns resume persistence and its revision/epoch-checked editor lifecycle. */
export function useResumeSession({
  importActive,
  importWorking,
  trackerDirty,
  confirmReload,
  documentStyle,
  exportSource,
}: SessionOptions) {
  const [health, setHealth] = useState<HealthState>({ kind: "checking" });
  const [editor, dispatch] = useReducer(editorReducer, initialEditorState);
  const close = useCloseGuard(
    importWorking ? { ...editor, status: "exporting" } : editor,
    trackerDirty || importActive,
  );
  const { document } = editor;
  const revision = editor.saved?.revision ?? null;
  const dirty = isDirty(editor);
  const busy = editor.status !== "idle" || importActive;
  const mustReload = requiresReload(editor);
  const issues = useMemo(
    () => (document ? validateEditorDocument(document) : []),
    [document],
  );
  const ioBusy = useRef(false);
  const loadGeneration = useRef(0);
  const loadWorkspace = useCallback(async () => {
    const generation = ++loadGeneration.current;
    dispatch({ type: "loading" });
    setHealth({ kind: "checking" });
    const healthResult = await requestHealth();
    if (generation !== loadGeneration.current) return;
    if (!healthResult.ok) {
      setHealth({ kind: "error", message: healthResult.error.messageKey });
      dispatch({ type: "failed", code: healthResult.error.code });
      return;
    }
    setHealth({ kind: "ready", health: healthResult.value });
    if (healthResult.value.storageStatus !== "ready") {
      dispatch({ type: "failed", code: "STORAGE_UNAVAILABLE" });
      return;
    }

    const workspace = await requestResumeWorkspace();
    if (generation !== loadGeneration.current) return;
    if (!workspace.ok) {
      dispatch({ type: "failed", code: workspace.error.code });
      return;
    }
    dispatch({
      type: "loaded",
      workspace: workspace.value,
      empty: upgradeDocumentV2(createResumeDocument()),
    });
  }, []);

  useEffect(() => {
    void loadWorkspace();
    return () => {
      loadGeneration.current += 1;
    };
  }, [loadWorkspace]);
  const save = useCallback(async () => {
    if (
      !document ||
      busy ||
      ioBusy.current ||
      !dirty ||
      issues.length ||
      mustReload
    )
      return;
    ioBusy.current = true;
    const submittedEpoch = editor.editEpoch;
    dispatch({ type: "saving" });
    const normalized = normalizeDocument(document);
    const result = await saveResume(revision, normalized);
    if (result.ok) {
      dispatch({ type: "saved", value: result.value, submittedEpoch });
    } else {
      dispatch({ type: "failed", code: result.error.code });
    }
    ioBusy.current = false;
  }, [
    document,
    busy,
    dirty,
    issues.length,
    mustReload,
    editor.editEpoch,
    revision,
  ]);

  useEffect(() => {
    if (
      !dirty ||
      busy ||
      !editor.editEpoch ||
      editor.autosavePaused ||
      issues.length ||
      confirmReload ||
      close.pending
    )
      return;
    const timer = window.setTimeout(() => void save(), 1200);
    return () => window.clearTimeout(timer);
  }, [
    dirty,
    busy,
    editor.editEpoch,
    editor.autosavePaused,
    issues.length,
    confirmReload,
    close.pending,
    save,
  ]);

  async function publish() {
    if (revision === null || dirty || busy || ioBusy.current || mustReload)
      return;
    ioBusy.current = true;
    dispatch({ type: "publishing" });
    const result = await publishResume(revision);
    if (result.ok) {
      dispatch({ type: "published", value: result.value.published });
    } else {
      dispatch({ type: "failed", code: result.error.code });
    }
    ioBusy.current = false;
  }

  async function selectedVersionForExport(source: ExportSource) {
    if (source === "published_snapshot") return editor.published;
    if (!document) return null;
    if (!dirty) return editor.saved;
    const submittedEpoch = editor.editEpoch;
    dispatch({ type: "saving" });
    const result = await saveResume(revision, normalizeDocument(document));
    if (!result.ok) {
      dispatch({ type: "failed", code: result.error.code });
      return null;
    }
    dispatch({ type: "saved", value: result.value, submittedEpoch });
    return result.value;
  }

  async function exportSelected(format: "pdf" | "docx") {
    if (
      busy ||
      ioBusy.current ||
      mustReload ||
      confirmReload ||
      close.pending ||
      issues.length
    )
      return;
    ioBusy.current = true;
    const selected = await selectedVersionForExport(exportSource);
    if (!selected) {
      ioBusy.current = false;
      return;
    }
    if (format === "pdf") {
      dispatch({ type: "rendering" });
      const rendered = await renderResumePdf(
        exportSource,
        selected.revision,
        documentStyle,
      );
      if (!rendered.ok) {
        dispatch({
          type: "export-finished",
          notice: pdfFailure(rendered.error.code),
        });
        ioBusy.current = false;
        return;
      }
      dispatch({ type: "exporting" });
      const exported = await exportResumePdf(rendered.value);
      await releaseResumePdf(rendered.value.renderId);
      dispatch({
        type: "export-finished",
        notice: !exported.ok
          ? pdfFailure(exported.error.code)
          : exported.value.status === "cancelled"
            ? "PDF export cancelled. No file was created."
            : "PDF exported successfully.",
      });
      ioBusy.current = false;
      return;
    }
    dispatch({ type: "exporting" });
    const result = await exportResumeDocument(
      exportSource,
      selected.revision,
      "docx",
      documentStyle,
    );
    dispatch({
      type: "export-finished",
      notice: exportFeedback(result, "docx"),
    });
    ioBusy.current = false;
  }

  return {
    editor,
    dispatch,
    health,
    close,
    loadWorkspace,
    save,
    publish,
    exportSelected,
    ioBusy,
    setHealth,
  };
}
