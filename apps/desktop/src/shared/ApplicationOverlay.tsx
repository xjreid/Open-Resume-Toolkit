import { invoke } from "@tauri-apps/api/core";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { emitTo } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useRef, useState } from "react";
import type { ResumeDocument } from "@ort/contracts/resume";
import { Brand } from "./AppShell";
import { PdfCanvas, ResumeFields } from "./ApplicationViews";
import { sanitizeCaptureUrl } from "./capture-url";
import {
  TrackerFields,
  emptyTrackerEntry,
  type TrackerEntry,
} from "./TrackerFields";

type Tab = "resume" | "cover" | "answers";
type MaterialKind = "resume" | "cover_letter";
type Alert = {
  id: string;
  kind: "not_found" | "confirmed_mismatch";
  category: string;
  requirement: string;
  jobExcerpt: string;
  resumeEvidence: { fieldId: string; value: string } | null;
};
type Workspace = {
  schemaVersion: number;
  publishedRevision: number;
  jobDescription: string;
  jobUrl: string;
  roleInfo: { company: string; title: string; location: string };
  resume: ResumeDocument;
  changePoints: string[];
  alerts: Alert[];
  alertsTruncated: boolean;
  dismissedAlertIds: string[];
  ignoreAllAlerts: boolean;
  coverLetter: string | null;
  question: string;
  answer: string;
  approvedAnswers: { question: string; answer: string }[];
  style: "technical" | "professional" | "modern" | "plain";
};
type Saved = { revision: number; workspace: Workspace };
type StageOne = {
  jobDescription: string;
  jobUrl: string;
  style: Workspace["style"];
};
type SavedStageOne = { revision: number; draft: StageOne };
type SavedPendingCapture = {
  revision: number;
  capture: {
    requestId: string;
    payload: {
      text: string;
      url: string;
      title: string;
      target: "job" | "question";
    };
  };
};
type Context = { publishedRevision: number | null; aiLabel: string };
type Response<T> =
  | { ok: true; value: T }
  | { ok: false; error: { code: string; messageKey: string } };

async function command<T>(
  name: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  const result = await invoke<Response<T>>(name, args);
  if (!result.ok) throw new Error(result.error.code);
  return result.value;
}

const errors: Record<string, string> = {
  PUBLISHED_RESUME_REQUIRED:
    "Publish a master resume in the main window before continuing.",
  AI_DISABLED: "Set up a Direct AI key in the main window before continuing.",
  AI_CAP_REJECTED: "This request would exceed your AI spending cap.",
  AI_OUTPUT_INVALID:
    "The provider returned incomplete or unusable material, so nothing was saved. Try again or shorten the job description.",
  AI_AUTHENTICATION_FAILED:
    "The provider rejected the active API key. Check the key and its permissions in My Keys.",
  AI_RATE_LIMITED:
    "The provider rate-limited this request. Check its usage limits before retrying.",
  AI_PROVIDER_SERVICE_UNAVAILABLE:
    "The provider is temporarily unavailable. Try again later.",
  AI_PROVIDER_TEMPORARY:
    "The provider returned a temporary server error. Try again later.",
  AI_PROVIDER_FAILED:
    "The provider rejected the request. Check the active key, model, and Monitoring for the attempt category.",
  AI_BUSY: "Another AI request is in progress.",
  AI_CANCELLED: "The request was cancelled.",
  AI_INPUT_TOO_LARGE:
    "The reviewed job and resume exceed the selected model's input limit.",
  PERSONAL_ANSWER_REQUIRED:
    "This question requires your personal answer. ORT will not draft an attestation.",
  REVISION_CONFLICT:
    "This workspace changed in another window. Reload before saving.",
  JOB_URL_INVALID:
    "Enter an http or https page URL, or leave the source blank.",
  JOB_INVALID:
    "Review the job text and trim it to 20,000 characters before tailoring.",
  PDF_UNAVAILABLE:
    "This material could not be rendered as a PDF. Review its length and characters.",
  EXPORT_CANCELLED: "Download cancelled.",
  DRAG_UNAVAILABLE:
    "File drag is unavailable. Use Download and select the saved PDF on the application site.",
};
function message(error: unknown): string {
  const code = error instanceof Error ? error.message : "UNKNOWN";
  return (
    errors[code] ??
    `The action could not be completed (${code}). Your workspace is unchanged.`
  );
}

export function ApplicationOverlay() {
  const [context, setContext] = useState<Context | null>(null);
  const [saved, setSaved] = useState<Saved | null>(null);
  const [draft, setDraft] = useState<Workspace | null>(null);
  const [job, setJob] = useState("");
  const [jobUrl, setJobUrl] = useState("");
  const [stageOneReady, setStageOneReady] = useState(false);
  const [pendingCapture, setPendingCapture] =
    useState<SavedPendingCapture | null>(null);
  const [captureText, setCaptureText] = useState("");
  const [captureUrl, setCaptureUrl] = useState("");
  const [stageOneStatus, setStageOneStatus] = useState<
    "saved" | "saving" | "error"
  >("saved");
  const stageOneRevision = useRef<number | null>(null);
  const stageOneSaved = useRef<StageOne | null>(null);
  const stageOnePending = useRef<StageOne | null>(null);
  const stageOneFlush = useRef<Promise<void> | null>(null);
  const [style, setStyle] = useState<Workspace["style"]>("technical");
  const [tab, setTab] = useState<Tab>("resume");
  const [busy, setBusy] = useState(false);
  const [closePending, setClosePending] = useState(false);
  const closeDirty = useRef(true);
  const [notice, setNotice] = useState("");
  const [instruction, setInstruction] = useState("");
  const [coverInstruction, setCoverInstruction] = useState("");
  const [question, setQuestion] = useState("");
  const [limit, setLimit] = useState("");
  const [preview, setPreview] = useState<{
    kind: MaterialKind;
    base64: string;
    filename: string;
  } | null>(null);
  const [editing, setEditing] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [finishing, setFinishing] = useState(false);
  const [tracking, setTracking] = useState<TrackerEntry>(emptyTrackerEntry);
  const [retainResume, setRetainResume] = useState(true);
  const [retainCover, setRetainCover] = useState(true);
  const [retainAnswers, setRetainAnswers] = useState(true);
  const [resetting, setResetting] = useState(false);
  const dirty =
    !!saved &&
    !!draft &&
    JSON.stringify(saved.workspace) !== JSON.stringify(draft);
  const jobBytes = new TextEncoder().encode(job).length;
  const jobReady =
    job.trim().length > 0 && job.length <= 20_000 && jobBytes <= 128 * 1_024;
  const captureReady =
    captureText.trim().length > 0 &&
    new TextEncoder().encode(captureText).length <= 128 * 1_024 &&
    (pendingCapture?.capture.payload.target !== "question" ||
      captureText.length <= 2_000);
  const savedReview = stageOneSaved.current;
  closeDirty.current =
    busy ||
    dirty ||
    !stageOneReady ||
    (!saved &&
      (stageOneStatus !== "saved" ||
        job !== (savedReview?.jobDescription ?? "") ||
        jobUrl !== (savedReview?.jobUrl ?? "") ||
        style !== (savedReview?.style ?? "technical"))) ||
    !!instruction.trim() ||
    !!coverInstruction.trim() ||
    !!limit.trim() ||
    finishing ||
    resetting ||
    (pendingCapture !== null &&
      (captureText !== pendingCapture.capture.payload.text ||
        captureUrl !== pendingCapture.capture.payload.url));

  useEffect(() => {
    let active = true;
    const window = getCurrentWebviewWindow();
    const subscription = Promise.all([
      window.listen<{ attempt: string }>("ort:overlay-close-probe", (event) => {
        if (!active || typeof event.payload?.attempt !== "string") return;
        setClosePending(true);
        void emitTo("main", "ort:overlay-close-reply", {
          attempt: event.payload.attempt,
          dirty: closeDirty.current,
        });
      }),
      window.listen("ort:overlay-close-cancelled", () => {
        if (active) setClosePending(false);
      }),
    ]).catch(() => []);
    return () => {
      active = false;
      void subscription.then((unlisten) => unlisten.forEach((stop) => stop()));
    };
  }, []);

  useEffect(() => {
    setCaptureText(pendingCapture?.capture.payload.text ?? "");
    setCaptureUrl(pendingCapture?.capture.payload.url ?? "");
  }, [pendingCapture?.capture.requestId]);

  useEffect(() => {
    void Promise.all([
      command<Context>("application_context"),
      command<Saved | null>("load_application_workspace"),
      command<SavedStageOne | null>("load_application_stage_one"),
      command<SavedPendingCapture | null>("load_application_capture"),
    ])
      .then(([nextContext, current, stageOne, capture]) => {
        setContext(nextContext);
        setSaved(current);
        setDraft(current?.workspace ?? null);
        setQuestion(current?.workspace.question ?? "");
        stageOneRevision.current = stageOne?.revision ?? null;
        stageOneSaved.current = stageOne?.draft ?? null;
        if (!current && stageOne) {
          setJob(stageOne.draft.jobDescription);
          setJobUrl(stageOne.draft.jobUrl);
          setStyle(stageOne.draft.style);
        }
        setStageOneReady(true);
        setPendingCapture(capture);
      })
      .catch((error: unknown) => setNotice(message(error)));
  }, []);

  useEffect(() => {
    let active = true;
    const refreshContext = () => {
      void Promise.all([
        command<SavedPendingCapture | null>("load_application_capture"),
        command<Context>("application_context"),
      ])
        .then(([capture, nextContext]) => {
          if (active) {
            setPendingCapture(capture);
            setContext(nextContext);
          }
        })
        .catch((error: unknown) => {
          if (active) setNotice(message(error));
        });
    };
    window.addEventListener("focus", refreshContext);
    const subscription = getCurrentWebviewWindow()
      .listen("ort:browser-capture", refreshContext)
      .catch(() => () => {});
    return () => {
      active = false;
      window.removeEventListener("focus", refreshContext);
      void subscription.then((unlisten) => unlisten());
    };
  }, []);

  async function flushStageOne(): Promise<void> {
    if (stageOneFlush.current) {
      await stageOneFlush.current;
      if (stageOnePending.current) await flushStageOne();
      return;
    }
    const operation = (async () => {
      while (stageOnePending.current) {
        const next = stageOnePending.current;
        stageOnePending.current = null;
        setStageOneStatus("saving");
        try {
          const savedDraft = await command<SavedStageOne>(
            "save_application_stage_one",
            {
              expectedRevision: stageOneRevision.current,
              draft: next,
            },
          );
          stageOneRevision.current = savedDraft.revision;
          stageOneSaved.current = savedDraft.draft;
          setStageOneStatus("saved");
        } catch (error) {
          stageOnePending.current ??= next;
          setStageOneStatus("error");
          throw error;
        }
      }
    })();
    stageOneFlush.current = operation;
    try {
      await operation;
    } finally {
      if (stageOneFlush.current === operation) stageOneFlush.current = null;
    }
    if (stageOnePending.current) await flushStageOne();
  }

  useEffect(() => {
    if (!stageOneReady || saved) return;
    if (
      !job &&
      !jobUrl &&
      style === "technical" &&
      stageOneRevision.current === null
    )
      return;
    stageOnePending.current = { jobDescription: job, jobUrl, style };
    setStageOneStatus("saving");
    const timer = window.setTimeout(() => {
      void flushStageOne().catch((error: unknown) => setNotice(message(error)));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [job, jobUrl, style, stageOneReady, saved]);

  function apply(current: Saved) {
    setSaved(current);
    setDraft(current.workspace);
    setQuestion(current.workspace.question);
    setPreview(null);
    setNotice("");
  }
  function update(change: (current: Workspace) => Workspace) {
    setDraft((current) => (current ? change(current) : current));
  }
  async function run(action: () => Promise<void>) {
    setBusy(true);
    setNotice("");
    try {
      await action();
    } catch (error) {
      setNotice(message(error));
    } finally {
      setBusy(false);
    }
  }
  function save() {
    if (!saved || !draft) return;
    const previewKind = preview?.kind;
    void run(async () => {
      const updated = await command<Saved>("save_application_workspace", {
        expectedRevision: saved.revision,
        workspace: draft,
      });
      apply(updated);
      if (previewKind) {
        try {
          const pdf = await command<{ base64: string; filename: string }>(
            "preview_application_pdf",
            { expectedRevision: updated.revision, kind: previewKind },
          );
          setPreview({ kind: previewKind, ...pdf });
        } catch {
          setNotice(
            "Edits saved, but the PDF preview could not refresh. Select Preview and edit to try again.",
          );
        }
      }
    });
  }
  function generateResume() {
    if (!saved || dirty || !instruction.trim()) return;
    void run(async () => {
      apply(
        await command<Saved>("regenerate_application_resume", {
          expectedRevision: saved.revision,
          correctionInstruction: instruction.trim(),
        }),
      );
      setInstruction("");
    });
  }
  function generateCover() {
    if (!saved || dirty) return;
    void run(async () =>
      apply(
        await command<Saved>("generate_application_cover_letter", {
          expectedRevision: saved.revision,
          instruction: coverInstruction.trim(),
        }),
      ),
    );
  }
  function generateAnswer() {
    if (!saved || dirty || !question.trim()) return;
    const parsed = limit.trim() ? Number(limit) : null;
    if (
      parsed !== null &&
      (!Number.isInteger(parsed) || parsed < 1 || parsed > 4000)
    ) {
      setNotice("Enter a character limit from 1 to 4000.");
      return;
    }
    void run(async () =>
      apply(
        await command<Saved>("generate_application_answer", {
          expectedRevision: saved.revision,
          question: question.trim(),
          limit: parsed,
        }),
      ),
    );
  }
  function showPdf(kind: MaterialKind) {
    if (!saved || dirty) return;
    void run(async () => {
      setPreview({
        kind,
        ...(await command<{ base64: string; filename: string }>(
          "preview_application_pdf",
          { expectedRevision: saved.revision, kind },
        )),
      });
      setEditing(true);
      const width = Math.max(
        680,
        Math.min(1120, Math.floor(screen.availWidth * 0.85)),
      );
      const height = Math.max(
        680,
        Math.min(820, Math.floor(screen.availHeight * 0.85)),
      );
      try {
        await getCurrentWebviewWindow().setSize(new LogicalSize(width, height));
        setExpanded(true);
      } catch {
        setNotice(
          "PDF preview is ready. Resize the window manually for a larger editor.",
        );
      }
    });
  }
  function compactWindow() {
    void getCurrentWebviewWindow()
      .setSize(new LogicalSize(680, 680))
      .then(() => setExpanded(false))
      .catch(() =>
        setNotice(
          "The window could not be resized. Drag its edge to make it smaller.",
        ),
      );
  }
  function downloadPdf(kind: MaterialKind) {
    if (!saved || dirty) return;
    void run(async () => {
      await command<boolean>("download_application_pdf", {
        expectedRevision: saved.revision,
        kind,
      });
      setNotice("PDF downloaded.");
    });
  }
  function dragPdf(kind: MaterialKind) {
    if (!saved || dirty || busy) return;
    void command<boolean>("drag_application_pdf", {
      expectedRevision: saved.revision,
      kind,
    }).catch((error: unknown) => setNotice(message(error)));
  }

  function finish(selection: boolean) {
    if (!saved || dirty) return;
    void run(async () => {
      await command<boolean>("finish_application", {
        expectedRevision: saved.revision,
        selection: selection
          ? {
              entry: tracking,
              retainResume,
              retainCoverLetter: retainCover,
              retainAnswers,
            }
          : null,
      });
      setSaved(null);
      setDraft(null);
      setPreview(null);
      setFinishing(false);
      stageOneRevision.current = null;
      stageOneSaved.current = null;
      stageOnePending.current = null;
      setStageOneStatus("saved");
      setJob("");
      setJobUrl("");
      setStyle("technical");
      setTracking(emptyTrackerEntry());
      if (expanded) compactWindow();
    });
  }

  function resolveCapture(accept: boolean) {
    if (!pendingCapture) return;
    const capture = pendingCapture;
    void run(async () => {
      if (
        accept &&
        capture.capture.payload.target === "job" &&
        !saved &&
        (job || jobUrl || stageOneRevision.current !== null)
      ) {
        stageOnePending.current = { jobDescription: job, jobUrl, style };
        await flushStageOne();
      }
      await command<boolean>("resolve_application_capture", {
        requestId: capture.capture.requestId,
        accept,
        reviewedText: accept ? captureText : null,
        reviewedUrl: accept ? sanitizeCaptureUrl(captureUrl) : null,
        expectedRevision: accept
          ? capture.capture.payload.target === "job"
            ? stageOneRevision.current
            : (saved?.revision ?? null)
          : null,
      });
      setPendingCapture(null);
      if (!accept) return;
      if (capture.capture.payload.target === "job") {
        const updated = await command<SavedStageOne | null>(
          "load_application_stage_one",
        );
        stageOneRevision.current = updated?.revision ?? null;
        stageOneSaved.current = updated?.draft ?? null;
        setJob(updated?.draft.jobDescription ?? "");
        setJobUrl(updated?.draft.jobUrl ?? "");
        setStyle(updated?.draft.style ?? "technical");
        setStageOneStatus("saved");
      } else {
        const updated = await command<Saved | null>(
          "load_application_workspace",
        );
        if (updated) apply(updated);
        setTab("answers");
      }
    });
  }

  return (
    <main className="application-shell" inert={closePending}>
      <header className="application-header">
        <Brand />
        <div>
          <strong>{context?.aiLabel ?? "Checking AI"}</strong>
          <span>Browser capture unavailable in unsigned preview</span>
        </div>
        {expanded && (
          <button
            type="button"
            className="application-secondary"
            onClick={compactWindow}
          >
            Compact window
          </button>
        )}
        {saved && (
          <button
            type="button"
            onClick={() => {
              setTracking({
                ...emptyTrackerEntry(),
                company: draft?.roleInfo?.company ?? "",
                title: draft?.roleInfo?.title ?? "",
                location: draft?.roleInfo?.location ?? "",
                sourceUrl: draft?.jobUrl ?? "",
              });
              setFinishing(true);
            }}
            disabled={busy || closePending}
          >
            Finish Application
          </button>
        )}
      </header>
      <div className="application-content">
        {notice && (
          <p className="application-notice" role="alert">
            {notice}
          </p>
        )}
        {busy && (
          <button
            type="button"
            className="application-secondary"
            onClick={() =>
              void command<boolean>("cancel_application_generation").catch(
                () => {},
              )
            }
          >
            Cancel AI request
          </button>
        )}
        <fieldset
          className="application-workspace-fields"
          disabled={busy || closePending}
        >
          {!saved ? (
            <section className="application-panel">
              <p className="application-kicker">Stage 1 · Review</p>
              <h1>Prepare application materials</h1>
              <p>
                Paste the job description and review it before sending it to
                your selected Direct AI provider.
              </p>
              <label>
                Reviewed job description
                <textarea
                  value={job}
                  onChange={(event) => setJob(event.target.value)}
                  rows={14}
                  maxLength={131072}
                />
              </label>
              {job.length > 20_000 && (
                <p className="application-note" role="status">
                  Trim the reviewed job to 20,000 characters before tailoring.
                </p>
              )}
              {jobBytes > 128 * 1_024 && (
                <p className="application-note" role="alert">
                  The reviewed job exceeds the 128 KiB local capture limit.
                </p>
              )}
              <label>
                Source URL (optional)
                <input
                  type="url"
                  value={jobUrl}
                  onChange={(event) => setJobUrl(event.target.value)}
                  maxLength={4096}
                />
              </label>
              <label>
                Resume design
                <select
                  value={style}
                  onChange={(event) =>
                    setStyle(event.target.value as Workspace["style"])
                  }
                >
                  <option value="technical">Technical</option>
                  <option value="professional">Business</option>
                  <option value="modern">Modern</option>
                </select>
              </label>
              <p className="application-note" aria-live="polite">
                {stageOneStatus === "saving"
                  ? "Saving reviewed job locally…"
                  : stageOneStatus === "error"
                    ? "Reviewed job could not be saved. Try again before continuing."
                    : "Reviewed job saved locally."}
              </p>
              <p className="application-note">
                {context?.publishedRevision
                  ? `Published master revision ${context.publishedRevision} will be used.`
                  : "Publish a master resume in the main window first."}
              </p>
              <button
                type="button"
                disabled={
                  busy ||
                  !stageOneReady ||
                  !jobReady ||
                  !context?.publishedRevision ||
                  context.aiLabel === "AI not configured"
                }
                onClick={() =>
                  void run(async () => {
                    stageOnePending.current = {
                      jobDescription: job,
                      jobUrl,
                      style,
                    };
                    await flushStageOne();
                    apply(
                      await command<Saved>("start_application", {
                        jobDescription: job.trim(),
                        jobUrl: sanitizeCaptureUrl(jobUrl),
                        style,
                      }),
                    );
                    setTab("resume");
                  })
                }
              >
                {busy ? "Working…" : "Continue and tailor resume"}
              </button>
            </section>
          ) : (
            draft && (
              <>
                <nav
                  className="application-tabs"
                  aria-label="Application materials"
                >
                  {(["resume", "cover", "answers"] as Tab[]).map((item) => (
                    <button
                      key={item}
                      type="button"
                      className={tab === item ? "selected" : ""}
                      aria-current={tab === item ? "page" : undefined}
                      onClick={() => {
                        setTab(item);
                        setPreview(null);
                        setEditing(false);
                      }}
                    >
                      {item === "cover"
                        ? "Cover letter"
                        : item === "answers"
                          ? "Answers"
                          : "Resume"}
                    </button>
                  ))}
                </nav>
                {dirty && (
                  <div className="application-savebar">
                    <span>
                      Unsaved edits · save before generation, preview, or
                      download.
                    </span>
                    <button type="button" disabled={busy} onClick={save}>
                      Save edits
                    </button>
                  </div>
                )}
                {tab === "resume" && (
                  <section className="application-panel">
                    <p className="application-kicker">Stage 2 · Resume</p>
                    <h1>Tailored resume</h1>
                    <div className="application-row">
                      <label>
                        Company from job description
                        <input
                          maxLength={200}
                          value={draft.roleInfo?.company ?? ""}
                          onChange={(event) =>
                            update((current) => ({
                              ...current,
                              roleInfo: {
                                ...current.roleInfo,
                                company: event.target.value,
                              },
                            }))
                          }
                        />
                      </label>
                      <label>
                        Role title
                        <input
                          maxLength={200}
                          value={draft.roleInfo?.title ?? ""}
                          onChange={(event) =>
                            update((current) => ({
                              ...current,
                              roleInfo: {
                                ...current.roleInfo,
                                title: event.target.value,
                              },
                            }))
                          }
                        />
                      </label>
                      <label>
                        Location
                        <input
                          maxLength={200}
                          value={draft.roleInfo?.location ?? ""}
                          onChange={(event) =>
                            update((current) => ({
                              ...current,
                              roleInfo: {
                                ...current.roleInfo,
                                location: event.target.value,
                              },
                            }))
                          }
                        />
                      </label>
                    </div>
                    <p className="application-note">Review these details before saving them to the tracker.</p>
                    {draft.changePoints.length > 0 && (
                      <div>
                        <h3>Tailoring notes</h3>
                        <ul aria-label="Tailoring notes">
                          {draft.changePoints.map((point, index) => (
                            <li key={index}>{point}</li>
                          ))}
                        </ul>
                      </div>
                    )}
                    <p aria-live="polite" className="application-note">
                      {draft.alerts.length === 0
                        ? "No validated Required Qualification Alerts."
                        : `${draft.alerts.length} validated Required Qualification Alert${draft.alerts.length === 1 ? "" : "s"}.`}
                    </p>
                    {(draft.alerts.length > 0 || draft.alertsTruncated) && (
                      <div className="application-alerts">
                        {draft.alertsTruncated && (
                          <p>
                            More alert candidates were returned than this
                            workspace can show. Review the job description
                            directly for any remaining required qualifications.
                          </p>
                        )}
                        <div className="application-row">
                          <h2>Required Qualification Alerts</h2>
                          <button
                            type="button"
                            className="application-secondary"
                            onClick={() =>
                              update((current) => ({
                                ...current,
                                ignoreAllAlerts: !current.ignoreAllAlerts,
                              }))
                            }
                          >
                            {draft.ignoreAllAlerts
                              ? "Show alerts"
                              : "Ignore all"}
                          </button>
                        </div>
                        {!draft.ignoreAllAlerts &&
                          draft.alerts
                            .filter(
                              (alert) =>
                                !draft.dismissedAlertIds.includes(alert.id),
                            )
                            .map((alert) => (
                              <article
                                key={alert.id}
                                className="application-alert"
                              >
                                <strong>{alert.requirement}</strong>
                                <p>
                                  {alert.kind === "not_found"
                                    ? "Not found in your published resume"
                                    : "Confirmed mismatch"}
                                </p>
                                <blockquote>{alert.jobExcerpt}</blockquote>
                                {alert.resumeEvidence && (
                                  <p>
                                    Resume evidence:{" "}
                                    {alert.resumeEvidence.value}
                                  </p>
                                )}
                                <button
                                  type="button"
                                  className="application-secondary"
                                  onClick={() =>
                                    update((current) => ({
                                      ...current,
                                      dismissedAlertIds: [
                                        ...current.dismissedAlertIds,
                                        alert.id,
                                      ],
                                    }))
                                  }
                                >
                                  Dismiss
                                </button>
                              </article>
                            ))}
                        {!draft.ignoreAllAlerts &&
                          draft.dismissedAlertIds.length > 0 && (
                            <div>
                              <h3>Dismissed alerts</h3>
                              {draft.alerts
                                .filter((alert) =>
                                  draft.dismissedAlertIds.includes(alert.id),
                                )
                                .map((alert) => (
                                  <button
                                    key={alert.id}
                                    type="button"
                                    className="application-secondary"
                                    onClick={() =>
                                      update((current) => ({
                                        ...current,
                                        dismissedAlertIds:
                                          current.dismissedAlertIds.filter(
                                            (id) => id !== alert.id,
                                          ),
                                      }))
                                    }
                                  >
                                    Reopen {alert.requirement}
                                  </button>
                                ))}
                            </div>
                          )}
                        <p>
                          Advisory only. These alerts do not prevent editing or
                          export.
                        </p>
                      </div>
                    )}
                    <div className="application-file">
                      <strong>tailored-resume.pdf</strong>
                      <div className="application-row">
                        <button
                          type="button"
                          onClick={() => showPdf("resume")}
                          disabled={busy || dirty}
                        >
                          Preview and edit
                        </button>
                        <button
                          type="button"
                          className="application-secondary"
                          onClick={() => downloadPdf("resume")}
                          disabled={busy || dirty}
                        >
                          Download
                        </button>
                      </div>
                      <div
                        className="application-drag"
                        aria-label="Drag tailored resume PDF to a compatible upload field"
                        onMouseDown={() => dragPdf("resume")}
                      >
                        Drag this PDF into a compatible upload field
                      </div>
                    </div>
                    <label>
                      What should change?
                      <textarea
                        value={instruction}
                        onChange={(event) => setInstruction(event.target.value)}
                        rows={3}
                        maxLength={2000}
                        placeholder="Give a specific correction for regeneration"
                      />
                    </label>
                    <button
                      type="button"
                      onClick={generateResume}
                      disabled={busy || dirty || !instruction.trim()}
                    >
                      Regenerate resume
                    </button>
                    {preview?.kind === "resume" && (
                      <div className="application-preview">
                        <div className="application-row">
                          <h2>PDF preview</h2>
                          <button
                            type="button"
                            className="application-secondary"
                            onClick={() => setEditing(!editing)}
                          >
                            {editing
                              ? "Close editor"
                              : "Edit structured content"}
                          </button>
                        </div>
                        {dirty && (
                          <p className="application-note" role="status">
                            This PDF shows the last saved version. Save edits to
                            refresh it.
                          </p>
                        )}
                        <div className="application-preview-grid">
                          <PdfCanvas base64={preview.base64} />
                          {editing && (
                            <ResumeFields
                              document={draft.resume}
                              onChange={(resume) =>
                                update((current) => ({ ...current, resume }))
                              }
                            />
                          )}
                        </div>
                      </div>
                    )}
                  </section>
                )}
                {tab === "cover" && (
                  <section className="application-panel">
                    <p className="application-kicker">Stage 2 · Cover letter</p>
                    <h1>Cover letter</h1>
                    <label>
                      Optional instruction
                      <textarea
                        value={coverInstruction}
                        onChange={(event) =>
                          setCoverInstruction(event.target.value)
                        }
                        rows={3}
                        maxLength={2000}
                      />
                    </label>
                    <button
                      type="button"
                      onClick={generateCover}
                      disabled={busy || dirty}
                    >
                      {draft.coverLetter
                        ? "Regenerate cover letter"
                        : "Generate cover letter"}
                    </button>
                    {draft.coverLetter !== null && (
                      <>
                        <p className="application-note">
                          Review the letter against your published resume before using it.
                        </p>
                        <div className="application-file">
                          <strong>cover-letter.pdf</strong>
                          <div className="application-row">
                            <button
                              type="button"
                              disabled={busy || dirty}
                              onClick={() => showPdf("cover_letter")}
                            >
                              Preview and edit
                            </button>
                            <button
                              type="button"
                              className="application-secondary"
                              disabled={busy || dirty}
                              onClick={() => downloadPdf("cover_letter")}
                            >
                              Download
                            </button>
                          </div>
                          <div
                            className="application-drag"
                            aria-label="Drag cover letter PDF to a compatible upload field"
                            onMouseDown={() => dragPdf("cover_letter")}
                          >
                            Drag this PDF into a compatible upload field
                          </div>
                        </div>
                        {preview?.kind === "cover_letter" && (
                          <div className="application-preview">
                            <div className="application-row">
                              <h2>PDF preview</h2>
                            </div>
                            {dirty && (
                              <p className="application-note" role="status">
                                This PDF shows the last saved version. Save
                                edits to refresh it.
                              </p>
                            )}
                            <div className="application-preview-grid">
                              <PdfCanvas base64={preview.base64} />
                              <label>
                                Review and edit cover letter
                                <textarea
                                  value={draft.coverLetter}
                                  onChange={(event) =>
                                    update((current) => ({
                                      ...current,
                                      coverLetter: event.target.value,
                                    }))
                                  }
                                  rows={20}
                                  maxLength={12000}
                                />
                              </label>
                            </div>
                          </div>
                        )}
                        {preview?.kind !== "cover_letter" && (
                          <label>
                            Review and edit cover letter
                            <textarea
                              value={draft.coverLetter}
                              onChange={(event) =>
                                update((current) => ({
                                  ...current,
                                  coverLetter: event.target.value,
                                }))
                              }
                              rows={14}
                              maxLength={12000}
                            />
                          </label>
                        )}
                      </>
                    )}
                  </section>
                )}
                {tab === "answers" && (
                  <section className="application-panel">
                    <p className="application-kicker">Stage 2 · Answers</p>
                    <h1>Application answers</h1>
                    <label>
                      Reviewed question
                      <textarea
                        value={question}
                        onChange={(event) => {
                          const next = event.target.value;
                          setQuestion(next);
                          update((current) => ({
                            ...current,
                            question: next,
                            answer:
                              next === current.question ? current.answer : "",
                          }));
                        }}
                        rows={5}
                        maxLength={2000}
                      />
                    </label>
                    <label>
                      Optional character limit
                      <input
                        type="number"
                        min="1"
                        max="4000"
                        value={limit}
                        onChange={(event) => setLimit(event.target.value)}
                      />
                    </label>
                    <button
                      type="button"
                      onClick={generateAnswer}
                      disabled={busy || dirty || !question.trim()}
                    >
                      Generate answer
                    </button>
                    {draft.answer && (
                      <>
                        <p className="application-note">
                          Review the answer against your published resume before using it.
                        </p>
                        <label>
                          Review and edit answer
                          <textarea
                            value={draft.answer}
                            onChange={(event) =>
                              update((current) => ({
                                ...current,
                                answer: event.target.value,
                              }))
                            }
                            rows={8}
                            maxLength={4000}
                          />
                        </label>
                        <div className="application-row">
                          <button
                            type="button"
                            className="application-secondary"
                            onClick={() =>
                              void navigator.clipboard.writeText(draft.answer)
                            }
                          >
                            Copy answer
                          </button>
                          <button
                            type="button"
                            className="application-secondary"
                            onClick={() =>
                              update((current) => ({
                                ...current,
                                approvedAnswers: [
                                  ...current.approvedAnswers,
                                  {
                                    question: current.question,
                                    answer: current.answer,
                                  },
                                ],
                              }))
                            }
                            disabled={draft.approvedAnswers.length >= 30}
                          >
                            Add to answer set
                          </button>
                          <button
                            type="button"
                            className="application-secondary"
                            onClick={() => setResetting(true)}
                            disabled={busy || dirty}
                          >
                            Reset and capture new question
                          </button>
                        </div>
                      </>
                    )}
                    {draft.approvedAnswers.length > 0 && (
                      <ol>
                        {draft.approvedAnswers.map((item, index) => (
                          <li key={index}>
                            <strong>{item.question}</strong>
                            <p>{item.answer}</p>
                          </li>
                        ))}
                      </ol>
                    )}
                  </section>
                )}
              </>
            )
          )}
        </fieldset>
      </div>
      {pendingCapture && !finishing && !resetting && (
        <div className="application-dialog-backdrop">
          <section
            role="dialog"
            aria-modal="true"
            aria-label="Review browser capture"
            className="application-dialog"
          >
            <h2>Review browser capture</h2>
            <p>
              {pendingCapture.capture.payload.target === "job"
                ? "Accepting this capture replaces the current reviewed job text and source URL."
                : "Accepting this capture replaces the current question and its unretained answer."}
            </p>
            {pendingCapture.capture.payload.title && (
              <p>{pendingCapture.capture.payload.title}</p>
            )}
            <label>
              Selected text
              <textarea
                value={captureText}
                onChange={(event) => setCaptureText(event.target.value)}
                rows={8}
                maxLength={131072}
              />
            </label>
            <label>
              Source URL (optional)
              <input
                type="url"
                value={captureUrl}
                onChange={(event) => setCaptureUrl(event.target.value)}
                maxLength={4096}
              />
            </label>
            {!captureReady && (
              <p role="status">
                {pendingCapture.capture.payload.target === "question"
                  ? "Trim the question to 2,000 characters before accepting it."
                  : "Trim the selection to the 128 KiB capture limit before accepting it."}
              </p>
            )}
            {pendingCapture.capture.payload.target === "job" && saved && (
              <p role="status">
                Finish the current application before accepting a new job.
              </p>
            )}
            {pendingCapture.capture.payload.target === "question" && !saved && (
              <p role="status">
                Start an application before accepting a question.
              </p>
            )}
            {dirty && (
              <p role="status">
                Save your edits before accepting this capture.
              </p>
            )}
            <div className="application-row">
              <button
                type="button"
                disabled={
                  busy ||
                  dirty ||
                  !captureReady ||
                  (pendingCapture.capture.payload.target === "job" &&
                    !!saved) ||
                  (pendingCapture.capture.payload.target === "question" &&
                    !saved)
                }
                onClick={() => resolveCapture(true)}
              >
                {pendingCapture.capture.payload.target === "job"
                  ? "Replace reviewed job"
                  : "Replace current question"}
              </button>
              <button
                type="button"
                className="application-secondary"
                disabled={busy}
                onClick={() => resolveCapture(false)}
              >
                Discard capture
              </button>
            </div>
          </section>
        </div>
      )}
      {resetting && (
        <div className="application-dialog-backdrop">
          <section
            role="dialog"
            aria-modal="true"
            aria-label="Reset question"
            className="application-dialog"
          >
            <h2>Reset current question?</h2>
            <p>
              The current unretained answer will be cleared. Approved answers
              stay in the answer set.
            </p>
            <div className="application-row">
              <button
                type="button"
                disabled={busy || dirty || !saved || !draft}
                onClick={() =>
                  void run(async () => {
                    if (!saved || !draft) return;
                    apply(
                      await command<Saved>("save_application_workspace", {
                        expectedRevision: saved.revision,
                        workspace: { ...draft, question: "", answer: "" },
                      }),
                    );
                    setQuestion("");
                    setResetting(false);
                  })
                }
              >
                Reset question
              </button>
              <button
                type="button"
                className="application-secondary"
                onClick={() => setResetting(false)}
              >
                Keep it
              </button>
            </div>
          </section>
        </div>
      )}
      {finishing && saved && (
        <div className="application-dialog-backdrop">
          <section
            role="dialog"
            aria-modal="true"
            aria-label="Finish application"
            className="application-dialog"
          >
            <h2>Finish Application</h2>
            <h3>Selected application</h3>
            <ul>
              <li>
                Tailored resume: {draft?.resume.title || "Untitled resume"}
              </li>
              <li>Cover letter: {draft?.coverLetter ? "reviewed" : "none"}</li>
              <li>Approved answers: {draft?.approvedAnswers.length ?? 0}</li>
            </ul>
            <p>
              Finishing clears the current workspace, including its job text,
              alerts, unretained answer, and generated drag files. Download any
              PDFs you want to keep first.
            </p>
            <p>
              Save selected materials to your local tracker, or finish without
              retaining an entry.
            </p>
            {dirty && (
              <p role="status">
                Return to the workspace and save your edits before finishing.
              </p>
            )}
            <TrackerFields entry={tracking} onChange={setTracking} />
            <div className="tracker-retention">
              <label>
                <input
                  type="checkbox"
                  checked={retainResume}
                  onChange={(event) => setRetainResume(event.target.checked)}
                />{" "}
                Retain selected resume snapshot
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={retainCover}
                  disabled={!draft?.coverLetter}
                  onChange={(event) => setRetainCover(event.target.checked)}
                />{" "}
                Retain reviewed cover letter
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={retainAnswers}
                  disabled={!draft?.approvedAnswers.length}
                  onChange={(event) => setRetainAnswers(event.target.checked)}
                />{" "}
                Retain approved answers ({draft?.approvedAnswers.length ?? 0})
              </label>
            </div>
            <div className="application-row">
              <button
                type="button"
                disabled={busy || dirty}
                onClick={() => finish(true)}
              >
                Save to tracker and finish
              </button>
              <button
                type="button"
                disabled={busy || dirty}
                onClick={() => finish(false)}
              >
                Finish without saving
              </button>
              <button
                type="button"
                className="application-secondary"
                onClick={() => setFinishing(false)}
              >
                Continue working
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}
