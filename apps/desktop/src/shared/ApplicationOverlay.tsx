import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useRef, useState } from "react";
import type { ResumeDocument } from "@ort/contracts/resume";
import logo from "../assets/open-frame-icon.svg";
import { useApplicationPopup } from "./application-popup";
import {
  DOCUMENT_STYLE_LABELS,
  SELECTABLE_DOCUMENT_STYLES,
} from "./document-styles";
import "./application-overlay.css";
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
type Preset = "economy" | "balanced" | "quality";
type Context = {
  publishedRevision: number | null;
  aiLabel: string;
  aiReady: boolean;
  aiBusy: boolean;
  selectedKeyId: string | null;
  preset: Preset | null;
  presetOptions: { preset: Preset; label: string; model: string | null }[];
  browserConnected: boolean;
};
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
  BROWSER_CAPTURE_UNAVAILABLE:
    "Browser capture is not connected in this build.",
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
  EXPORT_PREPARE_FAILED:
    "The updated files could not be prepared. Review the resume content and try again.",
  EXPORT_NOT_PREPARED:
    "The updated file is still being prepared. Try again in a moment.",
  PDF_UNAVAILABLE:
    "This material could not be rendered as a PDF. Review its length and characters.",
  EXPORT_CANCELLED: "Download cancelled.",
  DRAG_UNAVAILABLE:
    "File drag is unavailable. Use Download and select the saved file on the application site.",
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
  const [job, setJobState] = useState("");
  const jobRef = useRef("");
  function setJob(value: string) {
    jobRef.current = value;
    setJobState(value);
  }
  const [jobUrl, setJobUrlState] = useState("");
  const jobUrlRef = useRef("");
  function setJobUrl(value: string) {
    jobUrlRef.current = value;
    setJobUrlState(value);
  }
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
  const [working, setWorking] = useState(false);
  const [closePending, setClosePending] = useState(false);
  const closeDirty = useRef(true);
  const [notice, setNotice] = useState("");
  const [instruction, setInstruction] = useState("");
  const [coverInstruction, setCoverInstruction] = useState("");
  const [question, setQuestion] = useState("");
  const [limit, setLimit] = useState("");
  const [format, setFormat] = useState<"pdf" | "docx">("pdf");
  const [prepared, setPrepared] = useState<string | null>(null);
  const [exportError, setExportError] = useState("");
  const [exportAttempt, setExportAttempt] = useState(0);
  const [saveStatus, setSaveStatus] = useState<"saved" | "saving" | "error">(
    "saved",
  );
  const savedRef = useRef<Saved | null>(null);
  const draftRef = useRef<Workspace | null>(null);
  const workspacePending = useRef<Workspace | null>(null);
  const workspaceFlush = useRef<Promise<void> | null>(null);
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
  savedRef.current = saved;
  draftRef.current = draft;
  const aiWorking = working || !!context?.aiBusy;
  const materialKind: MaterialKind =
    tab === "cover" ? "cover_letter" : "resume";
  const exportKey = saved ? `${saved.revision}:${materialKind}` : "";
  const exportReady = !dirty && prepared === exportKey && !busy;
  const popup = useApplicationPopup({
    job,
    jobUrl,
    resume: draft?.resume ?? null,
    style: draft?.style ?? style,
    coverLetter: draft?.coverLetter ?? null,
    disabled: busy || closePending,
    onJobChange: setJob,
    onUrlChange: setJobUrl,
    onResumeChange: (resume) => update((current) => ({ ...current, resume })),
    onCoverChange: (coverLetter) =>
      update((current) => ({ ...current, coverLetter })),
    onError: (error) => setNotice(error),
  });
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
    saveStatus === "saving" ||
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
        const reply = (popupUnavailable = false) =>
          emitTo("main", "ort:overlay-close-reply", {
            attempt: event.payload.attempt,
            dirty:
              popupUnavailable ||
              closeDirty.current ||
              !!workspacePending.current ||
              (!savedRef.current &&
                (jobRef.current !==
                  (stageOneSaved.current?.jobDescription ?? "") ||
                  jobUrlRef.current !== (stageOneSaved.current?.jobUrl ?? ""))),
          });
        void popup
          .flush()
          .then(() => reply())
          .catch(() => reply(true));
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
        savedRef.current = current;
        draftRef.current = current?.workspace ?? null;
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
    const timer = window.setInterval(refreshContext, 2000);
    const subscription = getCurrentWebviewWindow()
      .listen("ort:browser-capture", refreshContext)
      .catch(() => () => {});
    return () => {
      active = false;
      window.removeEventListener("focus", refreshContext);
      window.clearInterval(timer);
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
    savedRef.current = current;
    draftRef.current = current.workspace;
    workspacePending.current = null;
    setSaveStatus("saved");
    setSaved(current);
    setDraft(current.workspace);
    setQuestion(current.workspace.question);

    setNotice("");
  }
  function update(change: (current: Workspace) => Workspace) {
    const current = draftRef.current;
    if (!current) return;
    const next = change(current);
    draftRef.current = next;
    workspacePending.current = next;
    setDraft(next);
    setSaveStatus("saving");
    setPrepared(null);
  }
  async function run(action: () => Promise<void>, ai = false) {
    setBusy(true);
    if (ai) setWorking(true);
    setNotice("");
    try {
      await action();
    } catch (error) {
      setNotice(message(error));
    } finally {
      setBusy(false);
      if (ai) setWorking(false);
    }
  }
  async function flushWorkspace(): Promise<void> {
    if (workspaceFlush.current) {
      await workspaceFlush.current;
      if (workspacePending.current) await flushWorkspace();
      return;
    }
    const operation = (async () => {
      while (workspacePending.current && savedRef.current) {
        const next = workspacePending.current;
        workspacePending.current = null;
        setSaveStatus("saving");
        try {
          const updated = await command<Saved>("save_application_workspace", {
            expectedRevision: savedRef.current.revision,
            workspace: next,
          });
          savedRef.current = updated;
          setSaved(updated);
          // A save acknowledgement must never replace more recent typing.
          if (draftRef.current === next) {
            draftRef.current = updated.workspace;
            setDraft(updated.workspace);
          }
          setSaveStatus(workspacePending.current ? "saving" : "saved");
        } catch (error) {
          workspacePending.current ??= next;
          setSaveStatus("error");
          throw error;
        }
      }
    })();
    workspaceFlush.current = operation;
    try {
      await operation;
    } finally {
      if (workspaceFlush.current === operation) workspaceFlush.current = null;
    }
  }
  function save() {
    void flushWorkspace().catch((error: unknown) => setNotice(message(error)));
  }
  useEffect(() => {
    if (!dirty || saveStatus === "error") return;
    const timer = window.setTimeout(save, 180);
    return () => window.clearTimeout(timer);
  }, [draft, dirty]);

  useEffect(() => {
    if (
      !saved ||
      dirty ||
      (materialKind === "cover_letter" && !draft?.coverLetter)
    )
      return;
    let active = true;
    setPrepared(null);
    setExportError("");
    void command("prepare_application_exports", {
      expectedRevision: saved.revision,
      kind: materialKind,
    })
      .then(() => {
        if (active) setPrepared(exportKey);
      })
      .catch((error: unknown) => {
        if (active) setExportError(message(error));
      });
    return () => {
      active = false;
    };
  }, [saved?.revision, dirty, materialKind, exportAttempt]);
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
    }, true);
  }
  function generateCover() {
    if (!saved || dirty) return;
    void run(
      async () =>
        apply(
          await command<Saved>("generate_application_cover_letter", {
            expectedRevision: saved.revision,
            instruction: coverInstruction.trim(),
          }),
        ),
      true,
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
    void run(
      async () =>
        apply(
          await command<Saved>("generate_application_answer", {
            expectedRevision: saved.revision,
            question: question.trim(),
            limit: parsed,
          }),
        ),
      true,
    );
  }
  function download(kind: MaterialKind) {
    if (!saved || !exportReady) return;
    void run(async () => {
      await command<boolean>("download_application_export", {
        expectedRevision: saved.revision,
        kind,
        format,
      });
      setNotice(`${format.toUpperCase()} downloaded.`);
    });
  }
  function drag(kind: MaterialKind) {
    if (!saved || !exportReady) return;
    void command<boolean>("drag_application_export", {
      expectedRevision: saved.revision,
      kind,
      format,
    }).catch((error: unknown) => setNotice(message(error)));
  }
  function fileControls(kind: MaterialKind) {
    return (
      <div className="application-file">
        <label>
          Download format
          <select
            aria-label="Download format"
            value={format}
            onChange={(event) =>
              setFormat(event.target.value as "pdf" | "docx")
            }
          >
            <option value="pdf">PDF</option>
            <option value="docx">Word (.docx)</option>
          </select>
        </label>
        <div className="application-button-pair">
          <button
            type="button"
            disabled={!exportReady}
            onClick={() => download(kind)}
          >
            <OverlayIcon name="download" />
            Download
          </button>
          <button
            type="button"
            className="application-drag"
            disabled={!exportReady}
            aria-label={`Drag ${kind === "resume" ? "resume" : "cover letter"} ${format.toUpperCase()} to upload`}
            onMouseDown={(event) => {
              if (event.button === 0) drag(kind);
            }}
          >
            <OverlayIcon name="file" />
            Drag me
          </button>
        </div>
        <div className="application-button-pair">
          <button
            type="button"
            className="application-secondary"
            onClick={() =>
              popup.open(kind === "resume" ? "resume-view" : "cover")
            }
          >
            <OverlayIcon name="view" />
            View
          </button>
          <button
            type="button"
            className="application-secondary"
            onClick={() =>
              popup.open(kind === "resume" ? "resume-edit" : "cover")
            }
          >
            <OverlayIcon name="edit" />
            Edit
          </button>
        </div>
        <p className="application-note application-export-status" role="status">
          {exportError ||
            (dirty
              ? "Saving your latest edits…"
              : exportReady
                ? `${format.toUpperCase()} ready to download or drag`
                : "Preparing updated files…")}
        </p>
        {exportError && (
          <button
            type="button"
            className="application-secondary"
            onClick={() => setExportAttempt((value) => value + 1)}
          >
            Retry preparing files
          </button>
        )}
      </div>
    );
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

      setFinishing(false);
      stageOneRevision.current = null;
      stageOneSaved.current = null;
      stageOnePending.current = null;
      setStageOneStatus("saved");
      setJob("");
      setJobUrl("");
      setStyle("technical");
      setTracking(emptyTrackerEntry());
      await popup.close();
      savedRef.current = null;
      draftRef.current = null;
      workspacePending.current = null;
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
      <header
        className="application-header"
        title="Drag header to move overlay"
        onMouseDown={(event) => {
          if (
            event.button === 0 &&
            !(event.target as Element).closest("button, select, input")
          ) {
            void getCurrentWebviewWindow()
              .startDragging()
              .catch((error: unknown) => setNotice(message(error)));
          }
        }}
      >
        <div className="application-drag-handle">
          <img src={logo} alt="Open Resume Toolkit" width="30" height="30" />
        </div>
        <div className="application-connection">
          <select
            aria-label="AI model preset"
            title={context?.aiLabel}
            value={context?.preset ?? ""}
            disabled={!context?.selectedKeyId || busy || aiWorking}
            onChange={(event) =>
              void run(async () => {
                await command("set_ai_key_preset", {
                  request: {
                    credentialId: context?.selectedKeyId,
                    preset: event.target.value,
                  },
                });
                setContext(await command<Context>("application_context"));
              })
            }
          >
            {!context?.preset && (
              <option value="">{context?.aiLabel ?? "Checking AI…"}</option>
            )}
            {(context?.presetOptions ?? []).map((item) => (
              <option
                key={item.preset}
                value={item.preset}
                disabled={!item.model}
              >
                {item.label}
              </option>
            ))}
          </select>
          <div className="application-key-status" role="status">
            <span
              className={`application-dot${aiWorking ? " application-dot--working" : context?.aiReady ? " application-dot--ready" : ""}`}
            />
            {aiWorking
              ? "Working"
              : context?.aiReady
                ? "Ready"
                : "Select an API key in the main app"}
            {aiWorking && (
              <button
                type="button"
                onClick={() =>
                  void command<boolean>("cancel_application_generation").catch(
                    (error: unknown) => setNotice(message(error)),
                  )
                }
              >
                Cancel
              </button>
            )}
          </div>
        </div>
        <div
          className={`application-browser${context?.browserConnected ? " is-connected" : ""}`}
          role="status"
          title={
            context?.browserConnected
              ? "Browser connected"
              : "Browser disconnected"
          }
        >
          <OverlayIcon name="browser" />
          <span>{context?.browserConnected ? "Connected" : "Offline"}</span>
        </div>
      </header>
      <div className="application-content">
        {notice && (
          <p className="application-notice" role="alert">
            {notice}
            <button
              type="button"
              className="application-dismiss"
              aria-label="Dismiss message"
              onClick={() => setNotice("")}
            >
              ×
            </button>
          </p>
        )}
        <fieldset
          className="application-workspace-fields"
          disabled={busy || closePending}
        >
          {!saved ? (
            <section className="application-stage-one">
              <p className="application-kicker">01 / Capture the opportunity</p>
              <h1>Ready for your next role.</h1>
              <p className="application-intro">
                Capture a job from your browser, or add the details below.
              </p>
              <div className="application-button-pair application-capture-actions">
                <button
                  type="button"
                  className="application-secondary"
                  disabled={!context?.browserConnected}
                  onClick={() =>
                    void run(async () => {
                      await command("request_application_capture", {
                        target: "job",
                      });
                      setNotice("Select the job description in your browser.");
                    })
                  }
                >
                  <OverlayIcon name="capture" />
                  Capture
                </button>
                <button
                  type="button"
                  disabled={
                    !stageOneReady ||
                    !jobReady ||
                    !context?.publishedRevision ||
                    !context?.aiReady ||
                    aiWorking
                  }
                  onClick={() =>
                    void run(async () => {
                      await popup.close();
                      stageOnePending.current = {
                        jobDescription: jobRef.current,
                        jobUrl: jobUrlRef.current,
                        style,
                      };
                      await flushStageOne();
                      apply(
                        await command<Saved>("start_application", {
                          jobDescription: jobRef.current.trim(),
                          jobUrl: sanitizeCaptureUrl(jobUrlRef.current),
                          style,
                        }),
                      );
                      setTab("resume");
                    }, true)
                  }
                >
                  Tailor
                  <OverlayIcon name="arrow" />
                </button>
              </div>
              <CaptureField
                title="Job Description"
                complete={jobReady}
                onView={() => popup.open("job")}
              />
              <CaptureField
                title="Job URL"
                complete={!!jobUrl.trim()}
                onView={() => popup.open("url")}
              />
              {job.length > 20_000 && (
                <p className="application-note" role="status">
                  Trim the job description to 20,000 characters before
                  tailoring.
                </p>
              )}
              {jobBytes > 128 * 1024 && (
                <p className="application-note" role="alert">
                  The job description exceeds the 128 KiB capture limit.
                </p>
              )}
              <p className="application-local-status" role="status">
                {stageOneStatus === "saving"
                  ? "Saving details…"
                  : stageOneStatus === "error"
                    ? "Details could not be saved. Try again."
                    : "Your details are saved locally."}
              </p>
              {!context?.publishedRevision && (
                <p className="application-note">
                  Publish your master resume in the main app to begin.
                </p>
              )}
            </section>
          ) : (
            draft && (
              <>
                <section className="application-role">
                  <p className="application-kicker">02 / Your application</p>
                  <h1>{draft.roleInfo?.company || "Your next opportunity"}</h1>
                  {draft.roleInfo?.title && <p>{draft.roleInfo.title}</p>}
                  <button
                    type="button"
                    className="application-secondary"
                    onClick={() =>
                      void run(async () => {
                        await popup.close();
                        setTracking({
                          ...emptyTrackerEntry(),
                          company: draft.roleInfo?.company ?? "",
                          title: draft.roleInfo?.title ?? "",
                          location: draft.roleInfo?.location ?? "",
                          sourceUrl: draft.jobUrl,
                        });
                        setFinishing(true);
                      })
                    }
                  >
                    Finish Application
                    <OverlayIcon name="check" />
                  </button>
                </section>
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
                      onClick={() =>
                        void popup
                          .close()
                          .then(() => setTab(item))
                          .catch((error: unknown) => setNotice(message(error)))
                      }
                    >
                      {item === "cover"
                        ? "Cover letter"
                        : item === "answers"
                          ? "Answers"
                          : "Resume"}
                    </button>
                  ))}
                </nav>
                {saveStatus === "error" && (
                  <div className="application-savebar" role="alert">
                    <span>Edits could not be saved.</span>
                    <button type="button" onClick={save}>
                      Retry save
                    </button>
                  </div>
                )}
                {tab === "resume" && (
                  <section className="application-panel">
                    <div className="application-tailoring-notes">
                      <h2>Tailoring notes</h2>
                      {draft.changePoints.length ? (
                        <ul aria-label="Tailoring notes">
                          {draft.changePoints.map((point, index) => (
                            <li key={index}>{point}</li>
                          ))}
                        </ul>
                      ) : (
                        <p className="application-note">
                          Your tailored resume is ready to review.
                        </p>
                      )}
                    </div>
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

                    <label>
                      Resume style
                      <select
                        value={draft.style}
                        onChange={(event) =>
                          update((current) => ({
                            ...current,
                            style: event.target.value as Workspace["style"],
                          }))
                        }
                      >
                        {draft.style === "plain" && (
                          <option value="plain">Plain</option>
                        )}
                        {SELECTABLE_DOCUMENT_STYLES.map((item) => (
                          <option key={item} value={item}>
                            {DOCUMENT_STYLE_LABELS[item]}
                          </option>
                        ))}
                      </select>
                    </label>
                    {fileControls("resume")}
                    <div className="application-refinement">
                      <label>
                        Refine with AI
                        <textarea
                          value={instruction}
                          onChange={(event) =>
                            setInstruction(event.target.value)
                          }
                          rows={3}
                          maxLength={2000}
                          placeholder="What would you like to change?"
                        />
                      </label>
                      <button
                        type="button"
                        onClick={generateResume}
                        disabled={
                          dirty ||
                          aiWorking ||
                          !context?.aiReady ||
                          !instruction.trim()
                        }
                      >
                        Refine resume
                        <OverlayIcon name="arrow" />
                      </button>
                    </div>
                  </section>
                )}
                {tab === "cover" && (
                  <section className="application-panel">
                    <h2>Cover letter</h2>
                    <p className="application-note">
                      A letter grounded in your experience and this role.
                    </p>
                    <label>
                      Instructions (optional)
                      <textarea
                        value={coverInstruction}
                        onChange={(event) =>
                          setCoverInstruction(event.target.value)
                        }
                        rows={3}
                        maxLength={2000}
                        placeholder="Add a focus or preferred tone"
                      />
                    </label>
                    <button
                      type="button"
                      onClick={generateCover}
                      disabled={dirty || aiWorking || !context?.aiReady}
                    >
                      {draft.coverLetter
                        ? "Refine cover letter"
                        : "Generate cover letter"}
                    </button>
                    {draft.coverLetter !== null && fileControls("cover_letter")}
                  </section>
                )}
                {tab === "answers" && (
                  <section className="application-panel">
                    <h2>Application answers</h2>
                    <label>
                      Question
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
                        rows={4}
                        maxLength={2000}
                        placeholder="Paste an application question"
                      />
                    </label>
                    <label>
                      Character limit (optional)
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
                      disabled={
                        dirty ||
                        aiWorking ||
                        !context?.aiReady ||
                        !question.trim()
                      }
                    >
                      Generate answer
                    </button>
                    {draft.answer && (
                      <>
                        <label>
                          Review your answer
                          <textarea
                            value={draft.answer}
                            onChange={(event) =>
                              update((current) => ({
                                ...current,
                                answer: event.target.value,
                              }))
                            }
                            rows={7}
                            maxLength={4000}
                          />
                        </label>
                        <div className="application-button-pair">
                          <button
                            type="button"
                            className="application-secondary"
                            onClick={() =>
                              void navigator.clipboard
                                .writeText(draft.answer)
                                .catch(() =>
                                  setNotice("The answer could not be copied."),
                                )
                            }
                          >
                            Copy
                          </button>
                          <button
                            type="button"
                            disabled={dirty}
                            onClick={() => {
                              setQuestion("");
                              update((current) => ({
                                ...current,
                                approvedAnswers: [
                                  ...current.approvedAnswers,
                                  {
                                    question: current.question,
                                    answer: current.answer,
                                  },
                                ],
                                question: "",
                                answer: "",
                              }));
                            }}
                          >
                            Keep answer
                          </button>
                        </div>
                      </>
                    )}
                    <button
                      type="button"
                      className="application-secondary application-reset"
                      onClick={() => setResetting(true)}
                    >
                      Reset question
                    </button>
                    {draft.approvedAnswers.length > 0 && (
                      <div className="application-answer-list">
                        <h3>Saved answers · {draft.approvedAnswers.length}</h3>
                        {draft.approvedAnswers.map((item, index) => (
                          <details key={index}>
                            <summary>{item.question}</summary>
                            <p>{item.answer}</p>
                            <button
                              type="button"
                              className="application-secondary"
                              onClick={() =>
                                void navigator.clipboard
                                  .writeText(item.answer)
                                  .catch(() =>
                                    setNotice(
                                      "The answer could not be copied.",
                                    ),
                                  )
                              }
                            >
                              Copy answer
                            </button>
                          </details>
                        ))}
                      </div>
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
              files you want to keep first.
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

function CaptureField({
  title,
  complete,
  onView,
}: {
  title: string;
  complete: boolean;
  onView: () => void;
}) {
  return (
    <section className="application-capture-field">
      <h2>{title}</h2>
      <p className={complete ? "is-complete" : ""}>
        <OverlayIcon name={complete ? "check" : "waiting"} />
        {complete ? "Complete" : "Waiting"}
      </p>
      <button
        type="button"
        className="application-secondary"
        aria-label={`View ${title}`}
        onClick={onView}
      >
        <OverlayIcon name="view" />
        View
      </button>
    </section>
  );
}
function OverlayIcon({
  name,
}: {
  name:
    | "download"
    | "file"
    | "view"
    | "edit"
    | "browser"
    | "capture"
    | "arrow"
    | "check"
    | "waiting";
}) {
  const paths = {
    download: (
      <>
        <path d="M12 3v12m-4-4 4 4 4-4" />
        <path d="M4 16v4h16v-4" />
      </>
    ),
    file: (
      <>
        <path d="M6 3h8l4 4v14H6z" />
        <path d="M14 3v5h4M9 12h6M9 16h6" />
      </>
    ),
    view: (
      <>
        <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z" />
        <circle cx="12" cy="12" r="3" />
      </>
    ),
    edit: (
      <>
        <path d="m4 16-1 5 5-1L20 8l-4-4L4 16ZM13 7l4 4" />
      </>
    ),
    browser: (
      <>
        <rect x="3" y="4" width="18" height="16" rx="3" />
        <path d="M3 9h18M7 6h.01M10 6h.01" />
      </>
    ),
    capture: (
      <>
        <path d="M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5" />
        <rect x="7" y="7" width="10" height="10" rx="1" />
      </>
    ),
    arrow: <path d="M4 12h16m-6-6 6 6-6 6" />,
    check: <path d="m5 12 4 4L19 6" />,
    waiting: (
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M12 7v5l3 2" />
      </>
    ),
  };
  return (
    <svg
      className="application-icon"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths[name]}
    </svg>
  );
}
