import { SettingsWorkspace } from "./SettingsWorkspace";
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
  useLayoutEffect,
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
  createSection,
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
type ContactDivider = "dot" | "bar" | "dash";
const PARAGRAPH_FIELD_LABEL = "__ort_body_paragraph__";
type HealthState =
  | { kind: "checking" }
  | { kind: "ready"; health: HealthResponse }
  | { kind: "error"; message: string };

export function App({ surface }: { surface: Surface }) {
  if (surface === "overlay") return <OverlayStatus />;
  return <ResumeEditor />;
}

function UndoIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M9 7 4 11.5 9 16v-3h4.4c2.8 0 4.5 1.1 5.6 3.7-.1-5.2-2.6-7.7-7.2-7.7H9V7Z" />
    </svg>
  );
}

function RedoIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="m15 7 5 4.5-5 4.5v-3h-4.4C7.8 13 6.1 14.1 5 16.7 5.1 11.5 7.6 9 12.2 9H15V7Z" />
    </svg>
  );
}

function ResumeEditor() {
  const [destination, setDestination] =
    useState<WorkspaceDestination>("resume");
  const [workflow, setWorkflow] = useState<"edit" | "view">("edit");
  const [health, setHealth] = useState<HealthState>({ kind: "checking" });
  const [editor, dispatch] = useReducer(editorReducer, initialEditorState);
  const [importActive, setImportActive] = useState(false);
  const [importWorking, setImportWorking] = useState(false);
  const close = useCloseGuard(
    importWorking ? { ...editor, status: "exporting" } : editor,
  );
  const [confirmReload, setConfirmReload] = useState(false);
  const [documentStyle, setDocumentStyle] =
    useState<DocumentStyle>("technical");
  const [exportSource, setExportSource] = useState<ExportSource>("saved_draft");
  const [contactDivider, setContactDivider] = useState<ContactDivider>("dot");
  const [profileGeneration, setProfileGeneration] = useState(0);
  const [focusedPart, setFocusedPart] = useState<string | null>(null);
  const [validationFocus, setValidationFocus] =
    useState<ValidationFocusRequest | null>(null);
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
  const [suggestedSection, setSuggestedSection] =
    useState<string>("Custom Section");
  const [renamingSection, setRenamingSection] = useState<string | null>(null);
  const [draggingSection, setDraggingSection] = useState<string | null>(null);
  const [sectionDragOrder, setSectionDragOrder] = useState<string[] | null>(
    null,
  );
  const [trashActive, setTrashActive] = useState(false);
  const [pendingTrashSection, setPendingTrashSection] = useState<string | null>(
    null,
  );
  const sectionPointerDrag = useRef<{
    id: string;
    pointerId: number;
    startX: number;
    startY: number;
    dragging: boolean;
    offsetX: number;
    offsetY: number;
    width: number;
    height: number;
  } | null>(null);
  const sectionDragOrderRef = useRef<string[] | null>(null);
  const sectionLayoutBefore = useRef<Map<string, DOMRect>>(new Map());
  const sectionCards = useRef<Map<string, HTMLDivElement>>(new Map());
  const sectionList = useRef<HTMLDivElement>(null);
  const sectionTrash = useRef<HTMLDivElement>(null);
  const sectionNavigator = useRef<HTMLElement>(null);
  const [sectionDragPosition, setSectionDragPosition] = useState<{
    x: number;
    y: number;
    label: string;
  } | null>(null);
  useLayoutEffect(() => {
    if (!sectionDragOrder) return;
    for (const [id, previous] of sectionLayoutBefore.current) {
      const card = sectionCards.current.get(id);
      if (!card || id === draggingSection) continue;
      card.getAnimations().forEach((animation) => animation.cancel());
      const current = card.getBoundingClientRect();
      const delta = previous.top - current.top;
      if (Math.abs(delta) < 1) continue;
      card.animate(
        [
          { transform: `translateY(${delta}px)` },
          { transform: "translateY(0)" },
        ],
        { duration: 190, easing: "cubic-bezier(.2,.8,.2,1)" },
      );
    }
    sectionLayoutBefore.current.clear();
  }, [sectionDragOrder, draggingSection]);
  const firstContactField = useRef<HTMLInputElement>(null);
  const focusAfterStart = useRef(false);
  const ioBusy = useRef(false);
  const loadGeneration = useRef(0);
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
      empty: upgradeDocumentV2(createResumeDocument()),
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

  function captureSectionLayout() {
    sectionLayoutBefore.current = new Map(
      [...sectionCards.current].map(([id, card]) => [
        id,
        card.getBoundingClientRect(),
      ]),
    );
  }

  function updateSectionDragOrder(sourceId: string, clientY: number) {
    const list = sectionList.current;
    const currentOrder = sectionDragOrderRef.current;
    if (!list || !currentOrder) return;
    const otherIds = currentOrder.filter((id) => id !== sourceId);
    const localY = clientY - list.getBoundingClientRect().top;
    let destination = otherIds.length;
    for (let index = 0; index < otherIds.length; index += 1) {
      const card = sectionCards.current.get(otherIds[index]);
      if (!card) continue;
      const center = card.offsetTop - list.offsetTop + card.offsetHeight / 2;
      if (localY < center) {
        destination = index;
        break;
      }
    }
    const nextOrder = [...otherIds];
    nextOrder.splice(destination, 0, sourceId);
    if (nextOrder.every((id, index) => id === currentOrder[index])) return;
    captureSectionLayout();
    sectionDragOrderRef.current = nextOrder;
    setSectionDragOrder(nextOrder);
  }

  function commitSectionDragOrder(order: string[]) {
    changeDocument((current) => {
      const byId = new Map(
        current.sections.map((section) => [section.id, section]),
      );
      const sections = order
        .map((id) => byId.get(id))
        .filter((section): section is ResumeSection => Boolean(section))
        .map((section, sectionOrder) => ({
          ...section,
          order: sectionOrder,
        }));
      if (sections.length !== current.sections.length) return current;
      return { ...current, sections };
    });
  }

  function clearSectionDrag() {
    window.document.body.classList.remove("is-section-sorting");
    sectionPointerDrag.current = null;
    sectionDragOrderRef.current = null;
    setSectionDragOrder(null);
    setDraggingSection(null);
    setSectionDragPosition(null);
    setTrashActive(false);
  }

  useEffect(() => {
    if (!document) return;
    const activeDocument = document;
    function moveSectionPointer(event: PointerEvent) {
      const gesture = sectionPointerDrag.current;
      if (!gesture || gesture.pointerId !== event.pointerId) return;
      if (!gesture.dragging) {
        const distance = Math.hypot(
          event.clientX - gesture.startX,
          event.clientY - gesture.startY,
        );
        if (distance < 6) return;
        gesture.dragging = true;
        const order = activeDocument.sections.map((section) => section.id);
        sectionDragOrderRef.current = order;
        setSectionDragOrder(order);
        setDraggingSection(gesture.id);
        window.document.body.classList.add("is-section-sorting");
      }
      event.preventDefault();
      const navigatorBounds = sectionNavigator.current?.getBoundingClientRect();
      const listBounds = sectionList.current?.getBoundingClientRect();
      const trashBounds = sectionTrash.current?.getBoundingClientRect();
      if (!navigatorBounds || !listBounds || !trashBounds) return;
      const navigator = sectionNavigator.current;
      if (navigator) {
        if (event.clientY < navigatorBounds.top + 28) navigator.scrollTop -= 8;
        else if (event.clientY > navigatorBounds.bottom - 28)
          navigator.scrollTop += 8;
      }
      const minimumX = navigatorBounds.left + 8;
      const maximumX = Math.max(
        minimumX,
        navigatorBounds.right - gesture.width - 8,
      );
      const minimumY = Math.max(navigatorBounds.top + 8, listBounds.top);
      const maximumY = Math.max(
        minimumY,
        Math.min(navigatorBounds.bottom - 8, trashBounds.bottom) -
          gesture.height,
      );
      const boundedX = Math.min(
        maximumX,
        Math.max(minimumX, event.clientX - gesture.offsetX),
      );
      const boundedY = Math.min(
        maximumY,
        Math.max(minimumY, event.clientY - gesture.offsetY),
      );
      const draggedSection = activeDocument.sections.find(
        (section) => section.id === gesture.id,
      );
      setSectionDragPosition({
        x: boundedX,
        y: boundedY,
        label: draggedSection?.heading || "Untitled section",
      });
      const overTrash =
        event.clientX >= trashBounds.left &&
        event.clientX <= trashBounds.right &&
        event.clientY >= trashBounds.top &&
        event.clientY <= trashBounds.bottom;
      setTrashActive(overTrash);
      if (overTrash) return;
      updateSectionDragOrder(
        gesture.id,
        Math.min(listBounds.bottom, Math.max(listBounds.top, event.clientY)),
      );
    }

    function finishSectionPointer(event: PointerEvent) {
      const gesture = sectionPointerDrag.current;
      if (!gesture || gesture.pointerId !== event.pointerId) return;
      const trashBounds = sectionTrash.current?.getBoundingClientRect();
      const droppedOnTrash = Boolean(
        trashBounds &&
          event.clientX >= trashBounds.left &&
          event.clientX <= trashBounds.right &&
          event.clientY >= trashBounds.top &&
          event.clientY <= trashBounds.bottom,
      );
      if (gesture.dragging) {
        if (droppedOnTrash) setPendingTrashSection(gesture.id);
        else if (sectionDragOrderRef.current)
          commitSectionDragOrder(sectionDragOrderRef.current);
      } else {
        setRenamingSection(gesture.id);
      }
      clearSectionDrag();
    }

    function cancelSectionPointer(event: PointerEvent) {
      if (sectionPointerDrag.current?.pointerId === event.pointerId)
        clearSectionDrag();
    }

    function cancelSectionPointerOnBlur() {
      if (sectionPointerDrag.current) clearSectionDrag();
    }

    window.addEventListener("pointermove", moveSectionPointer, {
      passive: false,
    });
    window.addEventListener("pointerup", finishSectionPointer);
    window.addEventListener("pointercancel", cancelSectionPointer);
    window.addEventListener("blur", cancelSectionPointerOnBlur);
    return () => {
      window.document.body.classList.remove("is-section-sorting");
      window.removeEventListener("pointermove", moveSectionPointer);
      window.removeEventListener("pointerup", finishSectionPointer);
      window.removeEventListener("pointercancel", cancelSectionPointer);
      window.removeEventListener("blur", cancelSectionPointerOnBlur);
    };
  }, [document]);

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

  const storageReady =
    health.kind === "ready" && health.health.storageStatus === "ready";
  const alreadyPublished =
    editor.saved !== null &&
    editor.published !== null &&
    JSON.stringify(editor.saved.document) ===
      JSON.stringify(editor.published.document);
  const navigationSections = document
    ? (sectionDragOrder ?? document.sections.map((section) => section.id))
        .map((id) => document.sections.find((section) => section.id === id))
        .filter((section): section is ResumeSection => Boolean(section))
    : [];

  return (
    <AppShell
      destination={destination}
      onNavigate={setDestination}
      navigationBlocked={busy || confirmReload || close.pending}
      status={<HealthBadge state={health} />}
    >
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
      <div className="resume-page" hidden={destination !== "resume"}>
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

        <div hidden={showStart || !document}>
          <nav className="workflow-steps" aria-label="Resume mode">
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
                aria-current={workflow === step ? "step" : undefined}
                onClick={() => {
                  setWorkflow(step);
                  if (step === "edit") setExportSource("saved_draft");
                  if (step !== "edit") setFocusedPart(null);
                }}
              >
                {label}
              </button>
            ))}
            <label className="workflow-style">
              Resume style
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
          </nav>
        </div>
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
              <nav
                ref={sectionNavigator}
                className={`document-navigator${draggingSection ? " document-navigator--sorting" : ""}`}
                aria-label="Resume section navigation"
                hidden={workflow !== "edit"}
              >
                <div id="resume-navigation">
                  <div className="contact-nav-item">
                    <span>Contact</span>
                  </div>
                  <div className="section-sort-list" ref={sectionList}>
                    {navigationSections.map((section) => (
                      <div
                        className={`section-nav-card${draggingSection === section.id ? " section-nav-card--dragging" : ""}`}
                        key={section.id}
                        data-section-id={section.id}
                        ref={(card) => {
                          if (card) sectionCards.current.set(section.id, card);
                          else sectionCards.current.delete(section.id);
                        }}
                        onPointerDown={(event) => {
                          if (
                            event.button !== 0 ||
                            renamingSection === section.id ||
                            (event.target as HTMLElement).closest("input")
                          )
                            return;
                          event.preventDefault();
                          const bounds =
                            event.currentTarget.getBoundingClientRect();
                          sectionPointerDrag.current = {
                            id: section.id,
                            pointerId: event.pointerId,
                            startX: event.clientX,
                            startY: event.clientY,
                            dragging: false,
                            offsetX: event.clientX - bounds.left,
                            offsetY: event.clientY - bounds.top,
                            width: bounds.width,
                            height: bounds.height,
                          };
                        }}
                      >
                        <div className="section-nav-row">
                          {renamingSection === section.id ? (
                            <input
                              aria-label={`Section name ${section.heading || "untitled"}`}
                              autoFocus
                              value={section.heading}
                              onBlur={() => setRenamingSection(null)}
                              onKeyDown={(event) => {
                                if (
                                  event.key === "Enter" ||
                                  event.key === "Escape"
                                ) {
                                  setRenamingSection(null);
                                  event.currentTarget.blur();
                                }
                              }}
                              onChange={(event) =>
                                changeDocument((current) => ({
                                  ...current,
                                  sections: current.sections.map((item) =>
                                    item.id === section.id
                                      ? { ...item, heading: event.target.value }
                                      : item,
                                  ),
                                }))
                              }
                            />
                          ) : (
                            <span className="section-nav-title">
                              {section.heading || "Untitled section"}
                            </span>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                  {sectionDragPosition ? (
                    <div
                      className="section-drag-ghost"
                      style={{
                        left: sectionDragPosition.x,
                        top: sectionDragPosition.y,
                      }}
                      aria-hidden="true"
                    >
                      {sectionDragPosition.label}
                    </div>
                  ) : null}
                  <div
                    ref={sectionTrash}
                    className={`section-trash${trashActive ? " section-trash--active" : ""}`}
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M8 4h8l1 2h4v2H3V6h4l1-2Zm-2 6h12l-1 10H7L6 10Zm3 2v6h2v-6H9Zm4 0v6h2v-6h-2Z" />
                    </svg>
                    <span>Drag a section here to delete</span>
                  </div>
                  {pendingTrashSection ? (
                    <div
                      className="section-trash-confirmation"
                      role="group"
                      aria-label="Confirm section deletion"
                    >
                      <strong>Delete this section?</strong>
                      <p>
                        {document.sections.find(
                          (section) => section.id === pendingTrashSection,
                        )?.heading || "This section"}{" "}
                        and all of its items will be removed. You can undo this
                        change afterward.
                      </p>
                      <div>
                        <button
                          type="button"
                          className="button--secondary button--compact"
                          onClick={() => setPendingTrashSection(null)}
                        >
                          Cancel
                        </button>
                        <button
                          type="button"
                          className="button--danger button--compact"
                          onClick={() => {
                            const sectionId = pendingTrashSection;
                            setPendingTrashSection(null);
                            changeDocument((current) => ({
                              ...current,
                              sections: current.sections
                                .filter((section) => section.id !== sectionId)
                                .map((section, order) => ({
                                  ...section,
                                  order,
                                })),
                            }));
                          }}
                        >
                          Delete section
                        </button>
                      </div>
                    </div>
                  ) : null}
                  <div className="section-add-control">
                    <label>
                      Add section
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
                      type="button"
                      className="button--secondary button--compact"
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
                      }}
                    >
                      Add
                    </button>
                  </div>
                </div>
              </nav>
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
        <section className="workspace-data" aria-labelledby="import-page-title">
          <p className="eyebrow">Separate workspace</p>
          <h2 id="import-page-title">Import a resume</h2>
          <p>
            Bring in an existing document here, then return to the resume
            workspace to edit it directly on the page.
          </p>
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
                workspace: { draft: saved, latestPublished: editor.published },
                empty: saved.document,
              });
              setDestination("resume");
              setWorkflow("edit");
            }}
          />
        </section>
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
          }
        />
      </div>
    </AppShell>
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
        + Add entry
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
      <details className="entry-options">
        <summary>Entry options</summary>
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
      </details>
      <div className="field-grid">
        {(
          [
            ["heading", guidance.heading],
            ["subheading", guidance.subheading],
            ["dateRange", "Date range"],
            ["location", "Location"],
          ] as const
        )
          .filter(
            ([field]) =>
              field !== "dateRange" ||
              entry.dates === undefined ||
              entry.dateRange.trim(),
          )
          .map(([field, label]) => (
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
          Details and skills
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
          + Add a custom detail
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
      <span id={`${id}-label`}>{label}</span>
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
      "aria-labelledby"?: string;
      "aria-describedby"?: string;
    }>(children)
  )
    return children;
  return cloneElement(children, {
    id,
    "aria-labelledby": `${id}-label`,
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
      <h3>Links (optional)</h3>
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
        + Add link
      </button>
    </div>
  );
}

function ResumeCanvas({
  document,
  style,
  contactDivider,
  onContactDividerChange,
  disabled,
  canAddEntry,
  onChange,
}: {
  document: ResumeDocument;
  style: DocumentStyle;
  contactDivider: ContactDivider;
  onContactDividerChange: (divider: ContactDivider) => void;
  disabled: boolean;
  canAddEntry: boolean;
  onChange: (update: (current: ResumeDocument) => ResumeDocument) => void;
}) {
  function changeContact(
    field: keyof Omit<ResumeDocument["contact"], "links">,
    value: string,
  ) {
    onChange((current) => ({
      ...current,
      contact: { ...current.contact, [field]: value },
    }));
  }
  function changeEntry(
    sectionId: string,
    entryId: string,
    update: (entry: ResumeEntry) => ResumeEntry,
  ) {
    onChange((current) => ({
      ...current,
      sections: current.sections.map((section) =>
        section.id !== sectionId
          ? section
          : {
              ...section,
              entries: section.entries.map((entry) =>
                entry.id === entryId ? update(entry) : entry,
              ),
            },
      ),
    }));
  }
  return (
    <div
      className={`resume-canvas resume-canvas--${style}`}
      aria-label="Editable resume"
    >
      <p className="resume-canvas__hint">
        Select a labeled area to add or change information. The labels are
        suggestions—you can use each area however it suits your resume.
      </p>
      <header className="resume-canvas__contact">
        <CanvasField
          label="Name"
          value={document.contact.fullName}
          bold
          disabled={disabled}
          onChange={(value) => changeContact("fullName", value)}
        />
        <ContactInformationEditor
          contact={document.contact}
          divider={contactDivider}
          disabled={disabled}
          onDividerChange={onContactDividerChange}
          onChange={(contact) =>
            onChange((current) => ({ ...current, contact }))
          }
        />
      </header>
      {document.sections.map((section) => (
        <section className="resume-canvas__section" key={section.id}>
          <h3>{section.heading || "Untitled section"}</h3>
          {section.entries.map((entry) => (
            <CanvasEntry
              key={entry.id}
              entry={entry}
              disabled={disabled}
              onChange={(next) => changeEntry(section.id, entry.id, () => next)}
              onRemove={() =>
                onChange((current) => ({
                  ...current,
                  sections: current.sections.map((candidate) =>
                    candidate.id === section.id
                      ? {
                          ...candidate,
                          entries: candidate.entries.filter(
                            (item) => item.id !== entry.id,
                          ),
                        }
                      : candidate,
                  ),
                }))
              }
            />
          ))}
          <button
            type="button"
            className="canvas-add-entry"
            disabled={disabled || !canAddEntry}
            onClick={() =>
              onChange((current) => ({
                ...current,
                sections: current.sections.map((candidate) =>
                  candidate.id === section.id
                    ? {
                        ...candidate,
                        entries: [
                          ...candidate.entries,
                          createEntry(candidate.entries.length),
                        ],
                      }
                    : candidate,
                ),
              }))
            }
          >
            + Add another item
          </button>
        </section>
      ))}
    </div>
  );
}

function ContactInformationEditor({
  contact,
  divider,
  disabled,
  onChange,
  onDividerChange,
}: {
  contact: ResumeDocument["contact"];
  divider: ContactDivider;
  disabled: boolean;
  onChange: (contact: ResumeDocument["contact"]) => void;
  onDividerChange: (divider: ContactDivider) => void;
}) {
  const initialItems = [
    contact.email,
    contact.phone,
    ...contact.location.split("\n"),
  ].filter((value) => value.length > 0);
  const [items, setItems] = useState<string[]>(() =>
    initialItems.length ? initialItems : [""],
  );
  const [activeIndex, setActiveIndex] = useState(0);
  const [selection, setSelection] = useState({ start: 0, end: 0 });
  const [typingFormat, setTypingFormat] = useState({
    bold: false,
    italic: false,
  });
  const [linkOpen, setLinkOpen] = useState(false);
  const [linkUrl, setLinkUrl] = useState("");
  const [linkRange, setLinkRange] = useState({ start: 0, end: 0 });
  const inputs = useRef<Array<HTMLInputElement | null>>([]);
  const activeValue = items[activeIndex] ?? "";
  const linkIsValid = safeInlineHref(linkUrl);

  useEffect(() => {
    const currentContact = {
      email: items[0] ?? "",
      phone: items[1] ?? "",
      location: items.slice(2).join("\n"),
    };
    if (
      currentContact.email === contact.email &&
      currentContact.phone === contact.phone &&
      currentContact.location === contact.location
    )
      return;
    const externalItems = [
      contact.email,
      contact.phone,
      ...contact.location.split("\n"),
    ].filter((value) => value.length > 0);
    setItems(externalItems.length ? externalItems : [""]);
  }, [contact.email, contact.phone, contact.location]);

  function commit(nextItems: string[]) {
    setItems(nextItems);
    onChange({
      ...contact,
      email: nextItems[0] ?? "",
      phone: nextItems[1] ?? "",
      location: nextItems.slice(2).join("\n"),
    });
  }
  function updateItem(index: number, value: string) {
    const next = [...items];
    next[index] = value;
    commit(next);
  }
  function restoreSelection(index: number, start: number, end: number) {
    window.requestAnimationFrame(() => {
      inputs.current[index]?.focus();
      inputs.current[index]?.setSelectionRange(start, end);
      setActiveIndex(index);
      setSelection({ start, end });
    });
  }
  function applyFormat(kind: "bold" | "italic") {
    const marker = kind === "bold" ? "**" : "*";
    const toggle =
      kind === "bold" ? toggleBoldSelection : toggleItalicSelection;
    if (selection.end > selection.start) {
      const result = toggle(activeValue, selection.start, selection.end);
      if (!result) return;
      updateItem(activeIndex, result.value);
      restoreSelection(activeIndex, result.selectionStart, result.selectionEnd);
      return;
    }
    const active = typingFormat[kind];
    const caret = selection.start;
    if (active && activeValue.slice(caret, caret + marker.length) === marker) {
      restoreSelection(
        activeIndex,
        caret + marker.length,
        caret + marker.length,
      );
      setTypingFormat((current) => ({ ...current, [kind]: false }));
      return;
    }
    const nextValue = active
      ? activeValue.slice(0, caret) + marker + activeValue.slice(caret)
      : activeValue.slice(0, caret) +
        marker +
        marker +
        activeValue.slice(caret);
    updateItem(activeIndex, nextValue);
    restoreSelection(activeIndex, caret + marker.length, caret + marker.length);
    setTypingFormat((current) => ({ ...current, [kind]: !active }));
  }
  function applyLink() {
    const result = linkSelection(
      activeValue,
      linkRange.start,
      linkRange.end,
      linkUrl,
    );
    if (!result) return;
    updateItem(activeIndex, result.value);
    setLinkOpen(false);
    setLinkUrl("");
    restoreSelection(activeIndex, result.selectionStart, result.selectionEnd);
  }

  return (
    <section className="contact-information-editor">
      <div className="contact-information-editor__toolbar">
        <span>Contact information</span>
        <div className="canvas-field__formatting">
          <button
            type="button"
            className="button--quiet button--compact"
            aria-label="Bold contact text"
            aria-pressed={typingFormat.bold}
            disabled={disabled}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => applyFormat("bold")}
          >
            <strong aria-hidden="true">B</strong>
          </button>
          <button
            type="button"
            className="button--quiet button--compact canvas-format-italic"
            aria-label="Italicize contact text"
            aria-pressed={typingFormat.italic}
            disabled={disabled}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => applyFormat("italic")}
          >
            <em aria-hidden="true">I</em>
          </button>
          <button
            type="button"
            className="button--quiet button--compact"
            aria-expanded={linkOpen}
            disabled={disabled}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => {
              setLinkRange(selection);
              setLinkOpen((open) => !open);
            }}
          >
            Link
          </button>
        </div>
      </div>
      {linkOpen ? (
        <div className="contact-information-editor__link">
          <span>
            Link text:{" "}
            {linkRange.end > linkRange.start
              ? activeValue.slice(linkRange.start, linkRange.end)
              : "Enter Text Here"}
          </span>
          <input
            autoFocus
            type="url"
            aria-label="Contact link address"
            value={linkUrl}
            placeholder="https://example.com"
            onChange={(event) => setLinkUrl(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && linkIsValid) {
                event.preventDefault();
                applyLink();
              }
            }}
          />
          <button type="button" disabled={!linkIsValid} onClick={applyLink}>
            Apply
          </button>
        </div>
      ) : null}
      <div className="contact-information-editor__items">
        {items.map((value, index) => (
          <div className="contact-information-editor__item" key={index}>
            <input
              ref={(node) => {
                inputs.current[index] = node;
              }}
              aria-label={`Contact information ${index + 1}`}
              value={value}
              disabled={disabled}
              className={`${activeIndex === index && typingFormat.bold ? "is-typing-bold" : ""}${activeIndex === index && typingFormat.italic ? " is-typing-italic" : ""}`}
              placeholder="Email, phone, location, portfolio…"
              onFocus={(event) => {
                setActiveIndex(index);
                setSelection({
                  start: event.currentTarget.selectionStart ?? 0,
                  end: event.currentTarget.selectionEnd ?? 0,
                });
                setTypingFormat({ bold: false, italic: false });
              }}
              onSelect={(event) => {
                setActiveIndex(index);
                setSelection({
                  start: event.currentTarget.selectionStart ?? 0,
                  end: event.currentTarget.selectionEnd ?? 0,
                });
              }}
              onChange={(event) => updateItem(index, event.target.value)}
            />
            <button
              type="button"
              className="contact-information-editor__delete button--quiet"
              aria-label={`Delete contact information ${index + 1}`}
              title="Delete contact"
              disabled={disabled}
              onClick={() => {
                const next = items.filter(
                  (_, itemIndex) => itemIndex !== index,
                );
                commit(next.length ? next : [""]);
                setActiveIndex(Math.max(0, index - 1));
              }}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <button
        type="button"
        className="contact-information-editor__add button--secondary button--compact"
        disabled={disabled}
        onClick={() => {
          commit([...items, ""]);
          setActiveIndex(items.length);
          window.requestAnimationFrame(() =>
            inputs.current[items.length]?.focus(),
          );
        }}
      >
        + Add contact information
      </button>
      <label className="contact-divider-control">
        Contact divider
        <select
          value={divider}
          disabled={disabled}
          onChange={(event) =>
            onDividerChange(
              event.target.value === "bar"
                ? "bar"
                : event.target.value === "dash"
                  ? "dash"
                  : "dot",
            )
          }
        >
          <option value="dot">Dot •</option>
          <option value="bar">Bar |</option>
          <option value="dash">Dash -</option>
        </select>
      </label>
    </section>
  );
}

function CanvasEntry({
  entry,
  disabled,
  onChange,
  onRemove,
}: {
  entry: ResumeEntry;
  disabled: boolean;
  onChange: (entry: ResumeEntry) => void;
  onRemove: () => void;
}) {
  const visibleFields = entry.fields.filter(
    (field) => field.label !== PARAGRAPH_FIELD_LABEL,
  );
  const paragraphField = entry.fields.find(
    (field) => field.label === PARAGRAPH_FIELD_LABEL,
  );
  const extra = visibleFields.find(
    (field) => field.label.trim().toLowerCase() === "extra",
  );
  const details = visibleFields.find((field) => field.id !== extra?.id);
  function updateVisibleField(
    field: (typeof visibleFields)[number] | undefined,
    slot: 0 | 1,
    label: string,
    value: string,
  ) {
    let fields: ResumeEntry["fields"];
    if (field) {
      fields = entry.fields.map((candidate) =>
        candidate.id === field.id ? { ...candidate, value } : candidate,
      );
    } else {
      const ordinary = [details, extra].map(
        (candidate, index) =>
          candidate ?? {
            ...createNamedField(index),
            label: index === 0 ? "Details" : "Extra",
          },
      );
      ordinary[slot] = { ...ordinary[slot], label, value };
      fields = paragraphField ? [...ordinary, paragraphField] : ordinary;
    }
    onChange({
      ...entry,
      fields: fields.map((candidate, order) => ({ ...candidate, order })),
    });
  }
  function setBodyMode(mode: "bullets" | "paragraph") {
    if (mode === "paragraph") {
      const value = entry.bullets.map((bullet) => bullet.text).join("\n");
      const nextParagraph = paragraphField
        ? { ...paragraphField, value }
        : {
            ...createNamedField(entry.fields.length),
            label: PARAGRAPH_FIELD_LABEL,
            value,
          };
      const fields = [
        ...entry.fields.filter(
          (field) => field.label !== PARAGRAPH_FIELD_LABEL,
        ),
        nextParagraph,
      ].map((field, order) => ({ ...field, order }));
      onChange({ ...entry, fields, bullets: [] });
      return;
    }
    const value = paragraphField?.value ?? "";
    onChange({
      ...entry,
      fields: entry.fields
        .filter((field) => field.label !== PARAGRAPH_FIELD_LABEL)
        .map((field, order) => ({ ...field, order })),
      bullets: value.split("\n").map((text, order) => ({
        ...createBullet(order),
        text,
      })),
    });
  }
  return (
    <article className="canvas-entry">
      <ConfirmRemoval
        label="Remove item"
        description="Remove this item and all of its information?"
        onRemove={onRemove}
      />
      <div className="canvas-entry__primary">
        <div className="canvas-entry__title-line">
          <CanvasField
            label="Title"
            value={entry.heading}
            bold
            disabled={disabled}
            onChange={(value) => onChange({ ...entry, heading: value })}
          />
          <span className="entry-title-separator" aria-hidden="true">
            |
          </span>
          <CanvasField
            label="Skills / details"
            value={details?.value ?? ""}
            disabled={disabled}
            onChange={(value) =>
              updateVisibleField(details, 0, "Details", value)
            }
          />
        </div>
        <CanvasField
          label="Role"
          value={entry.subheading}
          multiline
          disabled={disabled}
          onChange={(value) => onChange({ ...entry, subheading: value })}
        />
      </div>
      <div className="canvas-entry__dates">
        <CanvasField
          label="Location"
          value={entry.location}
          disabled={disabled}
          onChange={(value) => onChange({ ...entry, location: value })}
        />
        <CanvasDateField
          entry={entry}
          disabled={disabled}
          onChange={onChange}
        />
        <CanvasField
          label="Extra"
          value={extra?.value ?? ""}
          disabled={disabled}
          onChange={(value) => updateVisibleField(extra, 1, "Extra", value)}
        />
      </div>
      <CanvasField
        label="Information"
        value={
          paragraphField?.value ??
          entry.bullets.map((bullet) => bullet.text).join("\n")
        }
        multiline
        bulk
        bulkMode={paragraphField ? "paragraph" : "bullets"}
        onBulkModeChange={setBodyMode}
        disabled={disabled}
        onChange={(value) => {
          if (paragraphField) {
            onChange({
              ...entry,
              fields: entry.fields.map((field) =>
                field.id === paragraphField.id ? { ...field, value } : field,
              ),
            });
            return;
          }
          onChange({
            ...entry,
            bullets: value.split("\n").map((text, index) => ({
              ...(entry.bullets[index] ?? createBullet(index)),
              text,
              order: index,
            })),
          });
        }}
      />
    </article>
  );
}

const DATE_MONTHS = [
  "Jan.",
  "Feb.",
  "Mar.",
  "Apr.",
  "May.",
  "Jun.",
  "Jul.",
  "Aug.",
  "Sep.",
  "Oct.",
  "Nov.",
  "Dec.",
];

function CanvasDateField({
  entry,
  disabled,
  onChange,
}: {
  entry: ResumeEntry;
  disabled: boolean;
  onChange: (entry: ResumeEntry) => void;
}) {
  const [editing, setEditing] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const date: ResumeDate = entry.dates?.[0] ?? {
    id: createEntityId(),
    order: 0,
    label: "",
    start: null,
    end: null,
  };
  const display = entry.dateRange.trim() || dateText(date);
  useEffect(() => {
    if (!editing) return;
    function closeOnOutsideClick(event: PointerEvent) {
      if (!container.current?.contains(event.target as Node)) setEditing(false);
    }
    window.addEventListener("pointerdown", closeOnOutsideClick);
    return () => window.removeEventListener("pointerdown", closeOnOutsideClick);
  }, [editing]);
  function updateDate(next: ResumeDate) {
    onChange({ ...entry, dateRange: "", dates: [next] });
  }
  function updateStart(start: CalendarDate | null) {
    updateDate({ ...date, start });
  }
  function updateEnd(end: ResumeDate["end"]) {
    updateDate({ ...date, end });
  }
  return (
    <div
      ref={container}
      className={`canvas-field canvas-date-field${editing ? " canvas-field--editing" : ""}`}
    >
      {editing ? (
        <div className="canvas-date-editor">
          <div className="canvas-date-editor__header">
            <strong>Date</strong>
            <button
              type="button"
              className="button--quiet button--compact"
              aria-label="Clear date"
              title="Clear date"
              onClick={() => onChange({ ...entry, dateRange: "", dates: [] })}
            >
              ×
            </button>
          </div>
          {entry.dateRange.trim() ? (
            <p className="canvas-date-editor__legacy">
              Choosing a date below replaces “{entry.dateRange}”.
            </p>
          ) : null}
          <CompactCalendarFields
            label="Start"
            value={date.start}
            onChange={updateStart}
          />
          <label className="canvas-date-editor__end-mode">
            End
            <select
              value={date.end?.kind ?? "none"}
              onChange={(event) => {
                if (event.target.value === "present") {
                  updateEnd({ kind: "present" });
                  return;
                }
                if (event.target.value === "date") {
                  updateEnd({
                    kind: "date",
                    value: {
                      year: new Date().getFullYear(),
                      month: null,
                      expected: false,
                    },
                  });
                  return;
                }
                updateEnd(null);
              }}
            >
              <option value="none">No end date</option>
              <option value="date">Choose end date</option>
              <option value="present">Present (ongoing)</option>
            </select>
          </label>
          {date.end?.kind === "date" ? (
            <CompactCalendarFields
              label="End"
              value={date.end.value}
              onChange={(value) =>
                updateEnd(value ? { kind: "date", value } : null)
              }
            />
          ) : null}
          <p className="canvas-date-editor__preview">
            {dateText(date) || "Select a start date"}
          </p>
        </div>
      ) : (
        <button
          type="button"
          className="canvas-field__button"
          disabled={disabled}
          onClick={() => setEditing(true)}
        >
          <span className="canvas-field__label">Date</span>
          <span className="canvas-field__value">{display || "Add date"}</span>
        </button>
      )}
    </div>
  );
}

function CompactCalendarFields({
  label,
  value,
  onChange,
}: {
  label: string;
  value: CalendarDate | null;
  onChange: (value: CalendarDate | null) => void;
}) {
  return (
    <fieldset className="compact-calendar-fields">
      <legend>{label}</legend>
      <select
        aria-label={`${label} month`}
        value={value?.month ?? ""}
        disabled={!value}
        onChange={(event) => {
          if (!value) return;
          onChange({
            ...value,
            month: event.target.value ? Number(event.target.value) : null,
          });
        }}
      >
        <option value="">Year only</option>
        {DATE_MONTHS.map((month, index) => (
          <option key={month} value={index + 1}>
            {month}
          </option>
        ))}
      </select>
      <input
        type="number"
        min={1}
        max={9999}
        step={1}
        aria-label={`${label} year`}
        placeholder="Year"
        value={value?.year || ""}
        onChange={(event) => {
          const year = Number(event.target.value);
          onChange(
            event.target.value
              ? {
                  year,
                  month: value?.month ?? null,
                  expected: false,
                }
              : null,
          );
        }}
      />
    </fieldset>
  );
}

function CanvasField({
  label,
  value,
  onChange,
  disabled,
  multiline = false,
  bulk = false,
  bulkMode = "bullets",
  onBulkModeChange,
  bold = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  disabled: boolean;
  multiline?: boolean;
  bulk?: boolean;
  bulkMode?: "bullets" | "paragraph";
  onBulkModeChange?: (mode: "bullets" | "paragraph") => void;
  bold?: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [linkUrl, setLinkUrl] = useState("");
  const [linkOpen, setLinkOpen] = useState(false);
  const [selection, setSelection] = useState({ start: 0, end: 0 });
  const [linkSelectionRange, setLinkSelectionRange] = useState<{
    start: number;
    end: number;
  } | null>(null);
  const [typingFormat, setTypingFormat] = useState({
    bold: false,
    italic: false,
  });
  const editor = useRef<HTMLInputElement | HTMLTextAreaElement>(null);
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!editing) return;
    function closeOnOutsideClick(event: PointerEvent) {
      if (!container.current?.contains(event.target as Node)) {
        setEditing(false);
        setLinkOpen(false);
        setTypingFormat({ bold: false, italic: false });
      }
    }
    window.addEventListener("pointerdown", closeOnOutsideClick);
    return () => window.removeEventListener("pointerdown", closeOnOutsideClick);
  }, [editing]);
  const hasSelection = selection.end > selection.start;
  const bullets = bulk && bulkMode === "bullets";
  const selectedText = value.slice(selection.start, selection.end);
  const selectionIsBold =
    (selectedText.startsWith("**") && selectedText.endsWith("**")) ||
    (selection.start >= 2 &&
      value.slice(selection.start - 2, selection.start) === "**" &&
      value.slice(selection.end, selection.end + 2) === "**");
  const selectionIsItalic =
    (selectedText.startsWith("*") &&
      selectedText.endsWith("*") &&
      !selectedText.startsWith("**")) ||
    (selection.start >= 1 &&
      value.slice(selection.start - 1, selection.start) === "*" &&
      value.slice(selection.end, selection.end + 1) === "*" &&
      value.slice(selection.start - 2, selection.start) !== "**");
  const boldPressed = hasSelection ? selectionIsBold : typingFormat.bold;
  const italicPressed = hasSelection ? selectionIsItalic : typingFormat.italic;
  const linkIsValid = (() => {
    try {
      return ["http:", "https:", "mailto:"].includes(new URL(linkUrl).protocol);
    } catch {
      return false;
    }
  })();
  function restoreSelection(start: number, end: number) {
    window.requestAnimationFrame(() => {
      editor.current?.focus();
      editor.current?.setSelectionRange(start, end);
      setSelection({ start, end });
    });
  }
  function toggleTypingFormat(kind: "bold" | "italic") {
    const marker = kind === "bold" ? "**" : "*";
    const active = typingFormat[kind];
    const caret = selection.start;
    if (active) {
      if (value.slice(caret, caret + marker.length) === marker) {
        restoreSelection(caret + marker.length, caret + marker.length);
      } else {
        onChange(value.slice(0, caret) + marker + value.slice(caret));
        restoreSelection(caret + marker.length, caret + marker.length);
      }
      setTypingFormat((current) => ({ ...current, [kind]: false }));
      return;
    }
    onChange(
      value.slice(0, caret) + marker + marker + value.slice(selection.end),
    );
    restoreSelection(caret + marker.length, caret + marker.length);
    setTypingFormat((current) => ({ ...current, [kind]: true }));
  }
  function decorate(kind: "bold" | "italic" | "link") {
    const { start, end } =
      kind === "link" && linkSelectionRange ? linkSelectionRange : selection;
    if (kind === "bold") {
      if (end === start) {
        toggleTypingFormat("bold");
        return;
      }
      const result = toggleBoldSelection(value, start, end);
      if (!result) return;
      onChange(result.value);
      restoreSelection(result.selectionStart, result.selectionEnd);
      setTypingFormat((current) => ({ ...current, bold: false }));
      return;
    }
    if (kind === "italic") {
      if (end === start) {
        toggleTypingFormat("italic");
        return;
      }
      const result = toggleItalicSelection(value, start, end);
      if (!result) return;
      onChange(result.value);
      restoreSelection(result.selectionStart, result.selectionEnd);
      setTypingFormat((current) => ({ ...current, italic: false }));
      return;
    }
    const result = linkSelection(value, start, end, linkUrl);
    if (!result) return;
    onChange(result.value);
    restoreSelection(result.selectionStart, result.selectionEnd);
    setLinkUrl("");
    setLinkOpen(false);
    setLinkSelectionRange(null);
  }
  function rememberSelection() {
    const target = editor.current;
    setSelection({
      start: target?.selectionStart ?? 0,
      end: target?.selectionEnd ?? 0,
    });
  }
  return (
    <div
      ref={container}
      className={`canvas-field${editing ? " canvas-field--editing" : ""}${multiline ? " canvas-field--multiline" : " canvas-field--singleline"}${bold ? " canvas-field--bold" : ""}${bullets ? " canvas-field--bullets" : ""}`}
    >
      {editing ? (
        <>
          <div className="canvas-field__toolbar">
            <span>{label}</span>
            <span className="canvas-field__formatting">
              <button
                type="button"
                className="button--quiet button--compact canvas-format-clear"
                aria-label={`Clear ${label.toLowerCase()}`}
                title="Clear all text"
                disabled={!value}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  onChange("");
                  setTypingFormat({ bold: false, italic: false });
                  setLinkOpen(false);
                  setLinkSelectionRange(null);
                  restoreSelection(0, 0);
                }}
              >
                <span aria-hidden="true">×</span>
              </button>
              <button
                type="button"
                className="button--quiet button--compact"
                aria-label={boldPressed ? "Turn bold off" : "Turn bold on"}
                title={boldPressed ? "Turn bold off" : "Bold"}
                aria-pressed={boldPressed}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => decorate("bold")}
              >
                <strong aria-hidden="true">B</strong>
              </button>
              <button
                type="button"
                className="button--quiet button--compact canvas-format-italic"
                aria-label={
                  italicPressed ? "Turn italics off" : "Turn italics on"
                }
                title={italicPressed ? "Turn italics off" : "Italic"}
                aria-pressed={italicPressed}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => decorate("italic")}
              >
                <em aria-hidden="true">I</em>
              </button>
              <button
                type="button"
                className="button--quiet button--compact"
                onMouseDown={(event) => event.preventDefault()}
                aria-expanded={linkOpen}
                onClick={() => {
                  if (!linkOpen) setLinkSelectionRange(selection);
                  else setLinkSelectionRange(null);
                  setLinkOpen((open) => !open);
                }}
              >
                Link
              </button>
            </span>
            {bulk ? (
              <button
                type="button"
                className="button--quiet button--compact"
                onClick={() =>
                  onBulkModeChange?.(bullets ? "paragraph" : "bullets")
                }
              >
                {bullets ? "Paragraph" : "Bullet points"}
              </button>
            ) : null}
          </div>
          {linkOpen ? (
            <div className="canvas-field__link-popover">
              <span className="canvas-field__link-selection">
                Link text:{" "}
                <mark>
                  {linkSelectionRange &&
                  linkSelectionRange.end > linkSelectionRange.start
                    ? value.slice(
                        linkSelectionRange.start,
                        linkSelectionRange.end,
                      )
                    : "Enter Text Here"}
                </mark>
              </span>
              <input
                autoFocus
                type="url"
                aria-label="Link address"
                value={linkUrl}
                placeholder="https://example.com"
                onFocus={() => {
                  if (linkSelectionRange)
                    editor.current?.setSelectionRange(
                      linkSelectionRange.start,
                      linkSelectionRange.end,
                    );
                }}
                onChange={(event) => setLinkUrl(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && linkIsValid) {
                    event.preventDefault();
                    decorate("link");
                  }
                  if (event.key === "Escape") setLinkOpen(false);
                }}
              />
              <button
                type="button"
                disabled={!linkIsValid}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => decorate("link")}
              >
                Apply link
              </button>
            </div>
          ) : null}
          {multiline ? (
            <textarea
              ref={editor as React.RefObject<HTMLTextAreaElement>}
              className={`${typingFormat.bold ? "is-typing-bold" : ""}${typingFormat.italic ? " is-typing-italic" : ""}`}
              autoFocus
              aria-label={label}
              value={value}
              disabled={disabled}
              placeholder={
                bullets ? "One bullet per line" : `Add ${label.toLowerCase()}`
              }
              onChange={(event) => onChange(event.target.value)}
              onSelect={rememberSelection}
              onMouseUp={rememberSelection}
              onKeyUp={rememberSelection}
            />
          ) : (
            <input
              ref={editor as React.RefObject<HTMLInputElement>}
              className={`${typingFormat.bold ? "is-typing-bold" : ""}${typingFormat.italic ? " is-typing-italic" : ""}`}
              autoFocus
              aria-label={label}
              value={value}
              disabled={disabled}
              placeholder={`Add ${label.toLowerCase()}`}
              onChange={(event) => onChange(event.target.value)}
              onSelect={rememberSelection}
              onMouseUp={rememberSelection}
              onKeyUp={rememberSelection}
            />
          )}
        </>
      ) : (
        <button
          type="button"
          className="canvas-field__button"
          disabled={disabled}
          onClick={() => setEditing(true)}
        >
          <span className="canvas-field__label">{label}</span>
          <span
            className={`canvas-field__value${bulk && bullets ? " canvas-field__value--bullets" : ""}`}
          >
            {value ? (
              <FormattedText value={value} bullets={bulk && bullets} />
            ) : (
              `Add ${label.toLowerCase()}`
            )}
          </span>
        </button>
      )}
    </div>
  );
}

function FormattedText({
  value,
  bullets = false,
}: {
  value: string;
  bullets?: boolean;
}) {
  if (bullets) {
    return value
      .split("\n")
      .filter((line) => line.trim())
      .map((line, index) => (
        <span className="formatted-bullet" key={index}>
          <span aria-hidden="true">• </span>
          <FormattedText value={line} />
        </span>
      ));
  }
  const parts = value.split(/(\*\*[^*]+\*\*|\*[^*]+\*|\[[^\]]+\]\([^\s)]+\))/g);
  return parts.map((part, index) => {
    if (part.startsWith("**") && part.endsWith("**"))
      return <strong key={index}>{part.slice(2, -2)}</strong>;
    if (part.startsWith("*") && part.endsWith("*"))
      return <em key={index}>{part.slice(1, -1)}</em>;
    const match = part.match(/^\[([^\]]+)\]\(([^\s)]+)\)$/);
    if (match && safeInlineHref(match[2]))
      return (
        <a
          key={index}
          className="formatted-link"
          href={match[2]}
          target="_blank"
          rel="noreferrer"
          onClick={(event) => event.stopPropagation()}
        >
          {match[1]}
        </a>
      );
    return part;
  });
}

function safeInlineHref(value: string): boolean {
  try {
    return ["http:", "https:", "mailto:"].includes(new URL(value).protocol);
  } catch {
    return false;
  }
}

function DisplayLink({ label, url }: { label: string; url: string }) {
  return safeInlineHref(url) ? (
    <a className="formatted-link" href={url} target="_blank" rel="noreferrer">
      {label.trim() || url}
    </a>
  ) : (
    <span>{label.trim() || url}</span>
  );
}

// Only explicit web and email protocols become navigable links.
export function PublishedResume({
  document,
  pdf = false,
  style = "technical",
  contactDivider = "dot",
  onSelect,
  onSelectEntry,
  onAddEntry,
  canAddEntry = false,
}: {
  document: ResumeDocument;
  pdf?: boolean;
  style?: DocumentStyle;
  contactDivider?: ContactDivider;
  onSelect?: (part: string) => void;
  onSelectEntry?: (sectionId: string, entryId: string) => void;
  onAddEntry?: (sectionId: string) => void;
  canAddEntry?: boolean;
}) {
  const visibleSections = document.sections.filter(
    (section) =>
      onSelect ||
      onAddEntry ||
      section.entries.some((entry) => entryHasVisibleContent(entry)),
  );
  return (
    <article
      className={`published-content resume-document resume-document--${style}`}
      aria-label={
        onSelect
          ? "Live draft resume content"
          : pdf
            ? "PDF resume content"
            : "Published resume content"
      }
    >
      <header
        className={`resume-document__contact resume-document__contact--${contactDivider}`}
      >
        {onSelect ? (
          <button
            type="button"
            className="reading-edit-target"
            onClick={() => onSelect("contact")}
          >
            {document.contact.fullName || "Add contact information"}
          </button>
        ) : (
          <h2
            className={
              document.contact.fullName.trim() ? undefined : "visually-hidden"
            }
          >
            {document.contact.fullName.trim() ? (
              <FormattedText value={document.contact.fullName} />
            ) : (
              "Resume"
            )}
          </h2>
        )}
        {[
          document.contact.email,
          document.contact.phone,
          ...document.contact.location.split("\n"),
        ]
          .filter((value) => value.trim())
          .map((value, index) => (
            <span key={index}>
              <FormattedText value={value} />
            </span>
          ))}
        {document.contact.links
          .filter((link) => link.label.trim() || link.url.trim())
          .map((link, index) => (
            <DisplayLink
              key={link.id ?? index}
              label={link.label}
              url={link.url}
            />
          ))}
      </header>
      {visibleSections.map((section) => (
        <section className="resume-document__section" key={section.id}>
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
          {section.entries.map((entry, entryIndex) =>
            entryHasVisibleContent(entry) || onSelectEntry ? (
              <div
                className={`resume-document__entry${
                  entryHasTopRowContent(entry)
                    ? ""
                    : " resume-document__entry--without-top-row"
                }`}
                key={entry.id}
              >
                <div className="resume-document__primary">
                  <div className="resume-document__title-line">
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
                          <FormattedText value={entry.heading} />
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
                    {entry.fields.some(
                      (field) =>
                        field.label !== PARAGRAPH_FIELD_LABEL &&
                        field.label.trim().toLowerCase() !== "extra" &&
                        field.value.trim(),
                    ) ? (
                      <>
                        {entry.heading.trim() ? (
                          <span
                            className="entry-title-separator"
                            aria-hidden="true"
                          >
                            |
                          </span>
                        ) : null}
                        <span className="resume-document__inline-details">
                          {entry.fields
                            .filter(
                              (field) =>
                                field.label !== PARAGRAPH_FIELD_LABEL &&
                                field.label.trim().toLowerCase() !== "extra" &&
                                field.value.trim(),
                            )
                            .slice(0, 1)
                            .map((field) => (
                              <span key={field.id}>
                                <FormattedText value={field.value} />
                              </span>
                            ))}
                        </span>
                      </>
                    ) : null}
                  </div>
                  {entry.subheading.trim() ? (
                    <p>
                      <FormattedText value={entry.subheading} />
                    </p>
                  ) : null}
                </div>
                <div className="resume-document__meta">
                  {entry.location.trim() ? (
                    <p>
                      <FormattedText value={entry.location} />
                    </p>
                  ) : null}
                  {entry.dateRange.trim() ? (
                    <p>
                      <FormattedText value={entry.dateRange} />
                    </p>
                  ) : null}
                  {entry.dates
                    ?.filter((date) => dateText(date))
                    .map((date) => (
                      <p key={date.id}>{dateText(date)}</p>
                    ))}
                  {entry.fields
                    .filter(
                      (field) =>
                        field.label.trim().toLowerCase() === "extra" &&
                        field.value.trim(),
                    )
                    .map((field) => (
                      <p key={field.id}>
                        <FormattedText value={field.value} />
                      </p>
                    ))}
                </div>
                {entry.fields
                  .find((field) => field.label === PARAGRAPH_FIELD_LABEL)
                  ?.value.trim() ? (
                  <p className="resume-document__body resume-document__body--paragraph">
                    <FormattedText
                      value={
                        entry.fields.find(
                          (field) => field.label === PARAGRAPH_FIELD_LABEL,
                        )?.value ?? ""
                      }
                    />
                  </p>
                ) : entry.bullets.some((bullet) => bullet.text.trim()) ? (
                  <ul className="resume-document__body">
                    {entry.bullets
                      .filter((bullet) => bullet.text.trim())
                      .map((bullet) => (
                        <li key={bullet.id}>
                          <FormattedText value={bullet.text} />
                        </li>
                      ))}
                  </ul>
                ) : null}
                {entry.links.some(
                  (link) => link.label.trim() || link.url.trim(),
                ) ? (
                  <p className="resume-document__links">
                    {entry.links
                      .filter((link) => link.label.trim() || link.url.trim())
                      .map((link, index) => (
                        <DisplayLink
                          key={link.id ?? index}
                          label={link.label}
                          url={link.url}
                        />
                      ))}
                  </p>
                ) : null}
              </div>
            ) : null,
          )}
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

function entryHasVisibleContent(entry: ResumeEntry): boolean {
  return Boolean(
    entry.heading.trim() ||
      entry.subheading.trim() ||
      entry.dateRange.trim() ||
      entry.location.trim() ||
      entry.dates?.some((date) => dateText(date)) ||
      entry.fields.some((field) => field.value.trim()) ||
      entry.bullets.some((bullet) => bullet.text.trim()) ||
      entry.links.some((link) => link.label.trim() || link.url.trim()),
  );
}

function entryHasTopRowContent(entry: ResumeEntry): boolean {
  return Boolean(
    entry.heading.trim() ||
      entry.subheading.trim() ||
      entry.dateRange.trim() ||
      entry.location.trim() ||
      entry.dates?.some((date) => dateText(date)) ||
      entry.fields.some(
        (field) =>
          field.label !== PARAGRAPH_FIELD_LABEL && field.value.trim(),
      ),
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
        <Brand title="Application workspace" />
      </header>
      <section className="status-card">
        <h2>Your resume workspace is in the main window</h2>
        <HealthBadge state={health} />
        <p className="description">
          Build, review, and export your resume in the main window. This
          companion window will hold application materials when application
          workflows are available.
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
      return "Your local storage is unavailable. Please try again.";
    case "COMMAND_UNAVAILABLE":
    case "INVALID_RESPONSE":
      return "The save result is uncertain. Reload the saved draft to check what reached storage before retrying.";
    default:
      return "The operation could not be completed safely. Try again.";
  }
}
