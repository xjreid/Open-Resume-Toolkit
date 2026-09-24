import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { PdfCanvas } from "./ApplicationViews";
import {
  TrackerFields,
  emptyTrackerEntry,
  type TrackerEntry,
} from "./TrackerFields";
import { trackerLinkTarget } from "./tracker-link";

type RecordEntry = { id: string; revision: number; value: TrackerEntry };
type Response<T> =
  | { ok: true; value: T }
  | { ok: false; error: { code: string } };
type ContentKind = "resume" | "cover_letter" | "answers";
type ContentView = { id: string; kind: ContentKind };

async function command<T>(name: string, args: Record<string, unknown> = {}) {
  const response = await invoke<Response<T>>(name, args);
  if (!response.ok) throw new Error(response.error.code);
  return response.value;
}

const statuses = [
  ["applied", "Applied"],
  ["online_assessment", "Online Assessment"],
  ["interview", "Interview"],
  ["accepted", "Accepted"],
  ["rejected", "Rejected"],
  ["withdrawn", "Withdrawn"],
  ["other", "Other"],
] as const;

function csvCell(value: string) {
  const safe = /^[\s\u0000-\u001f]*[=+@-]/u.test(value) ? "'" + value : value;
  return '"' + safe.replaceAll('"', '""') + '"';
}

function contentLabel(kind: ContentKind) {
  if (kind === "resume") return "Final resume";
  if (kind === "cover_letter") return "Cover letter";
  return "Approved answers";
}

export function TrackerWorkspace({
  active,
  onDirtyChange,
}: {
  active: boolean;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const [entries, setEntries] = useState<RecordEntry[]>([]);
  const loadEpoch = useRef(0);
  const committed = useRef(new Map<string, RecordEntry>());
  const desired = useRef(new Map<string, TrackerEntry>());
  const timers = useRef(new Map<string, number>());
  const inFlight = useRef(new Map<string, Promise<void>>());
  const deletingIds = useRef(new Set<string>());
  const failed = useRef(new Set<string>());
  const [pendingCount, setPendingCount] = useState(0);
  const [saveErrors, setSaveErrors] = useState<Record<string, string>>({});
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("");
  const [editingLinkId, setEditingLinkId] = useState<string | null>(null);
  const linkTimer = useRef<number | null>(null);
  const [notice, setNotice] = useState("");
  const [createOpen, setCreateOpen] = useState(false);
  const [createDraft, setCreateDraft] = useState<TrackerEntry | null>(null);
  const [creating, setCreating] = useState(false);
  const createDialog = useRef<HTMLDialogElement>(null);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const deleteDialog = useRef<HTMLDialogElement>(null);
  const [content, setContent] = useState<ContentView | null>(null);
  const [pdf, setPdf] = useState<{ base64: string; filename: string } | null>(
    null,
  );
  const [contentLoading, setContentLoading] = useState(false);
  const [contentError, setContentError] = useState("");
  const contentRequest = useRef(0);
  const contentDialog = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    const epoch = loadEpoch.current;
    void command<RecordEntry[]>("list_tracker_entries")
      .then((loaded) => {
        if (cancelled || epoch !== loadEpoch.current || desired.current.size)
          return;
        committed.current = new Map(
          loaded.map((record) => [record.id, record]),
        );
        setEntries(loaded);
      })
      .catch((error: unknown) => {
        if (!cancelled)
          setNotice("Could not load tracker (" + String(error) + ").");
      });
    return () => {
      cancelled = true;
    };
  }, [active]);

  useEffect(() => {
    const dialog = createDialog.current;
    if (!dialog) return;
    if (createOpen && !dialog.open) {
      dialog.showModal();
      dialog.querySelector<HTMLInputElement>("input")?.focus();
    }
    if (!createOpen && dialog.open) dialog.close();
  }, [createOpen]);
  useEffect(() => {
    const dialog = deleteDialog.current;
    if (!dialog) return;
    if (deleteId && !dialog.open) dialog.showModal();
    if (!deleteId && dialog.open) dialog.close();
  }, [deleteId]);
  useEffect(() => {
    const dialog = contentDialog.current;
    if (!dialog) return;
    if (content && !dialog.open) dialog.showModal();
    if (!content && dialog.open) dialog.close();
  }, [content]);

  const createDirty =
    !!createDraft &&
    JSON.stringify(createDraft) !== JSON.stringify(emptyTrackerEntry());
  useEffect(
    () => onDirtyChange(pendingCount > 0 || createDirty),
    [pendingCount, createDirty, onDirtyChange],
  );
  useEffect(
    () => () => {
      for (const timer of timers.current.values()) window.clearTimeout(timer);
      if (linkTimer.current !== null) window.clearTimeout(linkTimer.current);
    },
    [],
  );

  const visible = useMemo(() => {
    const term = search.trim().toLocaleLowerCase();
    return entries
      .map((record, index) => ({ record, index }))
      .filter(({ record: { value } }) => {
        if (filter && value.status !== filter) return false;
        if (!term) return true;
        return [
          value.company,
          value.title,
          value.location,
          value.dateApplied,
          value.status,
          value.customStatus,
          value.sourceUrl,
          value.resume?.title ?? "",
          value.coverLetter ?? "",
          ...value.answers.flatMap(({ question, answer }) => [
            question,
            answer,
          ]),
        ].some((part) => part.toLocaleLowerCase().includes(term));
      })
      .sort(
        (a, b) =>
          b.record.value.dateApplied.localeCompare(
            a.record.value.dateApplied,
          ) || a.index - b.index,
      )
      .map(({ record }) => record);
  }, [entries, search, filter]);

  function clearTimer(id: string) {
    const timer = timers.current.get(id);
    if (timer !== undefined) window.clearTimeout(timer);
    timers.current.delete(id);
  }
  function schedule(id: string, delay: number) {
    clearTimer(id);
    if (deletingIds.current.has(id)) return;
    timers.current.set(
      id,
      window.setTimeout(() => {
        timers.current.delete(id);
        void persist(id);
      }, delay),
    );
  }
  function edit(
    id: string,
    change: (value: TrackerEntry) => TrackerEntry,
    delay = 400,
  ) {
    const base = desired.current.get(id) ?? committed.current.get(id)?.value;
    if (!base) return;
    const next = change(base);
    loadEpoch.current += 1;
    desired.current.set(id, next);
    failed.current.delete(id);
    setSaveErrors((current) => {
      const updated = { ...current };
      delete updated[id];
      return updated;
    });
    setPendingCount(desired.current.size);
    setEntries((current) =>
      current.map((record) =>
        record.id === id ? { ...record, value: next } : record,
      ),
    );
    if (delay >= 0) schedule(id, delay);
    else clearTimer(id);
  }
  function beginLinkEdit(id: string) {
    if (linkTimer.current !== null) window.clearTimeout(linkTimer.current);
    linkTimer.current = null;
    setEditingLinkId(id);
  }
  function openLink(source: string) {
    const target = trackerLinkTarget(source);
    if (!target) return;
    void command<boolean>("open_tracker_link", { target }).catch((error) =>
      setNotice("Could not open link (" + String(error) + ")."),
    );
  }
  function linkClick(
    event: MouseEvent<HTMLAnchorElement>,
    id: string,
    source: string,
  ) {
    event.preventDefault();
    if (event.detail >= 2) {
      beginLinkEdit(id);
      return;
    }
    if (event.detail === 0) {
      openLink(source);
      return;
    }
    if (linkTimer.current !== null) window.clearTimeout(linkTimer.current);
    linkTimer.current = window.setTimeout(() => {
      linkTimer.current = null;
      openLink(source);
    }, 280);
  }
  async function persist(id: string): Promise<void> {
    clearTimer(id);
    const existing = inFlight.current.get(id);
    if (existing) return existing;
    const run = async () => {
      while (true) {
        const target = desired.current.get(id);
        const previous = committed.current.get(id);
        if (!target || !previous) return;
        if (JSON.stringify(target) === JSON.stringify(previous.value)) {
          desired.current.delete(id);
          setPendingCount(desired.current.size);
          return;
        }
        try {
          const saved = await command<RecordEntry>("save_tracker_entry", {
            id,
            expectedRevision: previous.revision,
            entry: target,
          });
          committed.current.set(id, saved);
          const latest = desired.current.get(id);
          if (latest === target) desired.current.delete(id);
          setPendingCount(desired.current.size);
          setEntries((current) =>
            current.map((record) =>
              record.id === id
                ? {
                    ...saved,
                    value:
                      latest === target ? saved.value : (latest ?? saved.value),
                  }
                : record,
            ),
          );
          failed.current.delete(id);
          setSaveErrors((current) => {
            const updated = { ...current };
            delete updated[id];
            return updated;
          });
        } catch (error) {
          failed.current.add(id);
          setSaveErrors((current) => ({ ...current, [id]: String(error) }));
          return;
        }
      }
    };
    const task = run().finally(() => {
      inFlight.current.delete(id);
      if (
        desired.current.has(id) &&
        !failed.current.has(id) &&
        !deletingIds.current.has(id)
      )
        schedule(id, 0);
    });
    inFlight.current.set(id, task);
    return task;
  }

  async function createEntry() {
    if (!createDraft || creating) return;
    setCreating(true);
    setNotice("");
    try {
      const record = await command<RecordEntry>("save_tracker_entry", {
        id: null,
        expectedRevision: null,
        entry: createDraft,
      });
      loadEpoch.current += 1;
      committed.current.set(record.id, record);
      setEntries((current) => [record, ...current]);
      setCreateDraft(null);
      setCreateOpen(false);
      setNotice("Application added.");
    } catch (error) {
      setNotice("Could not add application (" + String(error) + ").");
    } finally {
      setCreating(false);
    }
  }
  async function deleteEntry() {
    if (!deleteId || deleting) return;
    const id = deleteId;
    setDeleting(true);
    deletingIds.current.add(id);
    clearTimer(id);
    try {
      await inFlight.current.get(id);
      const record = committed.current.get(id);
      if (!record) throw new Error("TRACKER_NOT_FOUND");
      await command<boolean>("delete_tracker_entry", {
        id,
        expectedRevision: record.revision,
      });
      loadEpoch.current += 1;
      committed.current.delete(id);
      desired.current.delete(id);
      failed.current.delete(id);
      setPendingCount(desired.current.size);
      setSaveErrors((current) => {
        const updated = { ...current };
        delete updated[id];
        return updated;
      });
      setEntries((current) => current.filter((item) => item.id !== id));
      deletingIds.current.delete(id);
      setDeleteId(null);
      setNotice("Application deleted.");
    } catch (error) {
      deletingIds.current.delete(id);
      setNotice("Delete failed (" + String(error) + ").");
      if (desired.current.has(id) && !failed.current.has(id)) schedule(id, 0);
    } finally {
      setDeleting(false);
    }
  }
  async function openContent(record: RecordEntry, kind: ContentKind) {
    const request = ++contentRequest.current;
    setContent({ id: record.id, kind });
    setPdf(null);
    setContentError("");
    setContentLoading(false);
    if (kind === "answers") return;
    setContentLoading(true);
    try {
      await persist(record.id);
      const saved = committed.current.get(record.id);
      if (!saved) throw new Error("TRACKER_NOT_FOUND");
      const result = await command<{ base64: string; filename: string }>(
        "preview_tracker_pdf",
        { id: record.id, expectedRevision: saved.revision, kind },
      );
      if (request === contentRequest.current) setPdf(result);
    } catch (error) {
      if (request === contentRequest.current)
        setContentError("Could not open this PDF (" + String(error) + ").");
    } finally {
      if (request === contentRequest.current) setContentLoading(false);
    }
  }
  function closeContent() {
    contentRequest.current += 1;
    setContent(null);
    setPdf(null);
    setContentError("");
    setContentLoading(false);
  }
  function exportCsv() {
    const rows = [
      [
        "Date applied",
        "Status",
        "Company",
        "Job title",
        "Location",
        "Link or source",
      ],
      ...visible.map(({ value }) => [
        value.dateApplied,
        value.status === "other" ? value.customStatus : value.status,
        value.company,
        value.title,
        value.location,
        value.sourceUrl,
      ]),
    ];
    const blob = new Blob(
      ["\uFEFF", rows.map((row) => row.map(csvCell).join(",")).join("\r\n")],
      { type: "text/csv;charset=utf-8" },
    );
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "application-tracker.csv";
    anchor.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
  }

  const contentRecord = content
    ? entries.find((record) => record.id === content.id)
    : null;
  const deleteRecord = deleteId
    ? entries.find((record) => record.id === deleteId)
    : null;

  return (
    <section className="tracker-workspace">
      <div className="tracker-heading">
        <div>
          <p className="application-kicker">Local applications</p>
          <h2>Application tracker</h2>
        </div>
        <div className="application-row">
          <button
            type="button"
            onClick={() => {
              setCreateDraft(emptyTrackerEntry());
              setCreateOpen(true);
            }}
          >
            New entry
          </button>
          <button
            type="button"
            className="button--secondary"
            onClick={exportCsv}
            disabled={!visible.length}
          >
            Export visible CSV
          </button>
        </div>
      </div>
      {notice && (
        <p role="alert" className="notice">
          {notice}
        </p>
      )}
      {Object.keys(saveErrors).length > 0 && (
        <p role="alert" className="notice">
          Some changes could not be saved. Correct the highlighted field or{" "}
          <button
            type="button"
            className="button--secondary"
            onClick={() => {
              for (const id of Object.keys(saveErrors)) {
                failed.current.delete(id);
                void persist(id);
              }
            }}
          >
            Retry saving
          </button>
        </p>
      )}
      <div className="tracker-filters">
        <label>
          Search
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search all application columns"
          />
        </label>
        <label>
          Status
          <select
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
          >
            <option value="">All statuses</option>
            {statuses.map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
      </div>
      <p className="tracker-table-hint">
        Edit fields directly. Changes save automatically. Content is a read-only
        snapshot of the finished application.
      </p>
      <div
        className="tracker-table-scroll"
        role="region"
        aria-label="Application tracker table"
        tabIndex={0}
      >
        <table className="tracker-table">
          <thead>
            <tr>
              <th scope="col">Date applied</th>
              <th scope="col">Status</th>
              <th scope="col">Company</th>
              <th scope="col">Job title</th>
              <th scope="col">Location</th>
              <th scope="col">Link or source</th>
              <th scope="col">Content</th>
              <th scope="col">Delete</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((record) => {
              const { id, value } = record;
              return (
                <tr
                  key={id}
                  className={
                    saveErrors[id]
                      ? `tracker-row tracker-status--${value.status} tracker-row--error`
                      : `tracker-row tracker-status--${value.status}`
                  }
                >
                  <td>
                    <input
                      aria-label={
                        "Date applied for " +
                        (value.company || value.title || "application")
                      }
                      type="date"
                      value={value.dateApplied}
                      onChange={(event) =>
                        edit(
                          id,
                          (current) => ({
                            ...current,
                            dateApplied: event.target.value,
                          }),
                          0,
                        )
                      }
                    />
                  </td>
                  <td className="tracker-status-cell">
                    <div className="tracker-cell-stack">
                      <select
                        aria-label={
                          "Status for " +
                          (value.company || value.title || "application")
                        }
                        value={value.status}
                        onChange={(event) =>
                          edit(
                            id,
                            (current) => ({
                              ...current,
                              status: event.target.value,
                              customStatus:
                                event.target.value === "other"
                                  ? current.customStatus
                                  : "",
                            }),
                            0,
                          )
                        }
                      >
                        {statuses.map(([key, label]) => (
                          <option key={key} value={key}>
                            {label}
                          </option>
                        ))}
                        {!statuses.some(([key]) => key === value.status) && (
                          <option value={value.status}>{value.status}</option>
                        )}
                      </select>
                      {value.status === "other" && (
                        <input
                          aria-label={
                            "Custom status for " +
                            (value.company || value.title || "application")
                          }
                          maxLength={80}
                          value={value.customStatus}
                          onChange={(event) =>
                            edit(id, (current) => ({
                              ...current,
                              customStatus: event.target.value,
                            }))
                          }
                        />
                      )}
                    </div>
                  </td>
                  <td>
                    <input
                      aria-label={
                        "Company for " +
                        (value.company || value.title || "application")
                      }
                      maxLength={200}
                      value={value.company}
                      onChange={(event) =>
                        edit(id, (current) => ({
                          ...current,
                          company: event.target.value,
                        }))
                      }
                    />
                  </td>
                  <td>
                    <input
                      aria-label={
                        "Job title for " +
                        (value.company || value.title || "application")
                      }
                      maxLength={200}
                      value={value.title}
                      onChange={(event) =>
                        edit(id, (current) => ({
                          ...current,
                          title: event.target.value,
                        }))
                      }
                    />
                  </td>
                  <td>
                    <input
                      aria-label={
                        "Location for " +
                        (value.company || value.title || "application")
                      }
                      maxLength={200}
                      value={value.location}
                      onChange={(event) =>
                        edit(id, (current) => ({
                          ...current,
                          location: event.target.value,
                        }))
                      }
                    />
                  </td>
                  <td>
                    {editingLinkId === id ? (
                      <input
                        autoFocus
                        aria-label={
                          "Edit link for " +
                          (value.company || value.title || "application")
                        }
                        type="text"
                        maxLength={4096}
                        value={value.sourceUrl}
                        onChange={(event) =>
                          edit(id, (current) => ({
                            ...current,
                            sourceUrl: event.target.value,
                          }))
                        }
                        onBlur={() => {
                          setEditingLinkId(null);
                          void persist(id);
                        }}
                        onKeyDown={(event) => {
                          if (event.key === "Enter") event.currentTarget.blur();
                        }}
                      />
                    ) : value.sourceUrl ? (
                      <a
                        className="tracker-source-link"
                        href={trackerLinkTarget(value.sourceUrl) ?? "#"}
                        target="_blank"
                        rel="noopener noreferrer"
                        title="Click to open; double-click to edit"
                        onClick={(event) =>
                          linkClick(event, id, value.sourceUrl)
                        }
                        onDoubleClick={(event) => {
                          event.preventDefault();
                          beginLinkEdit(id);
                        }}
                      >
                        {value.sourceUrl}
                      </a>
                    ) : (
                      <button
                        type="button"
                        className="tracker-link-empty"
                        onClick={() => beginLinkEdit(id)}
                      >
                        Add link
                      </button>
                    )}
                  </td>
                  <td className="tracker-content-cell">
                    {value.resume && (
                      <button
                        type="button"
                        className="button--secondary"
                        onClick={() => void openContent(record, "resume")}
                      >
                        Final resume
                      </button>
                    )}
                    {value.coverLetter && (
                      <button
                        type="button"
                        className="button--secondary"
                        onClick={() => void openContent(record, "cover_letter")}
                      >
                        Cover letter
                      </button>
                    )}
                    {value.answers.length > 0 && (
                      <button
                        type="button"
                        className="button--secondary"
                        onClick={() => void openContent(record, "answers")}
                      >
                        Answers ({value.answers.length})
                      </button>
                    )}
                    {!value.resume &&
                      !value.coverLetter &&
                      !value.answers.length && (
                        <span className="tracker-empty">No saved content</span>
                      )}
                  </td>
                  <td className="tracker-delete-cell">
                    <button
                      type="button"
                      className="button--danger"
                      onClick={() => setDeleteId(id)}
                      aria-label={
                        "Delete " +
                        (value.company || value.title || "application")
                      }
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {!visible.length && (
          <p className="tracker-table-empty">No matching applications.</p>
        )}
      </div>
      {pendingCount > 0 && !Object.keys(saveErrors).length && (
        <p role="status" className="tracker-save-status">
          Saving changes…
        </p>
      )}

      <dialog
        ref={createDialog}
        className="tracker-new-dialog"
        aria-labelledby="tracker-new-title"
        onCancel={(event) => {
          event.preventDefault();
          if (createDirty && !window.confirm("Discard this new application?"))
            return;
          setCreateOpen(false);
          setCreateDraft(null);
        }}
      >
        <div className="tracker-new-dialog-heading">
          <div>
            <p className="application-kicker">Local applications</p>
            <h3 id="tracker-new-title">New application</h3>
          </div>
          <button
            type="button"
            className="button--secondary"
            disabled={creating}
            onClick={() => {
              if (
                createDirty &&
                !window.confirm("Discard this new application?")
              )
                return;
              setCreateOpen(false);
              setCreateDraft(null);
            }}
          >
            Cancel
          </button>
        </div>
        {createDraft && (
          <fieldset disabled={creating} className="tracker-edit-fields">
            <TrackerFields entry={createDraft} onChange={setCreateDraft} />
          </fieldset>
        )}
        <button
          type="button"
          disabled={creating || !createDraft}
          onClick={() => void createEntry()}
        >
          Add application
        </button>
      </dialog>

      <dialog
        ref={deleteDialog}
        className="tracker-confirm-dialog"
        aria-labelledby="tracker-delete-title"
        onCancel={(event) => {
          event.preventDefault();
          if (!deleting) setDeleteId(null);
        }}
      >
        <h3 id="tracker-delete-title">Delete application?</h3>
        <p>
          Delete{" "}
          {deleteRecord?.value.company ||
            deleteRecord?.value.title ||
            "this application"}{" "}
          and its saved content? This cannot be undone.
          {deleteId &&
            desired.current.has(deleteId) &&
            " Pending edits to this row will also be discarded."}
        </p>
        <div className="application-row">
          <button
            type="button"
            className="button--secondary"
            disabled={deleting}
            onClick={() => setDeleteId(null)}
          >
            Cancel
          </button>
          <button
            type="button"
            className="button--danger"
            disabled={deleting}
            onClick={() => void deleteEntry()}
          >
            Delete application
          </button>
        </div>
      </dialog>

      <dialog
        ref={contentDialog}
        className="tracker-content-dialog"
        aria-labelledby="tracker-content-title"
        onCancel={(event) => {
          event.preventDefault();
          closeContent();
        }}
      >
        <div className="tracker-new-dialog-heading">
          <div>
            <p className="application-kicker">Saved application content</p>
            <h3 id="tracker-content-title">
              {content ? contentLabel(content.kind) : "Content"}
            </h3>
          </div>
          <button
            type="button"
            className="button--secondary"
            onClick={() => {
              closeContent();
            }}
          >
            Close
          </button>
        </div>
        {content?.kind === "answers" && (
          <ol className="tracker-answer-view">
            {contentRecord?.value.answers.map((answer, index) => (
              <li key={index}>
                <strong>{answer.question}</strong>
                <p>{answer.answer}</p>
              </li>
            ))}
          </ol>
        )}
        {content?.kind === "cover_letter" &&
          contentRecord?.value.coverLetter && (
            <p className="tracker-cover-view">
              {contentRecord.value.coverLetter}
            </p>
          )}
        {contentLoading && <p role="status">Opening PDF…</p>}
        {contentError && <p role="alert">{contentError}</p>}
        {pdf && !contentLoading && (
          <div className="application-preview">
            <h4>{pdf.filename}</h4>
            <PdfCanvas base64={pdf.base64} />
          </div>
        )}
      </dialog>
    </section>
  );
}
