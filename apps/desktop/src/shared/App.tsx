import { duplicateEntryGroups } from "./duplicate-hints";
import { entryGuidance } from "./entry-guidance";
import {
  validationDestination,
  focusValidationField,
  type ValidationFocusRequest,
} from "./validation-navigation";
import {
  cloneElement,
  isValidElement,
  useCallback,
  useEffect,
  useId,
  useMemo,
  useReducer,
  useRef,
  useState,
} from "react";
import type { HealthResponse } from "@ort/contracts/health";
import type {
  DocumentStyle,
  ExportSource,
  ExportFormat,
} from "@ort/contracts/export";
import {
  SELECTABLE_DOCUMENT_STYLES,
  DOCUMENT_STYLE_LABELS,
  DOCUMENT_STYLE_DESCRIPTIONS,
} from "./document-styles";
import { dateText } from "./resume-dates";
import { exportFeedback } from "./text-export";
import type {
  Link,
  ResumeDocument,
  ResumeEntry,
  ResumeSection,
} from "@ort/contracts/resume";
import { DOCUMENT_LIMITS, MAX_RESUME_DATES } from "@ort/contracts/resume";
import { ConfirmRemoval } from "./ConfirmRemoval";
import { duplicateEntry } from "./duplicate-entry";
import { DateEditor } from "./DateEditor";
import { CloseDialog } from "./CloseDialog";
import { BackupPanel } from "./BackupPanel";
import { PdfPreviewPanel } from "./PdfPreview";
import { StoragePanel } from "./StoragePanel";
import { ResumeStart } from "./ResumeStart";
import { DocumentImport } from "./DocumentImport";
import {
  createStartingSections,
  SUGGESTED_SECTIONS,
} from "./starting-profiles";
import { useCloseGuard } from "./use-close-guard";
import {
  editorReducer,
  initialEditorState,
  isDirty,
  requiresReload,
} from "./editor-state";
import {
  documentUsage,
  validateEditorDocument,
  type ValidationIssue,
} from "./resume-validation";
import {
  publishResume,
  requestHealth,
  requestResumeWorkspace,
  saveResume,
  exportResumeDocument,
} from "./command-client";
import {
  createBullet,
  createEntry,
  createResumeDocument,
  createSection,
  createNamedField,
  moveItem,
  normalizeDocument,
  upgradeDocumentV2,
} from "./resume-editor";

type Surface = "main" | "overlay";
type EntryTextField = "heading" | "subheading" | "dateRange" | "location";
type HealthState =
  | { kind: "checking" }
  | { kind: "ready"; health: HealthResponse }
  | { kind: "error"; message: string };

export function App({ surface }: { surface: Surface }) {
  if (surface === "overlay") return <OverlayStatus />;
  return <ResumeEditor />;
}

function ResumeEditor() {
  const [health, setHealth] = useState<HealthState>({ kind: "checking" });
  const [editor, dispatch] = useReducer(editorReducer, initialEditorState);
  const [importActive, setImportActive] = useState(false);
  const [importWorking, setImportWorking] = useState(false);
  const close = useCloseGuard(
    importWorking ? { ...editor, status: "exporting" } : editor,
  );
  const [confirmReload, setConfirmReload] = useState(false);
  const [exportFormat, setExportFormat] = useState<ExportFormat>("txt");
  const [documentStyle, setDocumentStyle] =
    useState<DocumentStyle>("technical");
  const [profileGeneration, setProfileGeneration] = useState(0);
  const [focusedPart, setFocusedPart] = useState<string | null>("contact");
  const [validationFocus, setValidationFocus] =
    useState<ValidationFocusRequest | null>(null);
  const [navigatorOpen, setNavigatorOpen] = useState(true);
  const [readingZoom, setReadingZoom] = useState(100);
  const [resumeView, setResumeView] = useState<"reading" | "pdf">("reading");
  const [previewTarget, setPreviewTarget] = useState<HTMLDivElement | null>(
    null,
  );
  const focusPanel = useRef<HTMLDivElement>(null);
  const readingHeading = useRef<HTMLHeadingElement>(null);
  const focusRequested = useRef(false);
  function openEditor(part: string) {
    setValidationFocus(null);
    focusRequested.current = true;
    setFocusedPart(part);
  }
  useEffect(() => {
    if (focusRequested.current) {
      focusRequested.current = false;
      if (!validationFocus) focusPanel.current?.focus();
    }
  }, [focusedPart, validationFocus]);
  useEffect(() => {
    if (validationFocus && !validationFocus.entryId) {
      focusValidationField(focusPanel.current, validationFocus.path);
    }
  }, [validationFocus]);
  const [suggestedSection, setSuggestedSection] =
    useState<string>("Custom Section");
  const firstContactField = useRef<HTMLInputElement>(null);
  const focusAfterStart = useRef(false);
  const ioBusy = useRef(false);
  const loadGeneration = useRef(0);
  const { document, notice } = editor;
  const revision = editor.saved?.revision ?? null;
  const publishedRevision = editor.published?.revision ?? null;
  const dirty = isDirty(editor);
  const showStart =
    document !== null && editor.saved === null && editor.editEpoch === 0;
  useEffect(() => {
    if (!showStart && focusAfterStart.current) {
      firstContactField.current?.focus();
      focusAfterStart.current = false;
    }
  }, [showStart]);
  // An untouched placeholder has no user edits to save before backup/recovery.
  // Keep ordinary dirty semantics for save, publication, and quit; never waive
  // the backup guard after an edit (including undo) or for a stored draft.
  const backupDirty = dirty && (editor.saved !== null || editor.editEpoch > 0);
  const busy = editor.status !== "idle" || importActive;
  const mustReload = requiresReload(editor);
  const issues = useMemo(
    () => (document ? validateEditorDocument(document) : []),
    [document],
  );
  const usage = document ? documentUsage(document) : null;
  const duplicateGroups = useMemo(
    () => (document ? duplicateEntryGroups(document) : []),
    [document],
  );
  function openEntry(sectionId: string, entryId: string) {
    openEditor(sectionId);
    setValidationFocus((previous) => ({
      path: `entry.${entryId}.heading`,
      entryId,
      sequence: (previous?.sequence ?? 0) + 1,
    }));
  }

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
      empty: createResumeDocument(),
    });
  }, []);

  useEffect(() => {
    void loadWorkspace();
    return () => {
      loadGeneration.current += 1;
    };
  }, [loadWorkspace]);

  function changeDocument(update: (current: ResumeDocument) => ResumeDocument) {
    dispatch({ type: "edit", update });
  }

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

  async function exportDocument(source: ExportSource) {
    const selected = source === "saved_draft" ? editor.saved : editor.published;
    if (
      !selected ||
      busy ||
      ioBusy.current ||
      mustReload ||
      confirmReload ||
      close.pending ||
      (source === "saved_draft" && dirty)
    )
      return;
    ioBusy.current = true;
    dispatch({ type: "exporting" });
    const result = await exportResumeDocument(
      source,
      selected.revision,
      exportFormat,
      documentStyle,
    );
    dispatch({
      type: "export-finished",
      notice: exportFeedback(result, exportFormat),
    });
    ioBusy.current = false;
  }

  const storageReady =
    health.kind === "ready" && health.health.storageStatus === "ready";
  const alreadyPublished =
    editor.saved !== null &&
    editor.published !== null &&
    JSON.stringify(editor.saved.document) ===
      JSON.stringify(editor.published.document);

  return (
    <main className="shell shell--editor">
      <CloseDialog
        open={close.pending}
        busy={editor.status !== "idle" || importWorking}
        resolving={close.resolving}
        canSave={!!document && dirty && !mustReload && issues.length === 0}
        error={close.error}
        saveError={editor.errorCode ? friendlyError(editor.errorCode) : null}
        onCancel={close.cancel}
        onSave={() => void save()}
        onDiscard={close.discard}
        onRetry={close.retry}
      />
      {close.error && !close.pending ? (
        <p className="notice" role="alert">
          {close.error}{" "}
          <button type="button" onClick={close.retry}>
            Retry quit connection
          </button>
        </p>
      ) : null}
      <header className="masthead masthead--workspace">
        <div className="brand-lockup">
          <div className="mark" aria-hidden="true">
            ORT
          </div>
          <div>
            <p className="eyebrow">Local development profile</p>
            <h1>Resume workspace</h1>
          </div>
        </div>
        <HealthBadge state={health} />
      </header>
      <nav className="workspace-shortcuts" aria-label="Workspace areas">
        {document && !showStart ? (
          <a href="#reading-title">Resume editor</a>
        ) : null}
        <a href="#workspace-data-title">Backup and recovery</a>
      </nav>

      {showStart ? (
        <ResumeStart
          key={`start-${profileGeneration}`}
          disabled={
            !storageReady ||
            busy ||
            mustReload ||
            confirmReload ||
            close.pending
          }
          importAction={
            <DocumentImport
              disabled={
                !storageReady ||
                busy ||
                mustReload ||
                confirmReload ||
                close.pending ||
                (dirty && !showStart)
              }
              revision={revision}
              onBusyChange={setImportActive}
              onOperationChange={setImportWorking}
              onSaved={(saved) => {
                dispatch({
                  type: "loaded",
                  workspace: {
                    draft: saved,
                    latestPublished: editor.published,
                  },
                  empty: saved.document,
                });
              }}
            />
          }
          onBuild={(profile) => {
            setFocusedPart("contact");
            focusAfterStart.current = true;
            changeDocument((current) => ({
              ...current,
              sections: createStartingSections(profile),
            }));
          }}
        />
      ) : null}

      {!showStart && document ? (
        <DocumentImport
          disabled={
            !storageReady ||
            busy ||
            dirty ||
            mustReload ||
            confirmReload ||
            close.pending
          }
          revision={revision}
          onBusyChange={setImportActive}
          onOperationChange={setImportWorking}
          onSaved={(saved) => {
            dispatch({
              type: "loaded",
              workspace: { draft: saved, latestPublished: editor.published },
              empty: saved.document,
            });
          }}
        />
      ) : null}

      <section className="workspace-summary" aria-label="Resume status">
        <div>
          <span>Draft</span>
          <strong>
            {revision === null ? "Not saved" : `Revision ${revision}`}
          </strong>
        </div>
        <div>
          <span>Published</span>
          <strong>
            {publishedRevision === null
              ? "No snapshot"
              : `Snapshot ${publishedRevision}`}
          </strong>
        </div>
        <div>
          <span>Changes</span>
          <strong>
            {editor.status === "saving"
              ? "Saving…"
              : dirty
                ? "Unsaved"
                : "Saved"}
          </strong>
        </div>
        <div className="workspace-actions">
          <button
            type="button"
            onClick={() => void save()}
            disabled={
              !storageReady ||
              !document ||
              showStart ||
              busy ||
              confirmReload ||
              close.pending ||
              mustReload ||
              issues.length > 0 ||
              (!dirty && revision !== null)
            }
          >
            {editor.status === "saving" ? "Saving…" : "Save draft"}
          </button>
          <button
            className="button--secondary"
            type="button"
            onClick={() => void publish()}
            disabled={
              !storageReady ||
              revision === null ||
              dirty ||
              busy ||
              confirmReload ||
              close.pending ||
              mustReload ||
              alreadyPublished
            }
          >
            Publish snapshot
          </button>
        </div>
      </section>

      <section className="editor-tools" aria-label="Document export">
        <label className="field">
          Document style
          <select
            value={documentStyle}
            disabled={busy || close.pending}
            aria-describedby="document-style-description document-style-scope"
            onChange={(event) => {
              const selected = SELECTABLE_DOCUMENT_STYLES.find(
                (style) => style === event.target.value,
              );
              if (selected) setDocumentStyle(selected);
            }}
          >
            {SELECTABLE_DOCUMENT_STYLES.map((style) => (
              <option key={style} value={style}>
                {DOCUMENT_STYLE_LABELS[style]}
              </option>
            ))}
          </select>
        </label>
        <p id="document-style-description" aria-live="polite">
          {DOCUMENT_STYLE_DESCRIPTIONS[documentStyle]}
        </p>
        <p id="document-style-scope">
          Applies to new PDF previews and Word exports. Your resume content
          stays the same. Plain text has no visual style. This choice lasts
          until you close the workspace; reopening starts with Technical /
          Engineering.
        </p>
        <details className="export-options">
          <summary>Text and Word export options</summary>
          <p>
            Exports are unencrypted and can be read by anyone with access to the
            destination, including synced-folder services. Choose a private
            local folder and a new filename. Export uses the selected saved
            revision, not unsaved edits. Existing files are never replaced.
          </p>

          <label>
            Export format
            <select
              value={exportFormat}
              disabled={busy || close.pending}
              onChange={(event) =>
                setExportFormat(event.target.value === "docx" ? "docx" : "txt")
              }
            >
              <option value="txt">Plain text (.txt)</option>
              <option value="docx">Word document (.docx)</option>
            </select>
          </label>
          {exportFormat === "docx" ? (
            <p>
              Word export style: {DOCUMENT_STYLE_LABELS[documentStyle]}.
              Pagination and fonts depend on your document reader. Use the PDF
              preview to review the PDF layout.
            </p>
          ) : null}
          <div className="move-controls">
            <button
              type="button"
              className="button--secondary"
              onClick={() => void exportDocument("saved_draft")}
              disabled={
                !storageReady ||
                !editor.saved ||
                dirty ||
                busy ||
                mustReload ||
                confirmReload ||
                close.pending
              }
            >
              Export saved draft (.{exportFormat})
            </button>
            <button
              type="button"
              className="button--secondary"
              onClick={() => void exportDocument("published_snapshot")}
              disabled={
                !storageReady ||
                !editor.published ||
                busy ||
                mustReload ||
                confirmReload ||
                close.pending
              }
            >
              Export published snapshot (.{exportFormat})
            </button>
          </div>
          {editor.status === "exporting" ? (
            <p role="status">
              Export in progress — finish or cancel the native Save dialog.
            </p>
          ) : null}
        </details>
      </section>

      <PdfPreviewPanel
        key={profileGeneration}
        active={resumeView === "pdf"}
        previewTarget={previewTarget}
        style={documentStyle}
        saved={editor.saved}
        published={editor.published}
        dirty={dirty}
        blocked={
          !storageReady || busy || mustReload || confirmReload || close.pending
        }
        onBegin={(kind) => {
          if (
            ioBusy.current ||
            busy ||
            mustReload ||
            confirmReload ||
            close.pending
          )
            return false;
          ioBusy.current = true;
          dispatch({ type: kind });
          return true;
        }}
        onFinish={(message) => {
          dispatch({ type: "export-finished", notice: message });
          ioBusy.current = false;
        }}
        semantic={(document) => <PublishedResume document={document} pdf />}
      />

      <div className="editor-tools">
        <p>
          Valid changes autosave after a short pause. Closing checks for unsaved
          edits; invalid edits must be corrected or explicitly discarded. Use
          the app menu or window close button. macOS Dock Quit and system
          shutdown checks are still pending; wait for Saved first. Use synthetic
          data only.
        </p>
        <div className="move-controls">
          <button
            type="button"
            className="button--secondary button--compact"
            disabled={
              !editor.undo.length ||
              editor.undo.at(-1)?.schemaVersion !== document?.schemaVersion ||
              editor.status === "loading" ||
              confirmReload ||
              close.pending
            }
            onClick={() => dispatch({ type: "undo" })}
          >
            Undo edit
          </button>
          <button
            type="button"
            className="button--secondary button--compact"
            disabled={
              !editor.redo.length ||
              editor.status === "loading" ||
              confirmReload ||
              close.pending
            }
            onClick={() => dispatch({ type: "redo" })}
          >
            Redo edit
          </button>
        </div>
        <button
          type="button"
          className="button--secondary button--compact"
          disabled={busy || close.pending}
          onClick={() =>
            dirty && editor.editEpoch > 0
              ? setConfirmReload(true)
              : void loadWorkspace()
          }
        >
          Reload saved draft
        </button>
      </div>
      {confirmReload ? (
        <section className="notice" aria-label="Confirm reload">
          <p>
            Reloading discards unsaved edits in this window. Keep editing if you
            need to preserve them.
          </p>
          <div className="editor-tools">
            <button type="button" onClick={() => setConfirmReload(false)}>
              Keep editing
            </button>
            <button
              type="button"
              className="button--danger"
              onClick={() => {
                setConfirmReload(false);
                void loadWorkspace();
              }}
            >
              Discard unsaved edits and reload
            </button>
          </div>
        </section>
      ) : null}
      {editor.errorCode ? (
        <p className="notice" role="alert">
          {friendlyError(editor.errorCode)} Autosave is paused.
        </p>
      ) : null}
      {issues.length ? (
        <section
          className="notice"
          aria-label="Resume validation"
          role="status"
        >
          <p>Correct these items before saving:</p>
          <ul>
            {issues.map((issue, index) => (
              <li key={`${issue.path}-${index}`}>
                {document && validationDestination(document, issue.path) ? (
                  <button
                    type="button"
                    className="button--quiet"
                    disabled={busy || mustReload}
                    onClick={() => {
                      const destination = validationDestination(
                        document,
                        issue.path,
                      );
                      if (!destination) return;
                      openEditor(destination.part);
                      setValidationFocus((previous) => ({
                        path: issue.path,
                        entryId: destination.entryId,
                        sequence: (previous?.sequence ?? 0) + 1,
                      }));
                    }}
                  >
                    {validationDestination(document, issue.path)?.label}:{" "}
                    {issue.message}
                  </button>
                ) : (
                  issue.message
                )}
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {duplicateGroups.length ? (
        <details className="notice duplicate-hints">
          <summary>
            Matching entries found ({duplicateGroups.length}{" "}
            {duplicateGroups.length === 1 ? "group" : "groups"})
          </summary>
          <p>
            These entries have matching completed fields. They may be
            intentional; review them if needed. Saving and publishing remain
            available when the draft is valid.
          </p>
          {duplicateGroups.map((group, index) => (
            <ul key={index} aria-label={`Matching entry group ${index + 1}`}>
              {group.map((item) => (
                <li key={item.entryId}>
                  <button
                    type="button"
                    className="button--quiet"
                    disabled={busy || mustReload}
                    onClick={() => openEntry(item.sectionId, item.entryId)}
                  >
                    {item.label}
                  </button>
                </li>
              ))}
            </ul>
          ))}
        </details>
      ) : null}

      {notice ? (
        <p className="notice" role="status">
          {notice}
        </p>
      ) : null}

      {document && !showStart ? (
        <fieldset
          className="editor-layout editor-fields"
          disabled={
            editor.status === "loading" ||
            editor.status === "deleting" ||
            confirmReload ||
            close.pending
          }
        >
          <div
            className={`document-workspace${navigatorOpen ? "" : " document-workspace--collapsed"}`}
          >
            <nav
              className="document-navigator"
              aria-label="Resume section navigation"
            >
              <button
                type="button"
                aria-expanded={navigatorOpen}
                aria-controls="resume-navigation"
                onClick={() => setNavigatorOpen(!navigatorOpen)}
              >
                {navigatorOpen ? "Collapse sections" : "Show sections"}
              </button>
              {navigatorOpen ? (
                <div id="resume-navigation">
                  <button
                    type="button"
                    aria-current={
                      focusedPart === "contact" ? "location" : undefined
                    }
                    onClick={() => openEditor("contact")}
                  >
                    Edit contact
                  </button>
                  {document.sections.map((section) => (
                    <button
                      key={section.id}
                      type="button"
                      aria-current={
                        focusedPart === section.id ? "location" : undefined
                      }
                      onClick={() => openEditor(section.id)}
                    >
                      Edit {section.heading || "Untitled section"}
                    </button>
                  ))}
                  <button type="button" onClick={() => openEditor("sections")}>
                    Manage sections
                  </button>
                </div>
              ) : null}
            </nav>
            <section
              className="resume-reading-panel"
              aria-labelledby="reading-title"
            >
              <h2 id="reading-title" ref={readingHeading} tabIndex={-1}>
                Your resume
              </h2>
              <label>
                Resume view{" "}
                <select
                  value={resumeView}
                  onChange={(event) =>
                    setResumeView(
                      event.target.value === "pdf" ? "pdf" : "reading",
                    )
                  }
                >
                  <option value="reading">Live reading and editing</option>
                  <option value="pdf">Exact PDF pages</option>
                </select>
              </label>
              <div hidden={resumeView !== "reading"}>
                <p>
                  Live draft reading view ·{" "}
                  {DOCUMENT_STYLE_LABELS[documentStyle]}. Includes unsaved
                  edits. Use PDF preview for exact pages, fonts and export
                  layout.
                </p>
                <label>
                  Reading zoom{" "}
                  <select
                    value={readingZoom}
                    onChange={(event) =>
                      setReadingZoom(Number(event.target.value))
                    }
                  >
                    <option value={100}>100%</option>
                    <option value={125}>125%</option>
                    <option value={150}>150%</option>
                  </select>
                </label>
                <div
                  className={`resume-reading-document resume-reading-document--${documentStyle}`}
                  style={{ fontSize: `${readingZoom}%` }}
                >
                  <PublishedResume
                    document={document}
                    onSelect={openEditor}
                    onSelectEntry={openEntry}
                    canAddEntry={
                      !busy &&
                      !mustReload &&
                      usage !== null &&
                      usage.entries < DOCUMENT_LIMITS.entries
                    }
                    onAddEntry={(sectionId) => {
                      const section = document.sections.find(
                        (item) => item.id === sectionId,
                      );
                      if (
                        !section ||
                        busy ||
                        mustReload ||
                        !usage ||
                        usage.entries >= DOCUMENT_LIMITS.entries
                      )
                        return;
                      const entry = createEntry(section.entries.length);
                      changeDocument((current) => ({
                        ...current,
                        sections: current.sections.map((item) =>
                          item.id === sectionId
                            ? { ...item, entries: [...item.entries, entry] }
                            : item,
                        ),
                      }));
                      openEntry(sectionId, entry.id);
                    }}
                  />
                </div>
              </div>
              <div ref={setPreviewTarget} hidden={resumeView !== "pdf"} />
            </section>
            {focusedPart !== null ? (
              <div
                className="focused-editor"
                ref={focusPanel}
                tabIndex={-1}
                role="region"
                aria-label="Focused resume editor"
              >
                <button
                  type="button"
                  className="button--secondary"
                  onClick={() => {
                    setFocusedPart(null);
                    readingHeading.current?.focus();
                  }}
                >
                  Close editor
                </button>
                {document.schemaVersion === 1 ? (
                  <section
                    className="editor-panel"
                    aria-labelledby="upgrade-heading"
                  >
                    <h2 id="upgrade-heading">
                      Structured dates and ordered links
                    </h2>
                    <p>
                      Enable date fields and link ordering for this draft.
                      Existing date text and published snapshots are preserved.
                      Older app versions cannot read the upgraded draft. You can
                      undo later content edits, but not this format upgrade.
                    </p>
                    <button
                      type="button"
                      disabled={busy || mustReload || Boolean(issues.length)}
                      onClick={() => changeDocument(upgradeDocumentV2)}
                    >
                      Enable structured dates and link ordering
                    </button>
                  </section>
                ) : null}
                {focusedPart === "contact" ? (
                  <section
                    className="editor-panel"
                    aria-labelledby="identity-heading"
                  >
                    <div className="section-heading">
                      <div>
                        <p className="eyebrow">Structured resume</p>
                        <h2 id="identity-heading">Identity and contact</h2>
                      </div>
                    </div>
                    <div className="field-grid">
                      <Field
                        label="Resume title"
                        wide
                        path={"title"}
                        error={issueAt(issues, "title")}
                      >
                        <input
                          value={document.title}
                          maxLength={2000}
                          required
                          onChange={(event) =>
                            changeDocument((current) => ({
                              ...current,
                              title: event.target.value,
                            }))
                          }
                        />
                      </Field>
                      {(
                        [
                          ["fullName", "Full name"],
                          ["email", "Email"],
                          ["phone", "Phone"],
                          ["location", "Location"],
                        ] as const
                      ).map(([field, label]) => (
                        <Field
                          key={field}
                          label={label}
                          path={`contact.${field}`}
                          error={issueAt(issues, `contact.${field}`)}
                        >
                          <input
                            ref={
                              field === "fullName"
                                ? firstContactField
                                : undefined
                            }
                            value={document.contact[field]}
                            maxLength={2000}
                            type={field === "email" ? "email" : "text"}
                            onChange={(event) =>
                              changeDocument((current) => ({
                                ...current,
                                contact: {
                                  ...current.contact,
                                  [field]: event.target.value,
                                },
                              }))
                            }
                          />
                        </Field>
                      ))}
                    </div>
                    <LinksEditor
                      links={document.contact.links}
                      path="contact.links"
                      issues={issues}
                      canAdd={usage!.links < DOCUMENT_LIMITS.links}
                      onChange={(links) =>
                        changeDocument((current) => ({
                          ...current,
                          contact: { ...current.contact, links },
                        }))
                      }
                    />
                  </section>
                ) : (
                  <section
                    className="editor-panel"
                    aria-labelledby="sections-heading"
                  >
                    <div className="section-heading">
                      <div>
                        <p className="eyebrow">Ordered content</p>
                        <h2 id="sections-heading">Resume sections</h2>
                      </div>
                      <label>
                        Section to add
                        <select
                          value={suggestedSection}
                          onChange={(event) =>
                            setSuggestedSection(event.target.value)
                          }
                        >
                          {SUGGESTED_SECTIONS.map((heading) => (
                            <option key={heading} value={heading}>
                              {heading}
                            </option>
                          ))}
                        </select>
                      </label>
                      <button
                        className="button--secondary button--compact"
                        type="button"
                        disabled={usage!.sections >= DOCUMENT_LIMITS.sections}
                        onClick={() => {
                          const next = {
                            ...createSection(document.sections.length),
                            heading: suggestedSection,
                          };
                          changeDocument((current) => ({
                            ...current,
                            sections: [...current.sections, next],
                          }));
                          openEditor(next.id);
                        }}
                      >
                        Add section
                      </button>
                    </div>

                    {document.sections.length === 0 ? (
                      <div className="empty-state">
                        <p>
                          Add sections such as Experience, Education, or Skills.
                        </p>
                      </div>
                    ) : null}

                    <div className="section-list">
                      {document.sections.map((section, sectionIndex) =>
                        section.id === focusedPart ? (
                          <ResumeSectionEditor
                            key={section.id}
                            section={section}
                            validationFocus={validationFocus}
                            index={sectionIndex}
                            count={document.sections.length}
                            issues={issues}
                            usage={usage!}
                            onMove={(direction) =>
                              changeDocument((current) => ({
                                ...current,
                                sections: moveItem(
                                  current.sections,
                                  section.id,
                                  direction,
                                ),
                              }))
                            }
                            onChange={(next) =>
                              changeDocument((current) => ({
                                ...current,
                                sections: current.sections.map((candidate) =>
                                  candidate.id === section.id
                                    ? next
                                    : candidate,
                                ),
                              }))
                            }
                            onRemove={() => {
                              openEditor("sections");
                              changeDocument((current) => ({
                                ...current,
                                sections: current.sections.filter(
                                  (candidate) => candidate.id !== section.id,
                                ),
                              }));
                            }}
                          />
                        ) : null,
                      )}
                    </div>
                  </section>
                )}
              </div>
            ) : null}
          </div>
        </fieldset>
      ) : !document ? (
        <section className="status-card">
          <h2>Opening encrypted workspace</h2>
          <p className="description">
            The application is connecting to its isolated database and OS
            credential vault.
          </p>
          {!busy ? (
            <button type="button" onClick={() => void loadWorkspace()}>
              Try again
            </button>
          ) : null}
        </section>
      ) : null}

      {editor.published ? (
        <details className="editor-panel published-review">
          <summary>
            Review published snapshot {editor.published.revision} (read-only)
          </summary>
          <PublishedResume document={editor.published.document} />
        </details>
      ) : null}

      <section
        aria-labelledby="workspace-data-title"
        className="workspace-data"
      >
        <h2 id="workspace-data-title" tabIndex={-1}>
          Backup, recovery, and local storage
        </h2>
        <BackupPanel
          dirty={backupDirty}
          blocked={
            !storageReady ||
            backupDirty ||
            busy ||
            mustReload ||
            confirmReload ||
            close.pending
          }
          onBegin={() => {
            if (
              ioBusy.current ||
              backupDirty ||
              busy ||
              mustReload ||
              confirmReload ||
              close.pending
            )
              return false;
            ioBusy.current = true;
            dispatch({ type: "exporting" });
            return true;
          }}
          onFinish={(message) => {
            dispatch({ type: "export-finished", notice: message });
            ioBusy.current = false;
          }}
        />

        <StoragePanel
          enabled={
            storageReady &&
            !busy &&
            !mustReload &&
            !confirmReload &&
            !close.pending
          }
          onDeleteBegin={() => {
            if (
              ioBusy.current ||
              busy ||
              mustReload ||
              confirmReload ||
              close.pending
            )
              return false;
            ioBusy.current = true;
            dispatch({ type: "deleting" });
            return true;
          }}
          onDeleteFinish={(committed, freshProfileReady) => {
            ioBusy.current = false;
            if (!committed) {
              dispatch({ type: "delete-finished" });
              return;
            }
            setProfileGeneration((value) => value + 1);
            dispatch({ type: "data-deleted" });
            if (freshProfileReady) {
              void loadWorkspace();
            } else {
              setHealth({
                kind: "error",
                message: "Local data cleanup requires restart.",
              });
            }
          }}
        />
      </section>

      <footer className="development-gates">
        PDF preview, PDF, DOCX and text export use saved revisions. PDF/DOCX
        import, export replacement and broader crash cleanup remain gated in M2;
        backup replacement is staged into a fresh encrypted profile and applied
        on restart with the previous profile retained as a manageable safety
        copy. AI and browser integration arrive in later milestones. Use
        synthetic data only.
      </footer>
    </main>
  );
}

function ResumeSectionEditor({
  validationFocus,
  section,
  onChange,
  onRemove,
  onMove,
  index,
  count,
  issues,
  usage,
}: {
  section: ResumeSection;
  validationFocus: ValidationFocusRequest | null;
  onChange: (section: ResumeSection) => void;
  onRemove: () => void;
  onMove: (direction: -1 | 1) => void;
  index: number;
  count: number;
  issues: ValidationIssue[];
  usage: ReturnType<typeof documentUsage>;
}) {
  const [expandedEntry, setExpandedEntry] = useState<string | null>(
    section.entries[0]?.id ?? null,
  );
  const sectionElement = useRef<HTMLElement>(null);
  const entryFocusRequested = useRef(false);
  useEffect(() => {
    if (validationFocus?.entryId) setExpandedEntry(validationFocus.entryId);
  }, [validationFocus]);
  useEffect(() => {
    if (validationFocus?.entryId === expandedEntry)
      focusValidationField(sectionElement.current, validationFocus.path);
  }, [validationFocus, expandedEntry]);
  useEffect(() => {
    if (!entryFocusRequested.current) return;
    entryFocusRequested.current = false;
    sectionElement.current
      ?.querySelector<HTMLElement>(
        expandedEntry ? '.entry-focus-toggle[aria-expanded="true"]' : "input",
      )
      ?.focus();
  }, [expandedEntry]);
  return (
    <article ref={sectionElement} className="resume-section">
      <div className="resume-section__header">
        <Field
          label="Section heading"
          wide
          path={`section.${section.id}.heading`}
          error={issueAt(issues, `section.${section.id}.heading`)}
        >
          <input
            value={section.heading}
            maxLength={2000}
            required
            onChange={(event) =>
              onChange({ ...section, heading: event.target.value })
            }
          />
        </Field>
        <MoveControls
          label={`section ${index + 1}`}
          index={index}
          count={count}
          onMove={onMove}
        />
        <ConfirmRemoval
          label="Remove section"
          description={`Remove ${section.heading || "this section"} and all ${section.entries.length} entries?`}
          onRemove={onRemove}
        />
      </div>

      {section.entries.map((entry, entryIndex) => (
        <div className="entry-focus-group" key={entry.id}>
          <button
            type="button"
            className="entry-focus-toggle"
            aria-expanded={expandedEntry === entry.id}
            aria-controls={`entry-editor-${entry.id}`}
            onClick={() =>
              setExpandedEntry(expandedEntry === entry.id ? null : entry.id)
            }
          >
            {entry.heading.trim() || `Entry ${entryIndex + 1}`} ·{" "}
            {expandedEntry === entry.id ? "Collapse" : "Edit"}
          </button>
          {expandedEntry === entry.id ? (
            <div id={`entry-editor-${entry.id}`}>
              <ResumeEntryEditor
                key={entry.id}
                entry={entry}
                sectionHeading={section.heading}
                issues={issues}
                usage={usage}
                index={entryIndex}
                count={section.entries.length}
                canDuplicate={
                  usage.entries < DOCUMENT_LIMITS.entries &&
                  usage.bullets + entry.bullets.length <=
                    DOCUMENT_LIMITS.bullets &&
                  usage.links + entry.links.length <= DOCUMENT_LIMITS.links &&
                  usage.dates + (entry.dates?.length ?? 0) <=
                    MAX_RESUME_DATES &&
                  usage.skills +
                    entry.fields.filter((field) => field.isSkill).length <=
                    DOCUMENT_LIMITS.skills
                }
                onDuplicate={() => {
                  const copied = duplicateEntry(entry);
                  const entries = [...section.entries];
                  entries.splice(entryIndex + 1, 0, copied);
                  onChange({ ...section, entries });
                  entryFocusRequested.current = true;
                  setExpandedEntry(copied.id);
                }}
                onMove={(direction) =>
                  onChange({
                    ...section,
                    entries: moveItem(section.entries, entry.id, direction),
                  })
                }
                onChange={(next) =>
                  onChange({
                    ...section,
                    entries: section.entries.map((candidate) =>
                      candidate.id === entry.id ? next : candidate,
                    ),
                  })
                }
                onRemove={() => {
                  entryFocusRequested.current = true;
                  setExpandedEntry(null);
                  onChange({
                    ...section,
                    entries: section.entries.filter(
                      (candidate) => candidate.id !== entry.id,
                    ),
                  });
                }}
              />
            </div>
          ) : null}
        </div>
      ))}

      <button
        className="button--quiet button--compact"
        type="button"
        disabled={usage.entries >= DOCUMENT_LIMITS.entries}
        onClick={() => {
          const entry = createEntry(section.entries.length);
          onChange({ ...section, entries: [...section.entries, entry] });
          entryFocusRequested.current = true;
          setExpandedEntry(entry.id);
        }}
      >
        Add entry
      </button>
    </article>
  );
}

function ResumeEntryEditor({
  entry,
  sectionHeading,
  onDuplicate,
  canDuplicate,
  onChange,
  onRemove,
  onMove,
  index,
  count,
  issues,
  usage,
}: {
  entry: ResumeEntry;
  sectionHeading: string;
  onDuplicate: () => void;
  canDuplicate: boolean;
  onChange: (entry: ResumeEntry) => void;
  onRemove: () => void;
  onMove: (direction: -1 | 1) => void;
  index: number;
  count: number;
  issues: ValidationIssue[];
  usage: ReturnType<typeof documentUsage>;
}) {
  const guidance = entryGuidance(sectionHeading);
  const [detailChoice, setDetailChoice] = useState("");
  const selectedDetail = guidance.details.includes(detailChoice)
    ? detailChoice
    : guidance.details[0];
  const entryElement = useRef<HTMLDivElement>(null);
  const newFieldFocus = useRef<string | null>(null);
  useEffect(() => {
    if (!newFieldFocus.current) return;
    entryElement.current
      ?.querySelector<HTMLElement>(
        `[data-validation-path="${newFieldFocus.current}"] :is(input, textarea)`,
      )
      ?.focus();
    newFieldFocus.current = null;
  }, [entry.fields]);
  function addDetail(label: string, isSkill = false) {
    if (isSkill && usage.skills >= DOCUMENT_LIMITS.skills) return;
    const field = { ...createNamedField(entry.fields.length), label, isSkill };
    newFieldFocus.current = `field.${field.id}.${label ? "value" : "label"}`;
    onChange({ ...entry, fields: [...entry.fields, field] });
  }
  function updateField(field: EntryTextField, value: string) {
    onChange({ ...entry, [field]: value });
  }

  return (
    <div className="resume-entry" ref={entryElement}>
      <div className="entry-actions">
        <strong>Entry {entry.order + 1}</strong>
        <MoveControls
          label={`entry ${index + 1}`}
          index={index}
          count={count}
          onMove={onMove}
        />
        <button
          type="button"
          className="button--secondary button--compact"
          disabled={!canDuplicate}
          onClick={onDuplicate}
        >
          Duplicate entry
        </button>
        <ConfirmRemoval
          label="Remove entry"
          description={`Remove ${entry.heading || `entry ${index + 1}`} and its details?`}
          onRemove={onRemove}
        />
      </div>
      <div className="field-grid">
        {(
          [
            ["heading", guidance.heading],
            ["subheading", guidance.subheading],
            ["dateRange", "Date range"],
            ["location", "Location"],
          ] as const
        ).map(([field, label]) => (
          <Field
            key={field}
            label={label}
            path={`entry.${entry.id}.${field}`}
            error={issueAt(issues, `entry.${entry.id}.${field}`)}
          >
            <input
              value={entry[field]}
              placeholder={
                field === "heading"
                  ? guidance.headingExample
                  : field === "subheading"
                    ? guidance.subheadingExample
                    : undefined
              }
              disabled={field === "dateRange" && Boolean(entry.dates?.length)}
              maxLength={2000}
              onChange={(event) => updateField(field, event.target.value)}
            />
          </Field>
        ))}
      </div>

      <div className="bullet-list">
        {entry.bullets.map((bullet, bulletIndex) => (
          <div className="bullet-row" key={bullet.id}>
            <Field
              label={`Bullet ${bulletIndex + 1}`}
              path={`bullet.${bullet.id}`}
              error={issueAt(issues, `bullet.${bullet.id}`)}
            >
              <textarea
                value={bullet.text}
                maxLength={500}
                rows={2}
                onChange={(event) =>
                  onChange({
                    ...entry,
                    bullets: entry.bullets.map((candidate) =>
                      candidate.id === bullet.id
                        ? { ...candidate, text: event.target.value }
                        : candidate,
                    ),
                  })
                }
              />
            </Field>
            <MoveControls
              label={`bullet ${bulletIndex + 1}`}
              index={bulletIndex}
              count={entry.bullets.length}
              onMove={(direction) =>
                onChange({
                  ...entry,
                  bullets: moveItem(entry.bullets, bullet.id, direction),
                })
              }
            />
            <button
              className="button--secondary button--compact"
              type="button"
              disabled={usage.bullets >= DOCUMENT_LIMITS.bullets}
              aria-label={`Duplicate bullet ${bulletIndex + 1}`}
              onClick={() => {
                const bullets = [...entry.bullets];
                bullets.splice(bulletIndex + 1, 0, {
                  ...createBullet(bulletIndex + 1),
                  text: bullet.text,
                });
                onChange({ ...entry, bullets });
              }}
            >
              Duplicate
            </button>
            <button
              className="button--danger button--icon"
              type="button"
              aria-label={`Remove bullet ${bulletIndex + 1}`}
              onClick={() =>
                onChange({
                  ...entry,
                  bullets: entry.bullets.filter(
                    (candidate) => candidate.id !== bullet.id,
                  ),
                })
              }
            >
              ×
            </button>
          </div>
        ))}
        <button
          className="button--quiet button--compact"
          type="button"
          disabled={usage.bullets >= DOCUMENT_LIMITS.bullets}
          onClick={() =>
            onChange({
              ...entry,
              bullets: [...entry.bullets, createBullet(entry.bullets.length)],
            })
          }
        >
          Add bullet
        </button>
      </div>
      <details className="custom-fields">
        <summary>
          Add optional detail
          {entry.fields.length ? ` (${entry.fields.length})` : ""}
        </summary>
        <p>
          Use labeled details for a degree, GPA, coursework, technologies,
          credential ID, or a skill group. Only information you enter appears on
          your resume.
        </p>
        <div className="detail-shortcuts">
          <label className="field">
            Suggested detail
            <select
              value={selectedDetail}
              onChange={(event) => setDetailChoice(event.target.value)}
            >
              {guidance.details.map((label) => (
                <option key={label} value={label}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <button
            type="button"
            className="button--secondary button--compact"
            disabled={
              selectedDetail === "Skill" &&
              usage.skills >= DOCUMENT_LIMITS.skills
            }
            onClick={() =>
              addDetail(selectedDetail, selectedDetail === "Skill")
            }
          >
            Add suggested detail
          </button>
        </div>
        {entry.fields.map((field, fieldIndex) => (
          <div className="custom-field" key={field.id}>
            <Field
              label={`Field ${fieldIndex + 1} label`}
              path={`field.${field.id}.label`}
              error={issueAt(issues, `field.${field.id}.label`)}
            >
              <input
                value={field.label}
                maxLength={DOCUMENT_LIMITS.fieldCharacters}
                onChange={(event) =>
                  onChange({
                    ...entry,
                    fields: entry.fields.map((item) =>
                      item.id === field.id
                        ? { ...item, label: event.target.value }
                        : item,
                    ),
                  })
                }
              />
            </Field>
            <Field
              label={`Field ${fieldIndex + 1} value`}
              path={`field.${field.id}.value`}
              error={issueAt(issues, `field.${field.id}.value`)}
            >
              <textarea
                value={field.value}
                maxLength={DOCUMENT_LIMITS.fieldCharacters}
                rows={2}
                onChange={(event) =>
                  onChange({
                    ...entry,
                    fields: entry.fields.map((item) =>
                      item.id === field.id
                        ? { ...item, value: event.target.value }
                        : item,
                    ),
                  })
                }
              />
            </Field>
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={field.isSkill}
                disabled={
                  !field.isSkill && usage.skills >= DOCUMENT_LIMITS.skills
                }
                onChange={(event) =>
                  onChange({
                    ...entry,
                    fields: entry.fields.map((item) =>
                      item.id === field.id
                        ? { ...item, isSkill: event.target.checked }
                        : item,
                    ),
                  })
                }
              />
              Skill
            </label>
            <MoveControls
              label={`field ${fieldIndex + 1}`}
              index={fieldIndex}
              count={entry.fields.length}
              onMove={(direction) =>
                onChange({
                  ...entry,
                  fields: moveItem(entry.fields, field.id, direction),
                })
              }
            />
            <button
              type="button"
              className="button--danger button--compact"
              onClick={() =>
                onChange({
                  ...entry,
                  fields: entry.fields.filter((item) => item.id !== field.id),
                })
              }
            >
              Remove field {fieldIndex + 1}
            </button>
          </div>
        ))}
        <button
          type="button"
          className="button--quiet button--compact"
          onClick={() => addDetail("")}
        >
          Add custom field
        </button>
      </details>
      {entry.dates !== undefined ? (
        <DateEditor
          entry={entry}
          canAdd={usage.dates < MAX_RESUME_DATES}
          onChange={onChange}
        />
      ) : null}
      <LinksEditor
        links={entry.links}
        path={`entry.${entry.id}.links`}
        issues={issues}
        canAdd={usage.links < DOCUMENT_LIMITS.links}
        onChange={(links) => onChange({ ...entry, links })}
      />
    </div>
  );
}

function Field({
  path,
  label,
  wide = false,
  children,
  error,
}: {
  label: string;
  path?: string;
  wide?: boolean;
  children: React.ReactNode;
  error?: string;
}) {
  const id = useId();
  return (
    <label
      data-validation-path={path}
      className={wide ? "field field--wide" : "field"}
      htmlFor={id}
    >
      <span>{label}</span>
      {cloneInputWithId(children, id, error)}
      {error ? (
        <span className="field-error" id={`${id}-error`}>
          {error}
        </span>
      ) : null}
    </label>
  );
}

function cloneInputWithId(
  children: React.ReactNode,
  id: string,
  error?: string,
) {
  if (
    !isValidElement<{
      id?: string;
      "aria-invalid"?: boolean;
      "aria-describedby"?: string;
    }>(children)
  )
    return children;
  return cloneElement(children, {
    id,
    "aria-invalid": !!error,
    "aria-describedby": error ? `${id}-error` : undefined,
  });
}

function issueAt(issues: ValidationIssue[], path: string) {
  return issues.find((issue) => issue.path === path)?.message;
}

function MoveControls({
  label,
  index,
  count,
  onMove,
}: {
  label: string;
  index: number;
  count: number;
  onMove: (direction: -1 | 1) => void;
}) {
  return (
    <div className="move-controls">
      <button
        type="button"
        className="button--secondary button--compact"
        aria-label={`Move ${label} up`}
        disabled={index === 0}
        onClick={() => onMove(-1)}
      >
        ↑
      </button>
      <button
        type="button"
        className="button--secondary button--compact"
        aria-label={`Move ${label} down`}
        disabled={index === count - 1}
        onClick={() => onMove(1)}
      >
        ↓
      </button>
    </div>
  );
}

function LinksEditor({
  links,
  path,
  issues,
  canAdd,
  onChange,
}: {
  links: Link[];
  path: string;
  issues: ValidationIssue[];
  canAdd: boolean;
  onChange: (links: Link[]) => void;
}) {
  return (
    <div className="link-list">
      <h3>Links</h3>
      {links.map((link, index) => (
        <div className="link-row" key={link.id ?? index}>
          <Field
            label={`Link ${index + 1} label`}
            path={`${path}.${index}.label`}
            error={issueAt(issues, `${path}.${index}.label`)}
          >
            <input
              value={link.label}
              maxLength={DOCUMENT_LIMITS.fieldCharacters}
              onChange={(event) =>
                onChange(
                  links.map((item, i) =>
                    i === index ? { ...item, label: event.target.value } : item,
                  ),
                )
              }
            />
          </Field>
          <Field
            label={`Link ${index + 1} URL`}
            path={`${path}.${index}.url`}
            error={issueAt(issues, `${path}.${index}.url`)}
          >
            <input
              value={link.url}
              maxLength={DOCUMENT_LIMITS.fieldCharacters}
              spellCheck={false}
              onChange={(event) =>
                onChange(
                  links.map((item, i) =>
                    i === index ? { ...item, url: event.target.value } : item,
                  ),
                )
              }
            />
          </Field>
          {link.id ? (
            <MoveControls
              label={`link ${index + 1}`}
              index={index}
              count={links.length}
              onMove={(direction) => {
                const reordered = [...links];
                [reordered[index], reordered[index + direction]] = [
                  reordered[index + direction],
                  reordered[index],
                ];
                onChange(reordered);
              }}
            />
          ) : null}
          <button
            type="button"
            className="button--danger button--compact"
            onClick={() => onChange(links.filter((_, i) => i !== index))}
          >
            Remove link {index + 1}
          </button>
        </div>
      ))}
      <button
        type="button"
        className="button--quiet button--compact"
        disabled={!canAdd}
        onClick={() => onChange([...links, { label: "", url: "" }])}
      >
        Add link
      </button>
    </div>
  );
}

// Deliberately renders text only: stored URLs cannot navigate the privileged webview.
export function PublishedResume({
  document,
  pdf = false,
  onSelect,
  onSelectEntry,
  onAddEntry,
  canAddEntry = false,
}: {
  document: ResumeDocument;
  pdf?: boolean;
  onSelect?: (part: string) => void;
  onSelectEntry?: (sectionId: string, entryId: string) => void;
  onAddEntry?: (sectionId: string) => void;
  canAddEntry?: boolean;
}) {
  return (
    <article
      className="published-content"
      aria-label={
        onSelect
          ? "Live draft resume content"
          : pdf
            ? "PDF resume content"
            : "Published resume content"
      }
    >
      {pdf || onSelect ? null : <h2>{document.title}</h2>}
      {onSelect ? (
        <button
          type="button"
          className="reading-edit-target"
          onClick={() => onSelect("contact")}
        >
          {document.contact.fullName || "Add contact information"}
        </button>
      ) : (
        <p>{document.contact.fullName}</p>
      )}
      <p>
        {[
          document.contact.email,
          document.contact.phone,
          document.contact.location,
        ]
          .filter((value) => value.trim())
          .join(" · ")}
      </p>
      {document.contact.links
        .filter((link) => link.label.trim() || link.url.trim())
        .map((link, index) => (
          <p key={link.id ?? index}>
            {[link.label, link.url].filter((value) => value.trim()).join(": ")}
          </p>
        ))}
      {document.sections.map((section) => (
        <section key={section.id}>
          <h3>
            {onSelect ? (
              <button
                type="button"
                className="reading-edit-target"
                onClick={() => onSelect(section.id)}
              >
                {section.heading || "Untitled section"}
              </button>
            ) : (
              section.heading
            )}
          </h3>
          {section.entries.map((entry, entryIndex) => (
            <div key={entry.id}>
              {entry.heading.trim() ? (
                <h4>
                  {onSelectEntry ? (
                    <button
                      type="button"
                      className="reading-edit-target"
                      onClick={() => onSelectEntry(section.id, entry.id)}
                    >
                      {entry.heading}
                    </button>
                  ) : (
                    entry.heading
                  )}
                </h4>
              ) : onSelectEntry ? (
                <button
                  type="button"
                  className="reading-entry-action"
                  onClick={() => onSelectEntry(section.id, entry.id)}
                >
                  Edit untitled entry {entryIndex + 1}
                </button>
              ) : null}
              <p>
                {[entry.subheading, entry.dateRange, entry.location]
                  .filter((value) => value.trim())
                  .join(" · ")}
              </p>
              {entry.dates
                ?.filter((date) => dateText(date))
                .map((date) => (
                  <p key={date.id}>{dateText(date)}</p>
                ))}
              <dl>
                {entry.fields
                  .filter((field) => field.value.trim())
                  .map((field) => (
                    <div key={field.id}>
                      <dt>
                        {field.label}
                        {field.isSkill && !pdf && !onSelect ? " (skill)" : ""}
                      </dt>
                      <dd>{field.value}</dd>
                    </div>
                  ))}
              </dl>
              <ul>
                {entry.bullets
                  .filter((bullet) => bullet.text.trim())
                  .map((bullet) => (
                    <li key={bullet.id}>{bullet.text}</li>
                  ))}
              </ul>
              {entry.links
                .filter((link) => link.label.trim() || link.url.trim())
                .map((link, index) => (
                  <p key={link.id ?? index}>
                    {[link.label, link.url]
                      .filter((value) => value.trim())
                      .join(": ")}
                  </p>
                ))}
            </div>
          ))}
          {onAddEntry ? (
            <button
              type="button"
              className="reading-entry-action"
              disabled={!canAddEntry}
              onClick={() => onAddEntry(section.id)}
            >
              Add entry to {section.heading.trim() || "untitled section"}
            </button>
          ) : null}
        </section>
      ))}
    </article>
  );
}

function OverlayStatus() {
  const [health, setHealth] = useState<HealthState>({ kind: "checking" });
  useEffect(() => {
    void requestHealth().then((result) =>
      setHealth(
        result.ok
          ? { kind: "ready", health: result.value }
          : { kind: "error", message: result.error.messageKey },
      ),
    );
  }, []);

  return (
    <main className="shell shell--overlay">
      <header className="masthead">
        <div className="mark" aria-hidden="true">
          ORT
        </div>
        <div>
          <p className="eyebrow">Development profile</p>
          <h1>Application workspace</h1>
        </div>
      </header>
      <section className="status-card">
        <h2>Browser bridge remains gated</h2>
        <HealthBadge state={health} />
        <p className="description">
          The main window now owns resume editing. Browser capture stays
          disabled until M5 authenticated IPC and permission tests pass.
        </p>
      </section>
    </main>
  );
}

function HealthBadge({ state }: { state: HealthState }) {
  if (state.kind === "checking") {
    return (
      <p className="badge badge--pending" role="status">
        Checking
      </p>
    );
  }
  if (state.kind === "error") {
    return (
      <p className="badge badge--error" role="alert" title={state.message}>
        Unavailable
      </p>
    );
  }
  const ready = state.health.storageStatus === "ready";
  return (
    <p
      className={`badge ${ready ? "badge--ready" : "badge--error"}`}
      role="status"
    >
      {ready ? "Encrypted storage ready" : "Storage unavailable"}
    </p>
  );
}

function friendlyError(code: string): string {
  switch (code) {
    case "REVISION_CONFLICT":
      return "This draft changed after it was loaded. Reload before saving again.";
    case "INVALID_RESUME":
      return "The resume contains an invalid or oversized field. Review required headings and links.";
    case "STORAGE_UNAVAILABLE":
      return "Encrypted storage or the OS credential vault is unavailable.";
    case "COMMAND_UNAVAILABLE":
    case "INVALID_RESPONSE":
      return "The save result is uncertain. Reload the saved draft to check what reached storage before retrying.";
    default:
      return "The operation could not be completed safely. Try again.";
  }
}
