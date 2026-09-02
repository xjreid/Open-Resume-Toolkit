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
  Link,
  ResumeDocument,
  ResumeEntry,
  ResumeSection,
} from "@ort/contracts/resume";
import { DOCUMENT_LIMITS } from "@ort/contracts/resume";
import { CloseDialog } from "./CloseDialog";
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
} from "./command-client";
import {
  createBullet,
  createEntry,
  createResumeDocument,
  createSection,
  createNamedField,
  moveItem,
  normalizeDocument,
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
  const close = useCloseGuard(editor);
  const [confirmReload, setConfirmReload] = useState(false);
  const ioBusy = useRef(false);
  const loadGeneration = useRef(0);
  const { document, notice } = editor;
  const revision = editor.saved?.revision ?? null;
  const publishedRevision = editor.published?.revision ?? null;
  const dirty = isDirty(editor);
  const busy = editor.status !== "idle";
  const mustReload = requiresReload(editor);
  const issues = useMemo(
    () => (document ? validateEditorDocument(document) : []),
    [document],
  );
  const usage = document ? documentUsage(document) : null;

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
        busy={busy}
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

      <div className="editor-tools">
        <p>
          Valid changes autosave after a short pause. Closing checks for unsaved
          edits; invalid edits must be corrected or explicitly discarded. Use
          the app menu or window close button. macOS Dock Quit and system
          shutdown are not yet protected; wait for Saved first. Use synthetic
          data only.
        </p>
        <div className="move-controls">
          <button
            type="button"
            className="button--secondary button--compact"
            disabled={
              !editor.undo.length ||
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
              <li key={`${issue.path}-${index}`}>{issue.message}</li>
            ))}
          </ul>
        </section>
      ) : null}

      {notice ? (
        <p className="notice" role="status">
          {notice}
        </p>
      ) : null}

      {document ? (
        <fieldset
          className="editor-layout editor-fields"
          disabled={
            editor.status === "loading" || confirmReload || close.pending
          }
        >
          <section className="editor-panel" aria-labelledby="identity-heading">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Structured resume</p>
                <h2 id="identity-heading">Identity and contact</h2>
              </div>
            </div>
            <div className="field-grid">
              <Field label="Resume title" wide error={issueAt(issues, "title")}>
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
                  error={issueAt(issues, `contact.${field}`)}
                >
                  <input
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

          <section className="editor-panel" aria-labelledby="sections-heading">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Ordered content</p>
                <h2 id="sections-heading">Resume sections</h2>
              </div>
              <button
                className="button--secondary button--compact"
                type="button"
                disabled={usage!.sections >= DOCUMENT_LIMITS.sections}
                onClick={() =>
                  changeDocument((current) => ({
                    ...current,
                    sections: [
                      ...current.sections,
                      createSection(current.sections.length),
                    ],
                  }))
                }
              >
                Add section
              </button>
            </div>

            {document.sections.length === 0 ? (
              <div className="empty-state">
                <p>Add sections such as Experience, Education, or Skills.</p>
              </div>
            ) : null}

            <div className="section-list">
              {document.sections.map((section, sectionIndex) => (
                <ResumeSectionEditor
                  key={section.id}
                  section={section}
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
                        candidate.id === section.id ? next : candidate,
                      ),
                    }))
                  }
                  onRemove={() =>
                    changeDocument((current) => ({
                      ...current,
                      sections: current.sections.filter(
                        (candidate) => candidate.id !== section.id,
                      ),
                    }))
                  }
                />
              ))}
            </div>
          </section>
        </fieldset>
      ) : (
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
      )}

      {editor.published ? (
        <details className="editor-panel published-review">
          <summary>
            Review published snapshot {editor.published.revision} (read-only)
          </summary>
          <PublishedResume document={editor.published.document} />
        </details>
      ) : null}

      <footer className="development-gates">
        This is a structured text review, not a PDF preview. Document import and
        exports remain gated in M2; AI and browser integration arrive in later
        milestones.
      </footer>
    </main>
  );
}

function ResumeSectionEditor({
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
  onChange: (section: ResumeSection) => void;
  onRemove: () => void;
  onMove: (direction: -1 | 1) => void;
  index: number;
  count: number;
  issues: ValidationIssue[];
  usage: ReturnType<typeof documentUsage>;
}) {
  return (
    <article className="resume-section">
      <div className="resume-section__header">
        <Field
          label="Section heading"
          wide
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
        <button
          className="button--danger button--compact"
          type="button"
          onClick={onRemove}
        >
          Remove section
        </button>
      </div>

      {section.entries.map((entry, entryIndex) => (
        <ResumeEntryEditor
          key={entry.id}
          entry={entry}
          issues={issues}
          usage={usage}
          index={entryIndex}
          count={section.entries.length}
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
          onRemove={() =>
            onChange({
              ...section,
              entries: section.entries.filter(
                (candidate) => candidate.id !== entry.id,
              ),
            })
          }
        />
      ))}

      <button
        className="button--quiet button--compact"
        type="button"
        disabled={usage.entries >= DOCUMENT_LIMITS.entries}
        onClick={() =>
          onChange({
            ...section,
            entries: [...section.entries, createEntry(section.entries.length)],
          })
        }
      >
        Add entry
      </button>
    </article>
  );
}

function ResumeEntryEditor({
  entry,
  onChange,
  onRemove,
  onMove,
  index,
  count,
  issues,
  usage,
}: {
  entry: ResumeEntry;
  onChange: (entry: ResumeEntry) => void;
  onRemove: () => void;
  onMove: (direction: -1 | 1) => void;
  index: number;
  count: number;
  issues: ValidationIssue[];
  usage: ReturnType<typeof documentUsage>;
}) {
  function updateField(field: EntryTextField, value: string) {
    onChange({ ...entry, [field]: value });
  }

  return (
    <div className="resume-entry">
      <div className="entry-actions">
        <strong>Entry {entry.order + 1}</strong>
        <MoveControls
          label={`entry ${index + 1}`}
          index={index}
          count={count}
          onMove={onMove}
        />
        <button
          className="button--danger button--compact"
          type="button"
          onClick={onRemove}
        >
          Remove entry
        </button>
      </div>
      <div className="field-grid">
        {(
          [
            ["heading", "Role or qualification"],
            ["subheading", "Organization"],
            ["dateRange", "Date range"],
            ["location", "Location"],
          ] as const
        ).map(([field, label]) => (
          <Field
            key={field}
            label={label}
            error={issueAt(issues, `entry.${entry.id}.${field}`)}
          >
            <input
              value={entry[field]}
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
      <div className="custom-fields">
        <h3>Custom fields and skills</h3>
        {entry.fields.map((field, fieldIndex) => (
          <div className="custom-field" key={field.id}>
            <Field
              label={`Field ${fieldIndex + 1} label`}
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
          onClick={() =>
            onChange({
              ...entry,
              fields: [...entry.fields, createNamedField(entry.fields.length)],
            })
          }
        >
          Add custom field
        </button>
      </div>
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
  label,
  wide = false,
  children,
  error,
}: {
  label: string;
  wide?: boolean;
  children: React.ReactNode;
  error?: string;
}) {
  const id = useId();
  return (
    <label className={wide ? "field field--wide" : "field"} htmlFor={id}>
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
        <div className="link-row" key={index}>
          <Field
            label={`Link ${index + 1} label`}
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
export function PublishedResume({ document }: { document: ResumeDocument }) {
  return (
    <article
      className="published-content"
      aria-label="Published resume content"
    >
      <h2>{document.title}</h2>
      <p>{document.contact.fullName}</p>
      <p>
        {[
          document.contact.email,
          document.contact.phone,
          document.contact.location,
        ]
          .filter(Boolean)
          .join(" · ")}
      </p>
      {document.contact.links.map((link, index) => (
        <p key={index}>
          {link.label}: {link.url}
        </p>
      ))}
      {document.sections.map((section) => (
        <section key={section.id}>
          <h3>{section.heading}</h3>
          {section.entries.map((entry) => (
            <div key={entry.id}>
              <h4>{entry.heading}</h4>
              <p>
                {[entry.subheading, entry.dateRange, entry.location]
                  .filter(Boolean)
                  .join(" · ")}
              </p>
              <dl>
                {entry.fields.map((field) => (
                  <div key={field.id}>
                    <dt>
                      {field.label}
                      {field.isSkill ? " (skill)" : ""}
                    </dt>
                    <dd>{field.value}</dd>
                  </div>
                ))}
              </dl>
              <ul>
                {entry.bullets.map((bullet) => (
                  <li key={bullet.id}>{bullet.text}</li>
                ))}
              </ul>
              {entry.links.map((link, index) => (
                <p key={index}>
                  {link.label}: {link.url}
                </p>
              ))}
            </div>
          ))}
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
