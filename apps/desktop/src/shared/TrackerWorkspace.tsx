import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import type { ContactDetails, ResumeDocument } from "@ort/contracts/resume";
import { PdfCanvas, ResumeFields } from "./ApplicationViews";
import { sanitizeCaptureUrl } from "./capture-url";

export type TrackerEntry = {
  company: string;
  title: string;
  location: string;
  dateApplied: string;
  status: string;
  customStatus: string;
  sourceUrl: string;
  resume: ResumeDocument | null;
  coverLetter: string | null;
  coverContact: ContactDetails | null;
  answers: { question: string; answer: string }[];
  style: "technical" | "professional" | "modern" | "plain";
};
type RecordEntry = { id: string; revision: number; value: TrackerEntry };
type Response<T> =
  | { ok: true; value: T }
  | { ok: false; error: { code: string } };

export const emptyTrackerEntry = (): TrackerEntry => ({
  company: "",
  title: "",
  location: "",
  dateApplied: "",
  status: "applied",
  customStatus: "",
  sourceUrl: "",
  resume: null,
  coverLetter: null,
  coverContact: null,
  answers: [],
  style: "technical",
});

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

export function TrackerFields({
  entry,
  onChange,
}: {
  entry: TrackerEntry;
  onChange: (entry: TrackerEntry) => void;
}) {
  function edit<K extends keyof TrackerEntry>(key: K, value: TrackerEntry[K]) {
    onChange({ ...entry, [key]: value });
  }
  return (
    <div className="tracker-fields">
      <label>
        Company
        <input
          maxLength={200}
          value={entry.company}
          onChange={(event) => edit("company", event.target.value)}
        />
      </label>
      <label>
        Job title
        <input
          maxLength={200}
          value={entry.title}
          onChange={(event) => edit("title", event.target.value)}
        />
      </label>
      <label>
        Location
        <input
          maxLength={200}
          value={entry.location}
          onChange={(event) => edit("location", event.target.value)}
        />
      </label>
      <label>
        Date applied
        <input
          type="date"
          value={entry.dateApplied}
          onChange={(event) => edit("dateApplied", event.target.value)}
        />
      </label>
      <label>
        Status
        <select
          value={entry.status}
          onChange={(event) =>
            onChange({
              ...entry,
              status: event.target.value,
              customStatus:
                event.target.value === "other" ? entry.customStatus : "",
            })
          }
        >
          {statuses.map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
          {!statuses.some(([value]) => value === entry.status) && (
            <option value={entry.status}>{entry.status}</option>
          )}
        </select>
      </label>
      {entry.status === "other" && (
        <label>
          Custom status
          <input
            maxLength={80}
            value={entry.customStatus}
            onChange={(event) => edit("customStatus", event.target.value)}
          />
        </label>
      )}
      <label>
        Source URL
        <input
          type="url"
          maxLength={4096}
          value={entry.sourceUrl}
          onChange={(event) => edit("sourceUrl", event.target.value)}
          onBlur={(event) => {
            try {
              edit("sourceUrl", sanitizeCaptureUrl(event.target.value));
            } catch {
              // Keep the input visible for correction.
            }
          }}
        />
      </label>
    </div>
  );
}

function csvCell(value: string) {
  const safe = /^[\s\u0000-\u001f]*[=+@-]/u.test(value) ? `'${value}` : value;
  return `"${safe.replaceAll('"', '""')}"`;
}

export function TrackerWorkspace({
  active,
  publishedResume,
  onDirtyChange,
}: {
  active: boolean;
  publishedResume: ResumeDocument | null;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const [entries, setEntries] = useState<RecordEntry[]>([]);
  const [selected, setSelected] = useState<RecordEntry | null>(null);
  const [draft, setDraft] = useState<TrackerEntry | null>(null);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [pdf, setPdf] = useState<{ base64: string; filename: string } | null>(
    null,
  );
  const [editingResume, setEditingResume] = useState(false);
  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    void command<RecordEntry[]>("list_tracker_entries")
      .then((loaded) => {
        if (!cancelled) setEntries(loaded);
      })
      .catch((error: unknown) => {
        if (!cancelled) setNotice(`Could not load tracker (${String(error)}).`);
      });
    return () => {
      cancelled = true;
    };
  }, [active]);
  const visible = useMemo(
    () =>
      entries.filter(({ value }) => {
        const term = search.trim().toLocaleLowerCase();
        return (
          (!filter || value.status === filter) &&
          (!term ||
            [value.company, value.title, value.location, value.status].some(
              (part) => part.toLocaleLowerCase().includes(term),
            ))
        );
      }),
    [entries, search, filter],
  );
  const dirty =
    !!draft &&
    JSON.stringify(selected?.value ?? emptyTrackerEntry()) !==
      JSON.stringify(draft);
  useEffect(() => onDirtyChange(dirty), [dirty, onDirtyChange]);
  function mayReplaceDraft() {
    return !dirty || window.confirm("Discard unsaved tracker edits?");
  }

  async function save() {
    if (!draft || busy) return;
    setBusy(true);
    setNotice("");
    try {
      const record = await command<RecordEntry>("save_tracker_entry", {
        id: selected?.id ?? null,
        expectedRevision: selected?.revision ?? null,
        entry: draft,
      });
      setEntries((current) => [
        record,
        ...current.filter((item) => item.id !== record.id),
      ]);
      setSelected(record);
      setDraft(record.value);
      setPdf(null);
      setNotice("Tracker entry saved.");
    } catch (error) {
      setNotice(`Save failed (${String(error)}). Your edits are still here.`);
    } finally {
      setBusy(false);
    }
  }
  async function remove() {
    if (
      !selected ||
      busy ||
      !window.confirm("Delete this tracker entry and its retained materials?")
    )
      return;
    setBusy(true);
    try {
      await command<boolean>("delete_tracker_entry", {
        id: selected.id,
        expectedRevision: selected.revision,
      });
      setEntries((current) =>
        current.filter((item) => item.id !== selected.id),
      );
      setSelected(null);
      setDraft(null);
      setNotice("Tracker entry deleted.");
    } catch (error) {
      setNotice(`Delete failed (${String(error)}).`);
    } finally {
      setBusy(false);
    }
  }
  async function preview(kind: "resume" | "cover_letter") {
    if (!selected || dirty) return;
    setBusy(true);
    try {
      setPdf(
        await command<{ base64: string; filename: string }>(
          "preview_tracker_pdf",
          { id: selected.id, expectedRevision: selected.revision, kind },
        ),
      );
    } catch (error) {
      setNotice(`Preview failed (${String(error)}).`);
    } finally {
      setBusy(false);
    }
  }
  function exportCsv() {
    const rows = [
      [
        "Company",
        "Job title",
        "Location",
        "Date applied",
        "Status",
        "Source URL",
      ],
      ...visible.map(({ value }) => [
        value.company,
        value.title,
        value.location,
        value.dateApplied,
        value.status === "other" ? value.customStatus : value.status,
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
    setTimeout(() => URL.revokeObjectURL(url), 0);
  }
  function exportAnswers() {
    if (!draft?.answers.length) return;
    const text = draft.answers
      .map((item, index) => `${index + 1}. ${item.question}\n${item.answer}`)
      .join("\n\n");
    const url = URL.createObjectURL(
      new Blob([text], { type: "text/plain;charset=utf-8" }),
    );
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "application-answers.txt";
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  }

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
              if (!mayReplaceDraft()) return;
              setSelected(null);
              setDraft(emptyTrackerEntry());
              setPdf(null);
              setEditingResume(false);
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
      <div className="tracker-filters">
        <label>
          Search
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Company, title, location"
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
      <div className="tracker-layout">
        <div className="tracker-list">
          {visible.length === 0 && <p>No matching applications.</p>}
          {visible.map((record) => (
            <button
              type="button"
              key={record.id}
              className={
                selected?.id === record.id
                  ? "tracker-item selected"
                  : "tracker-item"
              }
              onClick={() => {
                if (!mayReplaceDraft()) return;
                setSelected(record);
                setDraft(record.value);
                setPdf(null);
                setEditingResume(false);
                setNotice("");
              }}
            >
              <strong>{record.value.company || "Untitled company"}</strong>
              <span>{record.value.title || "Untitled role"}</span>
              <small>
                {record.value.dateApplied} ·{" "}
                {record.value.customStatus || record.value.status}
              </small>
            </button>
          ))}
        </div>
        {draft && (
          <div className="tracker-detail">
            <h3>{selected ? "Edit application" : "New application"}</h3>
            <fieldset disabled={busy} className="tracker-edit-fields">
              <TrackerFields entry={draft} onChange={setDraft} />
              {draft.resume && (
                <p>
                  Retained tailored resume snapshot · {draft.resume.title}{" "}
                  {selected && (
                    <button
                      type="button"
                      disabled={dirty}
                      onClick={() => void preview("resume")}
                    >
                      Open PDF
                    </button>
                  )}
                </p>
              )}
              {draft.coverLetter && selected && (
                <button
                  type="button"
                  disabled={dirty}
                  onClick={() => void preview("cover_letter")}
                >
                  Open cover letter PDF
                </button>
              )}
              {pdf && !dirty && (
                <div className="application-preview">
                  <h4>{pdf.filename}</h4>
                  <PdfCanvas base64={pdf.base64} />
                </div>
              )}
              {draft.coverLetter && (
                <details>
                  <summary>Retained cover letter</summary>
                  <label>
                    Review and edit cover letter
                    <textarea
                      rows={12}
                      maxLength={12000}
                      value={draft.coverLetter}
                      onChange={(event) =>
                        setDraft({ ...draft, coverLetter: event.target.value })
                      }
                    />
                  </label>
                  <button
                    type="button"
                    onClick={() =>
                      setDraft({
                        ...draft,
                        coverLetter: null,
                        coverContact: null,
                      })
                    }
                  >
                    Remove cover letter
                  </button>
                </details>
              )}
              {draft.answers.length > 0 && (
                <details>
                  <summary>Approved answers ({draft.answers.length})</summary>
                  <ol>
                    {draft.answers.map((answer, index) => (
                      <li key={index}>
                        <label>
                          Question {index + 1}
                          <textarea
                            maxLength={2000}
                            value={answer.question}
                            onChange={(event) =>
                              setDraft({
                                ...draft,
                                answers: draft.answers.map((item, i) =>
                                  i === index
                                    ? { ...item, question: event.target.value }
                                    : item,
                                ),
                              })
                            }
                          />
                        </label>
                        <label>
                          Answer
                          <textarea
                            maxLength={4000}
                            value={answer.answer}
                            onChange={(event) =>
                              setDraft({
                                ...draft,
                                answers: draft.answers.map((item, i) =>
                                  i === index
                                    ? { ...item, answer: event.target.value }
                                    : item,
                                ),
                              })
                            }
                          />
                        </label>
                        <button
                          type="button"
                          onClick={() =>
                            void navigator.clipboard
                              .writeText(answer.answer)
                              .catch(() =>
                                setNotice(
                                  "Copy failed. Select and copy the answer text manually.",
                                ),
                              )
                          }
                        >
                          Copy answer
                        </button>
                        <button
                          type="button"
                          onClick={() =>
                            setDraft({
                              ...draft,
                              answers: draft.answers.filter(
                                (_, item) => item !== index,
                              ),
                            })
                          }
                        >
                          Remove answer
                        </button>
                      </li>
                    ))}
                  </ol>
                  <button
                    type="button"
                    className="button--secondary"
                    onClick={exportAnswers}
                  >
                    Export answers as text
                  </button>
                </details>
              )}
              {draft.resume && (
                <button
                  type="button"
                  className="button--secondary"
                  onClick={() => setEditingResume(!editingResume)}
                >
                  {editingResume
                    ? "Close resume editor"
                    : "Edit retained resume"}
                </button>
              )}
              {draft.resume && editingResume && (
                <ResumeFields
                  document={draft.resume}
                  onChange={(resume) => setDraft({ ...draft, resume })}
                />
              )}
              {publishedResume && (
                <button
                  type="button"
                  className="button--secondary"
                  onClick={() => {
                    if (
                      window.confirm(
                        "Replace this entry's retained resume with the current published master?",
                      )
                    ) {
                      setDraft({ ...draft, resume: publishedResume });
                      setEditingResume(false);
                      setPdf(null);
                    }
                  }}
                >
                  Replace resume with published master
                </button>
              )}
              {draft.resume && (
                <button
                  type="button"
                  className="button--secondary"
                  onClick={() => setDraft({ ...draft, resume: null })}
                >
                  Remove retained resume
                </button>
              )}
            </fieldset>
            <div className="application-row">
              <button type="button" disabled={busy} onClick={() => void save()}>
                Save entry
              </button>
              {selected && (
                <button
                  type="button"
                  className="button--secondary"
                  disabled={busy}
                  onClick={() => void remove()}
                >
                  Delete entry
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
