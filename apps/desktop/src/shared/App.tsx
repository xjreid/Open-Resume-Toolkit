import {
  cloneElement,
  isValidElement,
  useCallback,
  useEffect,
  useId,
  useState,
} from "react";
import type { HealthResponse } from "@ort/contracts/health";
import type {
  ResumeDocument,
  ResumeEntry,
  ResumeSection,
} from "@ort/contracts/resume";
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
  const [document, setDocument] = useState<ResumeDocument | null>(null);
  const [revision, setRevision] = useState<number | null>(null);
  const [publishedRevision, setPublishedRevision] = useState<number | null>(
    null,
  );
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const loadWorkspace = useCallback(async () => {
    setHealth({ kind: "checking" });
    setNotice(null);
    const healthResult = await requestHealth();
    if (!healthResult.ok) {
      setHealth({ kind: "error", message: healthResult.error.messageKey });
      return;
    }
    setHealth({ kind: "ready", health: healthResult.value });
    if (healthResult.value.storageStatus !== "ready") {
      setNotice("Encrypted storage is unavailable. No edits can be saved.");
      return;
    }

    const workspace = await requestResumeWorkspace();
    if (!workspace.ok) {
      setNotice(friendlyError(workspace.error.code));
      return;
    }
    setDocument(workspace.value.draft?.document ?? createResumeDocument());
    setRevision(workspace.value.draft?.revision ?? null);
    setPublishedRevision(workspace.value.latestPublished?.revision ?? null);
    setDirty(false);
  }, []);

  useEffect(() => {
    void loadWorkspace();
  }, [loadWorkspace]);

  function changeDocument(update: (current: ResumeDocument) => ResumeDocument) {
    setDocument((current) => (current ? update(current) : current));
    setDirty(true);
    setNotice(null);
  }

  async function save() {
    if (!document || busy) return;
    setBusy(true);
    setNotice(null);
    const normalized = normalizeDocument(document);
    const result = await saveResume(revision, normalized);
    if (result.ok) {
      setDocument(result.value.document);
      setRevision(result.value.revision);
      setDirty(false);
      setNotice(`Draft revision ${result.value.revision} saved securely.`);
    } else {
      setNotice(friendlyError(result.error.code));
    }
    setBusy(false);
  }

  async function publish() {
    if (revision === null || dirty || busy) return;
    setBusy(true);
    setNotice(null);
    const result = await publishResume(revision);
    if (result.ok) {
      setPublishedRevision(result.value.published.revision);
      setNotice(
        `Published immutable snapshot ${result.value.published.revision} from draft ${result.value.draftRevision}.`,
      );
    } else {
      setNotice(friendlyError(result.error.code));
    }
    setBusy(false);
  }

  const storageReady =
    health.kind === "ready" && health.health.storageStatus === "ready";

  return (
    <main className="shell shell--editor">
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
          <strong>{dirty ? "Unsaved" : "Saved"}</strong>
        </div>
        <div className="workspace-actions">
          <button
            type="button"
            onClick={() => void save()}
            disabled={
              !storageReady ||
              !document ||
              busy ||
              (!dirty && revision !== null)
            }
          >
            {busy ? "Working…" : "Save draft"}
          </button>
          <button
            className="button--secondary"
            type="button"
            onClick={() => void publish()}
            disabled={!storageReady || revision === null || dirty || busy}
          >
            Publish snapshot
          </button>
        </div>
      </section>

      {notice ? (
        <p className="notice" role="status">
          {notice}
        </p>
      ) : null}

      {document ? (
        <div className="editor-layout">
          <section className="editor-panel" aria-labelledby="identity-heading">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Structured resume</p>
                <h2 id="identity-heading">Identity and contact</h2>
              </div>
            </div>
            <div className="field-grid">
              <Field label="Resume title" wide>
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
                <Field key={field} label={label}>
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
                  onChange={(next) =>
                    changeDocument((current) => ({
                      ...current,
                      sections: current.sections.map((candidate, index) =>
                        index === sectionIndex ? next : candidate,
                      ),
                    }))
                  }
                  onRemove={() =>
                    changeDocument((current) => ({
                      ...current,
                      sections: current.sections.filter(
                        (_, index) => index !== sectionIndex,
                      ),
                    }))
                  }
                />
              ))}
            </div>
          </section>
        </div>
      ) : (
        <section className="status-card">
          <h2>Opening encrypted workspace</h2>
          <p className="description">
            The application is connecting to its isolated database and OS
            credential vault.
          </p>
          {health.kind === "error" ? (
            <button type="button" onClick={() => void loadWorkspace()}>
              Try again
            </button>
          ) : null}
        </section>
      )}

      <footer className="development-gates">
        Document import, PDF preview, exports, AI, and browser access remain
        disabled until their later M2 security gates pass.
      </footer>
    </main>
  );
}

function ResumeSectionEditor({
  section,
  onChange,
  onRemove,
}: {
  section: ResumeSection;
  onChange: (section: ResumeSection) => void;
  onRemove: () => void;
}) {
  return (
    <article className="resume-section">
      <div className="resume-section__header">
        <Field label="Section heading" wide>
          <input
            value={section.heading}
            maxLength={2000}
            required
            onChange={(event) =>
              onChange({ ...section, heading: event.target.value })
            }
          />
        </Field>
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
          onChange={(next) =>
            onChange({
              ...section,
              entries: section.entries.map((candidate, index) =>
                index === entryIndex ? next : candidate,
              ),
            })
          }
          onRemove={() =>
            onChange({
              ...section,
              entries: section.entries.filter(
                (_, index) => index !== entryIndex,
              ),
            })
          }
        />
      ))}

      <button
        className="button--quiet button--compact"
        type="button"
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
}: {
  entry: ResumeEntry;
  onChange: (entry: ResumeEntry) => void;
  onRemove: () => void;
}) {
  function updateField(field: EntryTextField, value: string) {
    onChange({ ...entry, [field]: value });
  }

  return (
    <div className="resume-entry">
      <div className="entry-actions">
        <strong>Entry {entry.order + 1}</strong>
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
          <Field key={field} label={label}>
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
            <Field label={`Bullet ${bulletIndex + 1}`} wide>
              <textarea
                value={bullet.text}
                maxLength={500}
                rows={2}
                onChange={(event) =>
                  onChange({
                    ...entry,
                    bullets: entry.bullets.map((candidate, index) =>
                      index === bulletIndex
                        ? { ...candidate, text: event.target.value }
                        : candidate,
                    ),
                  })
                }
              />
            </Field>
            <button
              className="button--danger button--icon"
              type="button"
              aria-label={`Remove bullet ${bulletIndex + 1}`}
              onClick={() =>
                onChange({
                  ...entry,
                  bullets: entry.bullets.filter(
                    (_, index) => index !== bulletIndex,
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
    </div>
  );
}

function Field({
  label,
  wide = false,
  children,
}: {
  label: string;
  wide?: boolean;
  children: React.ReactNode;
}) {
  const id = useId();
  return (
    <label className={wide ? "field field--wide" : "field"} htmlFor={id}>
      <span>{label}</span>
      {cloneInputWithId(children, id)}
    </label>
  );
}

function cloneInputWithId(children: React.ReactNode, id: string) {
  if (!isValidElement<{ id?: string }>(children)) return children;
  return cloneElement(children, { id });
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
    default:
      return "The operation could not be completed safely. Try again.";
  }
}
