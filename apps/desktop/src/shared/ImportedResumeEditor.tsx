import { useEffect, useId, useRef, useState } from "react";
import type { DocumentStyle } from "@ort/contracts/export";
import { DOCUMENT_LIMITS, type ResumeDocument } from "@ort/contracts/resume";
import { ResumeCanvas } from "./ResumeCanvas";
import { ResumeSectionNavigator } from "./ResumeSectionNavigator";
import { UndoIcon, RedoIcon } from "./ResumeHistoryIcons";
import { normalizeDocument } from "./resume-editor";
import { documentUsage, validateEditorDocument } from "./resume-validation";

export function ImportedResumeEditor({
  initialDocument,
  style = "technical",
  busy,
  blocked,
  error,
  onMap,
  onCancel,
}: {
  initialDocument: ResumeDocument;
  style?: DocumentStyle;
  busy: boolean;
  blocked: boolean;
  error?: string;
  onMap: (document: ResumeDocument) => void;
  onCancel: () => void;
}) {
  const [history, setHistory] = useState({
    document: initialDocument,
    undo: [] as ResumeDocument[],
    redo: [] as ResumeDocument[],
  });
  const [divider, setDivider] = useState<"dot" | "bar" | "dash">("dot");
  const heading = useRef<HTMLHeadingElement>(null);
  const headingId = useId();
  useEffect(() => {
    heading.current?.focus();
  }, []);
  const document = history.document;
  const issues = validateEditorDocument(document);
  function issueLabel(path: string) {
    const contactLabels: Record<string, string> = {
      "contact.fullName": "Name",
      "contact.email": "Contact information",
      "contact.phone": "Contact information",
      "contact.location": "Contact information",
    };
    if (contactLabels[path]) return contactLabels[path];
    if (path === "title") return "Resume title";
    for (const section of document.sections) {
      if (path.includes(section.id)) return section.heading || "Section name";
      for (const [index, entry] of section.entries.entries()) {
        if (
          path.includes(entry.id) ||
          [...entry.fields, ...entry.bullets, ...(entry.dates ?? [])].some(
            (item) => path.includes(item.id),
          )
        ) {
          return `${section.heading || "Section"} / ${entry.heading || `Item ${index + 1}`}`;
        }
      }
    }
    return "Resume";
  }
  const disabled = busy || blocked;
  function change(update: (current: ResumeDocument) => ResumeDocument) {
    if (disabled) return;
    setHistory((current) => ({
      document: normalizeDocument(update(current.document)),
      undo: [...current.undo.slice(-49), current.document],
      redo: [],
    }));
  }
  function undo() {
    setHistory((current) =>
      current.undo.length
        ? {
            document: current.undo.at(-1)!,
            undo: current.undo.slice(0, -1),
            redo: [current.document, ...current.redo],
          }
        : current,
    );
  }
  function redo() {
    setHistory((current) =>
      current.redo.length
        ? {
            document: current.redo[0],
            undo: [...current.undo, current.document],
            redo: current.redo.slice(1),
          }
        : current,
    );
  }
  return (
    <section
      className="import-review-editor"
      aria-label="Review imported resume"
    >
      <fieldset className="import-review-editor__fields" disabled={disabled}>
        <legend className="import-review-editor__legend">
          Imported resume
        </legend>
        <div className="document-workspace import-review-editor__workspace">
          <ResumeSectionNavigator
            document={document}
            disabled={disabled}
            onChange={change}
          />
          <div className="resume-reading-panel">
            <header className="resume-display-header">
              <div className="resume-display-header__title-row">
                <div className="resume-display-header__title">
                  <h2 id={headingId} ref={heading} tabIndex={-1}>
                    Review imported resume
                  </h2>
                  <span className="resume-save-status" role="status">
                    <span
                      className="resume-save-status__dot"
                      aria-hidden="true"
                    />
                    {busy
                      ? "Working…"
                      : error
                        ? "Review needs attention"
                        : "Not saved yet"}
                  </span>
                </div>
                <div
                  className="resume-history-actions"
                  role="group"
                  aria-label="Import edit history"
                >
                  <button
                    type="button"
                    className="button--secondary button--compact"
                    aria-label="Undo import edit"
                    title="Undo"
                    disabled={disabled || !history.undo.length}
                    onClick={undo}
                  >
                    <UndoIcon />
                  </button>
                  <button
                    type="button"
                    className="button--secondary button--compact"
                    aria-label="Redo import edit"
                    title="Redo"
                    disabled={disabled || !history.redo.length}
                    onClick={redo}
                  >
                    <RedoIcon />
                  </button>
                </div>
              </div>
              <p className="import-review-editor__save-note">
                Review edits are temporary until you map them to your saved
                resume.
              </p>
            </header>
            <ResumeCanvas
              document={document}
              style={style}
              contactDivider={divider}
              onContactDividerChange={setDivider}
              disabled={disabled}
              canAddEntry={
                documentUsage(document).entries < DOCUMENT_LIMITS.entries
              }
              onChange={change}
            />
          </div>
        </div>
      </fieldset>
      {issues.length > 0 && (
        <div className="import-review-editor__issues" role="alert">
          <p>Correct these fields before mapping:</p>
          <ul>
            {issues.map((issue) => (
              <li key={`${issue.path}-${issue.message}`}>
                {issueLabel(issue.path)}: {issue.message}
              </li>
            ))}
          </ul>
        </div>
      )}
      {error && <p role="alert">{error}</p>}
      <footer className="import-review-editor__actions">
        <button
          type="button"
          className="button--secondary"
          disabled={busy}
          onClick={onCancel}
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={disabled || issues.length > 0}
          onClick={() => onMap(document)}
        >
          {busy ? "Please wait…" : "Map to current saved resume"}
        </button>
      </footer>
    </section>
  );
}
