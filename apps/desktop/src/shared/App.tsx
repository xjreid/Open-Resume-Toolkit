import { useResumeSession } from "./use-resume-session";
import { ResumeCanvas } from "./ResumeCanvas";
import { PublishedResume } from "./PublishedResume";
import { HealthBadge, type HealthState } from "./HealthBadge";
import type { ContactDivider } from "./resume-view-types";
import { SettingsWorkspace } from "./SettingsWorkspace";
import { invokeDesktop as invoke } from "./desktop-client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ApplicationOverlay } from "./ApplicationOverlay";
import { TrackerWorkspace } from "./TrackerTableWorkspace";
import { AiWorkspace } from "./AiWorkspace";
import { AppShell, Brand, type WorkspaceDestination } from "./AppShell";
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
import type { DocumentStyle, ExportSource } from "@ort/contracts/export";
import {
  SELECTABLE_DOCUMENT_STYLES,
  DOCUMENT_STYLE_LABELS,
  DOCUMENT_STYLE_DESCRIPTIONS,
} from "./document-styles";
import { dateText } from "./resume-dates";
import { exportFeedback } from "./text-export";
import type {
  CalendarDate,
  Link,
  ResumeDocument,
  ResumeDate,
  ResumeEntry,
  ResumeSection,
} from "@ort/contracts/resume";
import { DOCUMENT_LIMITS, MAX_RESUME_DATES } from "@ort/contracts/resume";
import { ConfirmRemoval } from "./ConfirmRemoval";
import { duplicateEntry } from "./duplicate-entry";
import { DateEditor } from "./DateEditor";
import { CloseDialog } from "./CloseDialog";
import { BackupPanel } from "./BackupPanel";
import { StoragePanel } from "./StoragePanel";
import { ResumeStart } from "./ResumeStart";
import { DocumentImport } from "./DocumentImport";
import { ResumeSectionNavigator } from "./ResumeSectionNavigator";
import { UndoIcon, RedoIcon } from "./ResumeHistoryIcons";
import { createStartingSections } from "./starting-profiles";
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
  exportResumePdf,
  releaseResumePdf,
  renderResumePdf,
} from "./command-client";
import { pdfFailure } from "./pdf-preview";
import {
  createBullet,
  createEntityId,
  createEntry,
  createResumeDocument,
  createNamedField,
  moveItem,
  normalizeDocument,
  upgradeDocumentV2,
} from "./resume-editor";
import {
  linkSelection,
  toggleBoldSelection,
  toggleItalicSelection,
} from "./inline-formatting";

type Surface = "main" | "overlay";
type EntryTextField = "heading" | "subheading" | "dateRange" | "location";

export function App({ surface }: { surface: Surface }) {
  if (surface === "overlay") return <ApplicationOverlay />;
  return <ResumeEditor />;
}

function ResumeEditor() {
  const [destination, setDestination] =
    useState<WorkspaceDestination>("resume");
  const [overlayError, setOverlayError] = useState("");
  const [overlayVisible, setOverlayVisible] = useState(false);
  const [workflow, setWorkflow] = useState<"edit" | "view">("edit");
  const [importActive, setImportActive] = useState(false);
  const [importWorking, setImportWorking] = useState(false);
  const [trackerDirty, setTrackerDirty] = useState(false);
  const [confirmReload, setConfirmReload] = useState(false);
  const [documentStyle, setDocumentStyle] =
    useState<DocumentStyle>("technical");
  const [exportSource, setExportSource] = useState<ExportSource>("saved_draft");
  const [contactDivider, setContactDivider] = useState<ContactDivider>("dot");
  const [focusedPart, setFocusedPart] = useState<string | null>(null);
  const [validationFocus, setValidationFocus] =
    useState<ValidationFocusRequest | null>(null);
  const {
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
  } = useResumeSession({
    importActive,
    importWorking,
    trackerDirty,
    confirmReload,
    documentStyle,
    exportSource,
  });
  const focusPanel = useRef<HTMLDivElement>(null);
  const readingHeading = useRef<HTMLHeadingElement>(null);
  const focusRequested = useRef(false);
  function openEditor(part: string) {
    setWorkflow("edit");
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
  const firstContactField = useRef<HTMLInputElement>(null);
  const focusAfterStart = useRef(false);
  const { document, notice } = editor;
  const revision = editor.saved?.revision ?? null;
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
  const backupDirty =
    (dirty && (editor.saved !== null || editor.editEpoch > 0)) || trackerDirty;
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

  useEffect(() => {
    const window = getCurrentWebviewWindow();
    let active = true;
    void window
      .listen<boolean>("ort:overlay-visibility", (event) => {
        if (active) setOverlayVisible(event.payload);
      })
      .then((unlisten) => {
        if (!active) unlisten();
        else stop = unlisten;
      });
    let stop: (() => void) | undefined;
    void invoke("application_overlay_visibility").then((response) => {
      if (active && response.ok) setOverlayVisible(!!response.value);
    });
    return () => {
      active = false;
      stop?.();
    };
  }, []);

  function changeDocument(update: (current: ResumeDocument) => ResumeDocument) {
    dispatch({ type: "edit", update });
  }

  const storageReady =
    health.kind === "ready" && health.health.storageStatus === "ready";
  const alreadyPublished =
    editor.saved !== null &&
    editor.published !== null &&
    JSON.stringify(editor.saved.document) ===
      JSON.stringify(editor.published.document);

  return (
    <AppShell
      destination={destination}
      onNavigate={setDestination}
      onOpenApplication={() => {
        setOverlayError("");
        void invoke("toggle_application_overlay")
          .then((response) => {
            if (!response.ok) setOverlayError("Overlay could not be changed.");
            else setOverlayVisible(!!response.value);
          })
          .catch(() => setOverlayError("Overlay could not be changed."));
      }}
      overlayVisible={overlayVisible}
      navigationBlocked={busy || confirmReload || close.pending}
      status={
        <HealthBadge
          state={health}
          onRetry={() => {
            void invoke("retry_storage")
              .then(() => void loadWorkspace())
              .catch(() => void loadWorkspace());
          }}
        />
      }
    >
      {overlayError && <p role="alert">{overlayError}</p>}
      <CloseDialog
        open={close.pending}
        busy={editor.status !== "idle" || importWorking}
        resolving={close.resolving}
        canSave={
          !!document &&
          dirty &&
          !trackerDirty &&
          !importActive &&
          !close.overlayDirty &&
          !mustReload &&
          issues.length === 0
        }
        error={close.error}
        saveError={editor.errorCode ? friendlyError(editor.errorCode) : null}
        otherUnsavedWork={trackerDirty || importActive || close.overlayDirty}
        otherUnsavedWorkMessage={
          importActive
            ? "Keep editing to review and map your imported resume before quitting."
            : undefined
        }
        overlayUnsavedWork={close.overlayDirty}
        overlayCheckFailed={close.overlayCheckFailed}
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
      <nav
        className="workflow-steps"
        aria-label="Master resume mode"
        hidden={destination !== "resume" && destination !== "import"}
      >
        {(
          [
            ["edit", "Edit"],
            ["view", "View"],
          ] as const
        ).map(([step, label]) => (
          <button
            key={step}
            type="button"
            className="button--secondary"
            aria-current={
              destination === "resume" && workflow === step ? "step" : undefined
            }
            onClick={() => {
              setDestination("resume");
              setWorkflow(step);
              if (step === "edit") setExportSource("saved_draft");
              if (step !== "edit") setFocusedPart(null);
            }}
          >
            {label}
          </button>
        ))}
        <button
          type="button"
          className="button--secondary"
          aria-current={destination === "import" ? "step" : undefined}
          onClick={() => setDestination("import")}
        >
          Import
        </button>
        {destination !== "import" && document && !showStart && (
          <label className="workflow-style">
            <span>Resume style</span>
            <select
              value={documentStyle}
              disabled={busy || close.pending}
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
        )}
        {destination !== "import" && document && !showStart && (
          <button
            type="button"
            className="workflow-publish"
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
            Publish
          </button>
        )}
      </nav>
      <div className="resume-page" hidden={destination !== "resume"}>
        {showStart ? (
          <ResumeStart
            disabled={
              !storageReady ||
              busy ||
              mustReload ||
              confirmReload ||
              close.pending
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

        {confirmReload ? (
          <section className="notice" aria-label="Confirm reload">
            <p>
              Reloading discards unsaved edits in this window. Keep editing if
              you need to preserve them.
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
            {friendlyError(editor.errorCode)} We’ll keep your edits here until
            you try again.
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
              className={`document-workspace${workflow === "view" ? " document-workspace--view" : ""}`}
            >
              <ResumeSectionNavigator
                document={document}
                disabled={busy || mustReload || confirmReload || close.pending}
                hidden={workflow !== "edit"}
                currentSection={focusedPart}
                onChange={changeDocument}
              />
              <section
                className="resume-reading-panel"
                aria-labelledby="reading-title"
              >
                <header
                  className={`resume-display-header${workflow === "view" ? " resume-display-header--view" : ""}`}
                >
                  <div className="resume-display-header__title-row">
                    <div className="resume-display-header__title">
                      <h2 id="reading-title" ref={readingHeading} tabIndex={-1}>
                        {workflow === "edit" ? "Your resume" : "Version"}
                      </h2>
                      {workflow === "edit" ? (
                        <span className="resume-save-status" aria-live="polite">
                          <span
                            className={`resume-save-status__dot${!dirty && revision !== null && !editor.errorCode ? " resume-save-status__dot--saved" : ""}`}
                            aria-hidden="true"
                          />
                          {editor.status === "saving" || dirty
                            ? "Saving…"
                            : editor.errorCode
                              ? "Save interrupted"
                              : revision === null
                                ? "Not saved yet"
                                : "Saved"}
                        </span>
                      ) : null}
                    </div>
                    {workflow === "edit" ? (
                      <div
                        className="resume-history-actions"
                        aria-label="Editing history"
                      >
                        <button
                          type="button"
                          className="button--secondary button--compact"
                          aria-label="Undo edit"
                          title="Undo"
                          disabled={
                            !editor.undo.length ||
                            editor.undo.at(-1)?.schemaVersion !==
                              document.schemaVersion ||
                            editor.status === "loading" ||
                            close.pending
                          }
                          onClick={() => dispatch({ type: "undo" })}
                        >
                          <UndoIcon />
                        </button>
                        <button
                          type="button"
                          className="button--secondary button--compact"
                          aria-label="Redo edit"
                          title="Redo"
                          disabled={
                            !editor.redo.length ||
                            editor.status === "loading" ||
                            close.pending
                          }
                          onClick={() => dispatch({ type: "redo" })}
                        >
                          <RedoIcon />
                        </button>
                      </div>
                    ) : (
                      <div
                        className="resume-version-switch"
                        role="group"
                        aria-label="Resume version"
                      >
                        <button
                          type="button"
                          className="button--secondary button--compact"
                          aria-pressed={exportSource === "saved_draft"}
                          onClick={() => setExportSource("saved_draft")}
                        >
                          Saved resume
                        </button>
                        <button
                          type="button"
                          className="button--secondary button--compact"
                          aria-pressed={exportSource === "published_snapshot"}
                          disabled={!editor.published}
                          onClick={() => setExportSource("published_snapshot")}
                        >
                          Published resume
                        </button>
                      </div>
                    )}
                  </div>
                  {workflow === "view" ? (
                    <div
                      className="resume-display-header__export-actions"
                      aria-label="Export shown resume"
                    >
                      <span>Export shown version</span>
                      <button
                        type="button"
                        onClick={() => void exportSelected("pdf")}
                        disabled={
                          !storageReady ||
                          busy ||
                          issues.length > 0 ||
                          (exportSource === "saved_draft" &&
                            !editor.saved &&
                            !dirty) ||
                          (exportSource === "published_snapshot" &&
                            !editor.published)
                        }
                      >
                        Export PDF
                      </button>
                      <button
                        type="button"
                        className="button--secondary"
                        onClick={() => void exportSelected("docx")}
                        disabled={
                          !storageReady ||
                          busy ||
                          issues.length > 0 ||
                          (exportSource === "saved_draft" &&
                            !editor.saved &&
                            !dirty) ||
                          (exportSource === "published_snapshot" &&
                            !editor.published)
                        }
                      >
                        Export Word
                      </button>
                    </div>
                  ) : null}
                </header>
                {workflow === "edit" ? (
                  <ResumeCanvas
                    document={document}
                    style={documentStyle}
                    contactDivider={contactDivider}
                    onContactDividerChange={setContactDivider}
                    onFocusSection={setFocusedPart}
                    disabled={busy || mustReload}
                    canAddEntry={usage!.entries < DOCUMENT_LIMITS.entries}
                    onChange={changeDocument}
                  />
                ) : (
                  <PublishedResume
                    document={
                      exportSource === "published_snapshot" && editor.published
                        ? editor.published.document
                        : document
                    }
                    style={documentStyle}
                    contactDivider={contactDivider}
                  />
                )}
              </section>
            </div>
          </fieldset>
        ) : !document ? (
          <section className="status-card">
            <h2>Opening your workspace</h2>
            <p className="description">Getting your local resume ready.</p>
            {!busy ? (
              <button type="button" onClick={() => void loadWorkspace()}>
                Try again
              </button>
            ) : null}
          </section>
        ) : null}
      </div>
      <div className="import-page" hidden={destination !== "import"}>
        <section className="workspace-data" aria-label="Resume import">
          <DocumentImport
            style={documentStyle}
            reviewDisabled={!storageReady || mustReload || close.pending}
            disabledReason={
              dirty && !showStart
                ? "Save your current resume edits before importing."
                : undefined
            }
            disabled={
              !storageReady ||
              busy ||
              mustReload ||
              confirmReload ||
              close.pending ||
              (dirty && !showStart)
            }
            revision={revision}
            onReloadRequired={() =>
              dispatch({ type: "failed", code: "COMMAND_UNAVAILABLE" })
            }
            onReload={() => void loadWorkspace()}
            onBusyChange={setImportActive}
            onOperationChange={setImportWorking}
            onSaved={(saved) => {
              if (!editor.profileId) {
                void loadWorkspace();
                return;
              }
              dispatch({
                type: "loaded",
                workspace: {
                  profileId: editor.profileId,
                  draft: saved,
                  latestPublished: editor.published,
                },
                empty: saved.document,
              });
              setDestination("resume");
              setWorkflow("edit");
            }}
          />
        </section>
      </div>
      <div className="tracker-page" hidden={destination !== "tracker"}>
        <TrackerWorkspace
          active={destination === "tracker"}
          onDirtyChange={setTrackerDirty}
        />
      </div>
      <div className="settings-page" hidden={destination !== "settings"}>
        <SettingsWorkspace
          blocked={busy || confirmReload || close.pending}
          backup={
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
          }
          storage={
            <StoragePanel
              enabled={
                storageReady &&
                !busy &&
                !trackerDirty &&
                !mustReload &&
                !confirmReload &&
                !close.pending
              }
              onDeleteBegin={() => {
                if (
                  ioBusy.current ||
                  busy ||
                  trackerDirty ||
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
          }
        />
      </div>
      <div className="settings-page" hidden={destination !== "ai"}>
        <AiWorkspace
          blocked={!storageReady || busy || confirmReload || close.pending}
        />
      </div>
    </AppShell>
  );
}

function friendlyError(code: string): string {
  switch (code) {
    case "REVISION_CONFLICT":
      return "This draft changed after it was loaded. Reload before saving again.";
    case "INVALID_RESUME":
      return "The resume contains an invalid or oversized field. Review required headings and links.";
    case "STORAGE_UNAVAILABLE":
      return "Your local storage is unavailable. Please try again.";
    case "COMMAND_UNAVAILABLE":
    case "INVALID_RESPONSE":
      return "The save result is uncertain. Reload the saved draft to check what reached storage before retrying.";
    default:
      return "The operation could not be completed safely. Try again.";
  }
}
