import { SaveCoordinator } from "./save-coordinator";
import { desktopCommand as command } from "./desktop-client";
import type * as Wire from "@ort/contracts/wire";
import { emitTo } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { availableMonitors } from "@tauri-apps/api/window";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { useEffect, useRef, useState } from "react";
import type { ResumeDocument } from "@ort/contracts/resume";
import logo from "../assets/open-folio-mark.svg";
import { useApplicationPopup } from "./application-popup";
import {
  DOCUMENT_STYLE_LABELS,
  SELECTABLE_DOCUMENT_STYLES,
} from "./document-styles";
import "./application-overlay.css";
import { sanitizeCaptureUrl } from "./capture-url";
import { clampOverlayPosition } from "./overlay-drag";
import {
  TrackerFields,
  emptyTrackerEntry,
  type TrackerEntry,
} from "./TrackerFields";

type Tab = "resume" | "cover" | "answers";
type MaterialKind = "resume" | "cover_letter";
type Alert = Wire.QualificationAlert;
type Workspace = Wire.ApplicationWorkspace;
type Saved = Wire.SavedWorkspace;
type FinishMode = "edit" | "discard" | null;

function qualificationPoint(alert: Alert): string {
  const text = (alert.target?.trim() || alert.requirement)
    .replace(/\s+/g, " ")
    .replace(/^(?:missing|need|requires?)\s+/i, "")
    .replace(/[.;:]+$/, "")
    .trim();
  const detail = text.length > 100 ? `${text.slice(0, 97).trimEnd()}…` : text;
  if (alert.kind === "confirmed_mismatch") {
    return alert.category === "graduation_date" && /^20\d{2}$/.test(detail)
      ? `Need graduation in ${detail}`
      : `Requirement mismatch: ${detail}`;
  }
  if (alert.category === "experience_duration") return `Need ${detail}`;
  if (
    alert.category === "named_skill_or_technology" &&
    /^[cr]$/i.test(detail)
  ) {
    return `Missing ${detail.toUpperCase()} language`;
  }
  if (alert.category === "language_proficiency")
    return `Missing ${detail}${/\b(?:proficiency|fluency|fluent|proficient)\b/i.test(detail) ? "" : " proficiency"}`;
  return `Missing ${detail}`;
}

function trackerEntryFor(workspace: Workspace): TrackerEntry {
  const now = new Date();
  const dateApplied = [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
  ].join("-");
  return {
    ...emptyTrackerEntry(),
    company: workspace.roleInfo.company,
    title: workspace.roleInfo.title,
    location: workspace.roleInfo.location,
    dateApplied,
    sourceUrl: workspace.jobUrl,
    ...workspace.trackerMetadata,
  };
}
type StageOne = Wire.StageOneDraft;
type SavedStageOne = Wire.SavedStageOneDraft;
type SavedPendingCapture = Wire.SavedPendingCapture;
type Preset = "economy" | "balanced" | "quality";
type Context = Wire.ApplicationContext;
type CaptureMode = Wire.CaptureStatus;
const idleCapture: CaptureMode = {
  phase: "idle",
  sessionId: null,
  error: null,
};
const errors: Record<string, string> = {
  BROWSER_CAPTURE_UNAVAILABLE:
    "Enable Browser connections in ORT Settings and reload the Chrome extension.",
  PAGE_UNAVAILABLE:
    "Open a normal web page in Chrome and allow the ORT extension access to that site.",
  PAGE_CHANGED: "The page changed. Press Capture to select the text again.",
  EMPTY_SELECTION:
    "The box contained no readable text. Press Capture and choose another area.",
  CAPTURE_EXPIRED: "Capture mode timed out. Press Capture to begin again.",
  CAPTURE_TOO_LARGE: "Choose a smaller area of text (128 KiB maximum).",
  CAPTURE_PENDING:
    "Review or discard the current capture before starting another.",
  BRIDGE_UNAVAILABLE:
    "The browser connection is unavailable. Check Browser connections in Settings.",
  DELIVERY_UNCONFIRMED:
    "Check the capture review before trying again; delivery could not be confirmed.",
  CAPTURE_BUSY: "Capture mode is already active.",
  PUBLISHED_RESUME_REQUIRED:
    "Publish a master resume in the main window before continuing.",
  AI_DISABLED: "Set up a Direct AI key in the main window before continuing.",
  AI_CAP_REJECTED: "This request would exceed your AI spending cap.",
  AI_OUTPUT_INVALID:
    "The provider returned a blocked, incomplete, or unreadable response. Nothing was saved.",
  AI_OUTPUT_INCOMPLETE:
    "The provider did not finish generating the material. Nothing was saved.",
  AI_MODEL_MISMATCH:
    "The provider returned a different model than the one selected. Nothing was saved.",
  AI_MATERIAL_INVALID:
    "The provider response did not match the required material format. Nothing was saved.",
  AI_AUTHENTICATION_FAILED:
    "The provider rejected the active API key. Check the key and its permissions in My Keys.",
  AI_RATE_LIMITED:
    "The provider rate-limited this request. Check its usage limits before retrying.",
  AI_PROVIDER_SERVICE_UNAVAILABLE:
    "The provider is temporarily unavailable. Try again later.",
  AI_PROVIDER_TEMPORARY:
    "The provider returned a temporary server error. Try again later.",
  AI_PROVIDER_BAD_REQUEST:
    "The provider rejected the request parameters or API key (HTTP 400). Nothing was saved.",
  AI_MODEL_UNAVAILABLE:
    "The selected model is unavailable to this API key (HTTP 404). Nothing was saved.",
  ANSWER_LIMIT_REACHED:
    "This workspace already has 30 saved answers. Finish this application to start another.",
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
  RESUME_INVALID:
    "This resume edit could not be saved because an item has invalid structure. Review the new item and try again.",
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
  const [captureMode, setCaptureMode] = useState<CaptureMode>(idleCapture);
  const captureRefreshEpoch = useRef(0);
  const applyingJobCapture = useRef(false);
  const attemptedJobCapture = useRef<string | null>(null);
  const [jobCaptureError, setJobCaptureError] = useState(false);
  const capturing =
    captureMode.phase === "waiting" || captureMode.phase === "selecting";
  const [captureText, setCaptureText] = useState("");
  const [captureUrl, setCaptureUrl] = useState("");
  const headerDrag = useRef<{
    pointerId: number;
    startScreenX: number;
    startScreenY: number;
    startX: number;
    startY: number;
    pointerOffsetX: number;
    pointerOffsetY: number;
    scale: number;
    size: { width: number; height: number };
    monitors: Awaited<ReturnType<typeof availableMonitors>>;
    pending: { x: number; y: number } | null;
    moving: boolean;
  } | null>(null);
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
  const [answerInstruction, setAnswerInstruction] = useState("");
  const [format, setFormat] = useState<"pdf" | "docx">("pdf");
  const [prepared, setPrepared] = useState<string | null>(null);
  const [exportError, setExportError] = useState("");
  const [exportAttempt, setExportAttempt] = useState(0);
  const [saveStatus, setSaveStatus] = useState<"saved" | "saving" | "error">(
    "saved",
  );
  const contextRef = useRef(context);
  contextRef.current = context;
  const savedRef = useRef<Saved | null>(null);
  const draftRef = useRef<Workspace | null>(null);
  const workspacePending = useRef<Workspace | null>(null);
  const workspaceFlush = useRef<Promise<void> | null>(null);
  const stageOneCoordinator = useRef<SaveCoordinator<
    StageOne,
    SavedStageOne
  > | null>(null);
  const workspaceCoordinator = useRef<SaveCoordinator<Workspace, Saved> | null>(
    null,
  );
  useEffect(() => {
    stageOneCoordinator.current = new SaveCoordinator(
      stageOnePending,
      stageOneFlush,
      (draft) =>
        command("save_application_stage_one", {
          expectedRevision: stageOneRevision.current,
          expectedProfileId: contextRef.current!.profileId,
          draft,
        }),
      (result) => {
        stageOneRevision.current = result.revision;
        stageOneSaved.current = result.draft;
      },
      setStageOneStatus,
    );
    workspaceCoordinator.current = new SaveCoordinator(
      workspacePending,
      workspaceFlush,
      (workspace) =>
        command("save_application_workspace", {
          expectedRevision: savedRef.current!.revision,
          expectedProfileId: contextRef.current!.profileId,
          workspace,
        }),
      (result, submitted) => {
        savedRef.current = result;
        setSaved(result);
        if (draftRef.current === submitted) {
          draftRef.current = result.workspace;
          setDraft(result.workspace);
        }
      },
      setSaveStatus,
      () => savedRef.current !== null,
    );
    return () => {
      stageOneCoordinator.current?.dispose();
      workspaceCoordinator.current?.dispose();
    };
  }, []);
  const [finishMode, setFinishMode] = useState<FinishMode>(null);
  const [tracking, setTracking] = useState<TrackerEntry>(emptyTrackerEntry);
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
    workspaceRevision: saved?.revision,
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
    !!answerInstruction.trim() ||
    finishMode !== null ||
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
      command("application_context"),
      command("load_application_workspace"),
      command("load_application_stage_one"),
      command("load_application_capture"),
      command("application_capture_status"),
    ])
      .then(([nextContext, current, stageOne, capture, mode]) => {
        setCaptureMode(mode ?? idleCapture);
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
      const epoch = ++captureRefreshEpoch.current;
      void Promise.all([
        command("load_application_capture"),
        command("application_context"),
        command("application_capture_status"),
      ])
        .then(([capture, nextContext, mode]) => {
          if (active && epoch === captureRefreshEpoch.current) {
            setPendingCapture(capture);
            setContext(nextContext);
            setCaptureMode(mode ?? idleCapture);
          }
        })
        .catch((error: unknown) => {
          if (active && epoch === captureRefreshEpoch.current)
            setNotice(message(error));
        });
    };
    window.addEventListener("focus", refreshContext);
    const timer = window.setInterval(refreshContext, 500);
    const overlayWindow = getCurrentWebviewWindow();
    const subscriptions = Promise.all([
      overlayWindow.listen("ort:browser-capture", refreshContext),
      overlayWindow.listen("ort:capture-mode", refreshContext),
      overlayWindow.listen("ort:ai-preset-changed", refreshContext),
    ]).catch(() => []);
    return () => {
      active = false;
      window.removeEventListener("focus", refreshContext);
      window.clearInterval(timer);
      void subscriptions.then((unlisten) => unlisten.forEach((stop) => stop()));
    };
  }, []);

  async function flushStageOne(): Promise<void> {
    await stageOneCoordinator.current?.flush();
  }

  useEffect(() => {
    if (!stageOneReady || saved || applyingJobCapture.current) return;
    const next = { jobDescription: job, jobUrl, style };
    if (
      !stageOneFlush.current &&
      JSON.stringify(next) ===
        JSON.stringify(
          stageOneSaved.current ?? {
            jobDescription: "",
            jobUrl: "",
            style: "technical",
          },
        )
    ) {
      stageOnePending.current = null;
      setStageOneStatus("saved");
      return;
    }
    stageOnePending.current = next;
    setStageOneStatus("saving");
    const timer = window.setTimeout(() => {
      void flushStageOne().catch((error: unknown) => setNotice(message(error)));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [job, jobUrl, style, stageOneReady, saved, busy]);

  useEffect(() => {
    if (
      !stageOneReady ||
      saved ||
      busy ||
      closePending ||
      pendingCapture?.capture.payload.target !== "job" ||
      attemptedJobCapture.current === pendingCapture.capture.requestId
    )
      return;
    applyJobCapture(pendingCapture);
  }, [pendingCapture, stageOneReady, saved, busy, closePending]);

  function applyJobCapture(capture: SavedPendingCapture) {
    if (applyingJobCapture.current) return;
    applyingJobCapture.current = true;
    attemptedJobCapture.current = capture.capture.requestId;
    setJobCaptureError(false);
    void run(async () => {
      try {
        // Finish earlier local edits before applying the browser result. This
        // keeps their revision from racing the atomic capture transaction.
        await flushStageOne();
        const updated = await command("apply_application_job_capture", {
          requestId: capture.capture.requestId,
          expectedRevision: stageOneRevision.current,
        });
        captureRefreshEpoch.current++;
        stageOneRevision.current = updated.revision;
        stageOneSaved.current = updated.draft;
        stageOnePending.current = null;
        setJob(updated.draft.jobDescription);
        setJobUrl(updated.draft.jobUrl);
        setStyle(updated.draft.style);
        setStageOneStatus("saved");
        setPendingCapture(null);
      } catch (error) {
        setJobCaptureError(true);
        throw error;
      } finally {
        applyingJobCapture.current = false;
      }
    });
  }

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
    await workspaceCoordinator.current?.flush();
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
        await command("regenerate_application_resume", {
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
          await command("generate_application_cover_letter", {
            expectedRevision: saved.revision,
            instruction: coverInstruction.trim(),
          }),
        ),
      true,
    );
  }
  function generateAnswer() {
    if (!saved || dirty || !question.trim()) return;
    void run(
      async () =>
        apply(
          await command("generate_application_answer", {
            expectedRevision: saved.revision,
            question: question.trim(),
          }),
        ),
      true,
    );
  }
  function refineAnswer() {
    if (!saved || dirty || !draft?.answer || !answerInstruction.trim()) return;
    void run(async () => {
      apply(
        await command("refine_application_answer", {
          expectedRevision: saved.revision,
          instruction: answerInstruction.trim(),
        }),
      );
      setAnswerInstruction("");
    }, true);
  }
  function finalizeAnswer(): boolean {
    const current = draftRef.current;
    if (!current) return false;
    if (current.answer.trim() && current.approvedAnswers.length >= 30) {
      setNotice("Save at most 30 application answers in one workspace.");
      return false;
    }
    if (current.question || current.answer) {
      update((workspace) => ({
        ...workspace,
        approvedAnswers: workspace.answer.trim()
          ? [
              ...workspace.approvedAnswers,
              { question: workspace.question, answer: workspace.answer },
            ]
          : workspace.approvedAnswers,
        question: "",
        answer: "",
      }));
    }
    setQuestion("");
    setAnswerInstruction("");
    return true;
  }
  function download(kind: MaterialKind) {
    if (!saved || !exportReady) return;
    void run(async () => {
      await command("download_application_export", {
        expectedRevision: saved.revision,
        kind,
        format,
      });
      setNotice(`${format.toUpperCase()} downloaded.`);
    });
  }
  function drag(kind: MaterialKind) {
    if (!saved || !exportReady) return;
    void command("drag_application_export", {
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
          {kind === "cover_letter" ? (
            <>
              <button
                type="button"
                className="application-secondary"
                disabled={!exportReady}
                onClick={() =>
                  void run(async () => {
                    await popup.flush();
                    await flushWorkspace();
                    const current = savedRef.current;
                    if (!current) return;
                    await command("prepare_application_exports", {
                      expectedRevision: current.revision,
                      kind: "cover_letter",
                    });
                    await popup.open("cover-view");
                  })
                }
              >
                <OverlayIcon name="view" />
                View
              </button>
              <button
                type="button"
                className="application-secondary"
                onClick={() => popup.open("cover")}
              >
                <OverlayIcon name="edit" />
                Edit
              </button>
            </>
          ) : (
            <>
              <button
                type="button"
                className="application-secondary"
                onClick={() => popup.open("resume-view")}
              >
                <OverlayIcon name="view" />
                View
              </button>
              <button
                type="button"
                className="application-secondary"
                onClick={() => popup.open("resume-edit")}
              >
                <OverlayIcon name="edit" />
                Edit
              </button>
            </>
          )}
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

  function finish(mode: "automatic" | "edited" | "discard") {
    if (!savedRef.current || busy || aiWorking) return;
    void run(async () => {
      await popup.close();
      if (mode !== "discard" && !finalizeAnswer()) return;
      if (mode === "discard") {
        // Stop queued draft writes, then let any in-flight write settle before
        // deleting the workspace. A failed write does not block discard.
        workspacePending.current = null;
        await workspaceFlush.current?.catch(() => {});
      } else {
        await flushWorkspace();
      }
      const current = savedRef.current;
      if (!current) return;
      const entry =
        mode === "automatic"
          ? trackerEntryFor(draftRef.current ?? current.workspace)
          : mode === "edited"
            ? tracking
            : null;
      await command("finish_application", {
        expectedRevision: current.revision,
        selection: entry ? { entry } : null,
      });
      setSaved(null);
      setDraft(null);
      setFinishMode(null);
      stageOneRevision.current = null;
      stageOneSaved.current = null;
      stageOnePending.current = null;
      setStageOneStatus("saved");
      setJob("");
      setJobUrl("");
      setStyle("technical");
      setTracking(emptyTrackerEntry());
      savedRef.current = null;
      draftRef.current = null;
      workspacePending.current = null;
    });
  }

  function saveTrackerDetails() {
    return run(async () => {
      update((current) => ({
        ...current,
        roleInfo: {
          company: tracking.company,
          title: tracking.title,
          location: tracking.location,
        },
        trackerMetadata: {
          company: tracking.company,
          title: tracking.title,
          location: tracking.location,
          dateApplied: tracking.dateApplied,
          status: tracking.status,
          customStatus: tracking.customStatus,
          sourceUrl: tracking.sourceUrl,
        },
      }));
      await flushWorkspace();
      setFinishMode(null);
    });
  }

  function toggleCapture(target: "job" | "question") {
    return run(async () => {
      // An older heartbeat must not restore a cancelled generation in the UI.
      captureRefreshEpoch.current++;
      let nextMode: CaptureMode;
      if (capturing && captureMode.sessionId) {
        nextMode = await command("cancel_application_capture", {
          sessionId: captureMode.sessionId,
        });
      } else {
        nextMode = await command("request_application_capture", {
          target,
        });
      }
      captureRefreshEpoch.current++;
      setCaptureMode(nextMode);
      setNotice("");
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
      await command("resolve_application_capture", {
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
      captureRefreshEpoch.current++;
      setPendingCapture(null);
      if (!accept) return;
      if (capture.capture.payload.target === "job") {
        const updated = await command("load_application_stage_one");
        stageOneRevision.current = updated?.revision ?? null;
        stageOneSaved.current = updated?.draft ?? null;
        setJob(updated?.draft.jobDescription ?? "");
        setJobUrl(updated?.draft.jobUrl ?? "");
        setStyle(updated?.draft.style ?? "technical");
        setStageOneStatus("saved");
      } else {
        const updated = await command("load_application_workspace");
        if (updated) apply(updated);
        setTab("answers");
      }
    });
  }

  async function flushHeaderDrag(drag: NonNullable<typeof headerDrag.current>) {
    if (drag.moving) return;
    drag.moving = true;
    try {
      while (headerDrag.current === drag && drag.pending) {
        const next = drag.pending;
        drag.pending = null;
        await getCurrentWebviewWindow().setPosition(
          new PhysicalPosition(next.x, next.y),
        );
      }
    } catch (error) {
      setNotice(message(error));
    } finally {
      drag.moving = false;
    }
  }

  return (
    <main className="application-shell" inert={closePending}>
      <header
        className="application-header"
        title="Drag header to move overlay"
        onPointerDown={(event) => {
          if (
            event.button !== 0 ||
            (event.target as Element).closest("button, select, input")
          )
            return;
          const pointerId = event.pointerId;
          const screenX = event.screenX;
          const screenY = event.screenY;
          const clientX = event.clientX;
          const clientY = event.clientY;
          const header = event.currentTarget;
          header.setPointerCapture(pointerId);
          const overlayWindow = getCurrentWebviewWindow();
          void Promise.all([
            overlayWindow.outerPosition(),
            overlayWindow.outerSize(),
            overlayWindow.scaleFactor(),
            availableMonitors(),
          ])
            .then(([position, size, scale, monitors]) => {
              if (!header.hasPointerCapture(pointerId)) return;
              headerDrag.current = {
                pointerId,
                startScreenX: screenX,
                startScreenY: screenY,
                startX: position.x,
                startY: position.y,
                pointerOffsetX: clientX * scale,
                pointerOffsetY: clientY * scale,
                scale,
                size,
                monitors,
                pending: null,
                moving: false,
              };
            })
            .catch((error: unknown) => setNotice(message(error)));
        }}
        onPointerMove={(event) => {
          const drag = headerDrag.current;
          if (!drag || drag.pointerId !== event.pointerId) return;
          const x =
            drag.startX + (event.screenX - drag.startScreenX) * drag.scale;
          const y =
            drag.startY + (event.screenY - drag.startScreenY) * drag.scale;
          drag.pending = clampOverlayPosition(
            { x, y },
            drag.size,
            { x: x + drag.pointerOffsetX, y: y + drag.pointerOffsetY },
            drag.monitors,
          );
          void flushHeaderDrag(drag);
        }}
        onPointerUp={(event) => {
          if (headerDrag.current?.pointerId === event.pointerId)
            headerDrag.current = null;
          if (event.currentTarget.hasPointerCapture(event.pointerId)) {
            event.currentTarget.releasePointerCapture(event.pointerId);
          }
        }}
        onPointerCancel={(event) => {
          if (headerDrag.current?.pointerId === event.pointerId)
            headerDrag.current = null;
        }}
        onLostPointerCapture={() => {
          headerDrag.current = null;
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
                    credentialId: context!.selectedKeyId!,
                    preset: event.target.value,
                  },
                });
                setContext(await command("application_context"));
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
                className="application-stop"
                onClick={() =>
                  void command("cancel_application_generation").catch(
                    (error: unknown) => setNotice(message(error)),
                  )
                }
              >
                Stop
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
        {(capturing || captureMode.error) && (
          <p className="application-notice" role="status">
            {captureMode.error
              ? message(new Error(captureMode.error))
              : captureMode.phase === "selecting"
                ? "Move to the bottom-right corner and click to capture. Press Cancel to stop."
                : "In Chrome, click the top-left corner of the text you want to capture. Press Capture again to cancel."}
          </p>
        )}
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
              <div className="application-button-pair application-capture-actions">
                <button
                  type="button"
                  className="application-secondary"
                  disabled={
                    !capturing &&
                    (!context?.browserConnected || !!pendingCapture)
                  }
                  aria-pressed={capturing}
                  title={
                    capturing
                      ? "Cancel capture mode"
                      : "Capture text from Chrome"
                  }
                  onClick={() => void toggleCapture("job")}
                >
                  <OverlayIcon name="capture" />
                  {captureMode.phase === "selecting" ? "Cancel" : "Capture"}
                </button>
                <button
                  type="button"
                  disabled={
                    !stageOneReady ||
                    pendingCapture?.capture.payload.target === "job" ||
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
                        await command("start_application", {
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
              <label className="application-job-field">
                Job Description
                <textarea
                  aria-label="Job Description"
                  value={job}
                  onChange={(event) => setJob(event.target.value)}
                  placeholder="Capture a job description or paste it here"
                  maxLength={131072}
                  spellCheck={false}
                />
              </label>
              <label className="application-job-field">
                Job URL
                <input
                  aria-label="Job URL"
                  type="url"
                  value={jobUrl}
                  onChange={(event) => setJobUrl(event.target.value)}
                  placeholder="Job page URL (optional)"
                  maxLength={4096}
                  spellCheck={false}
                />
              </label>
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
                  {(draft.roleInfo?.company || draft.roleInfo?.title) && (
                    <div className="application-role-details">
                      {draft.roleInfo.company && (
                        <h1>{draft.roleInfo.company}</h1>
                      )}
                      {draft.roleInfo.title && <p>{draft.roleInfo.title}</p>}
                    </div>
                  )}
                  <div className="application-finish-actions">
                    <button
                      type="button"
                      className="application-secondary application-finish-primary"
                      disabled={busy || aiWorking}
                      onClick={() => finish("automatic")}
                    >
                      Finish Application
                      <OverlayIcon name="check" />
                    </button>
                    <button
                      type="button"
                      className="application-secondary application-finish-square"
                      aria-label="Edit tracker details"
                      title="Edit tracker details"
                      disabled={busy || aiWorking}
                      onClick={() =>
                        void run(async () => {
                          await popup.close();
                          const current = draftRef.current;
                          if (!current) return;
                          setTracking(trackerEntryFor(current));
                          setFinishMode("edit");
                        })
                      }
                    >
                      <OverlayIcon name="edit" />
                    </button>
                    <button
                      type="button"
                      className="application-finish-square application-finish-discard"
                      aria-label="End application without saving"
                      title="End application without saving"
                      disabled={busy || aiWorking}
                      onClick={() => setFinishMode("discard")}
                    >
                      <OverlayIcon name="close" />
                    </button>
                  </div>
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
                    {draft.alerts.length > 0 && (
                      <section
                        className="application-alerts"
                        aria-label="Qualification alerts"
                      >
                        <h2>Qualification gaps</h2>
                        <p className="application-note">
                          Based on your published resume.
                        </p>
                        <ul className="application-alert-list">
                          {draft.alerts.map((alert) => (
                            <li key={alert.id} className="application-alert">
                              {qualificationPoint(alert)}
                            </li>
                          ))}
                        </ul>
                        {draft.alertsTruncated && (
                          <p className="application-note">
                            Showing up to 10 qualification gaps.
                          </p>
                        )}
                      </section>
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
                    {draft.answer ? (
                      <h3 className="application-question-title">
                        {draft.question}
                      </h3>
                    ) : (
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
                            }));
                          }}
                          rows={4}
                          maxLength={2000}
                          placeholder="Paste an application question"
                        />
                      </label>
                    )}
                    {!!draft.answer && (
                      <label>
                        Refinement instructions
                        <textarea
                          value={answerInstruction}
                          onChange={(event) =>
                            setAnswerInstruction(event.target.value)
                          }
                          rows={3}
                          maxLength={2000}
                          placeholder="What would you like to change?"
                        />
                      </label>
                    )}
                    <button
                      type="button"
                      onClick={draft.answer ? refineAnswer : generateAnswer}
                      disabled={
                        dirty ||
                        aiWorking ||
                        !context?.aiReady ||
                        !question.trim() ||
                        (draft.answer
                          ? !answerInstruction.trim()
                          : draft.approvedAnswers.length >= 30)
                      }
                    >
                      {draft.answer ? "Refine answer" : "Generate answer"}
                    </button>
                    {!draft.answer && draft.approvedAnswers.length >= 30 && (
                      <p className="application-note" role="status">
                        This workspace already has 30 saved answers. Finish this
                        application to start another.
                      </p>
                    )}
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
                            disabled={dirty || aiWorking || busy}
                            onClick={finalizeAnswer}
                          >
                            Reset question
                          </button>
                        </div>
                      </>
                    )}
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
      {pendingCapture?.capture.payload.target === "job" &&
        (jobCaptureError || saved) &&
        !busy && (
          <div className="application-notice" role="status">
            <p>
              {saved
                ? "Finish the current application to fill in the captured job."
                : "The captured job is kept locally. Retry to fill in the fields."}
            </p>
            {!saved && (
              <button
                type="button"
                disabled={closePending}
                onClick={() => applyJobCapture(pendingCapture)}
              >
                Retry capture
              </button>
            )}
            <button
              type="button"
              disabled={closePending}
              onClick={() => resolveCapture(false)}
            >
              Discard capture
            </button>
          </div>
        )}
      {pendingCapture?.capture.payload.target === "question" && !finishMode && (
        <div className="application-dialog-backdrop">
          <section
            role="dialog"
            aria-modal="true"
            aria-label="Review browser capture"
            className="application-dialog"
          >
            <h2>Review browser capture</h2>
            <p>
              Accepting this capture saves the current answer and replaces the
              question.
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
                Trim the question to 2,000 characters before accepting it.
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
                  (pendingCapture.capture.payload.target === "question" &&
                    !saved)
                }
                onClick={() => resolveCapture(true)}
              >
                Replace current question
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
      {finishMode === "edit" && saved && (
        <div className="application-dialog-backdrop">
          <section
            role="dialog"
            aria-modal="true"
            aria-label="Edit tracker details"
            className="application-dialog"
          >
            <h2>Tracker details</h2>
            {notice && <p role="alert">{notice}</p>}
            <TrackerFields
              entry={tracking}
              disabled={busy}
              onChange={setTracking}
            />
            <div className="application-row">
              <button
                type="button"
                disabled={busy || aiWorking}
                onClick={() => finish("edited")}
              >
                Finish
              </button>
              <button
                type="button"
                className="application-secondary"
                disabled={busy}
                onClick={() => void saveTrackerDetails()}
              >
                Back
              </button>
            </div>
          </section>
        </div>
      )}
      {finishMode === "discard" && saved && (
        <div className="application-dialog-backdrop">
          <section
            role="dialog"
            aria-modal="true"
            aria-label="End application without saving"
            className="application-dialog"
          >
            <h2>End application without saving?</h2>
            <p>
              This will discard the current application and its materials
              without adding them to the tracker.
            </p>
            <div className="application-row">
              <button
                type="button"
                className="application-secondary"
                disabled={busy}
                onClick={() => setFinishMode(null)}
              >
                Back
              </button>
              <button
                type="button"
                className="application-confirm-discard"
                disabled={busy}
                onClick={() => finish("discard")}
              >
                End without saving
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
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
    | "close"
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
    close: <path d="M5 5 19 19M19 5 5 19" />,
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
