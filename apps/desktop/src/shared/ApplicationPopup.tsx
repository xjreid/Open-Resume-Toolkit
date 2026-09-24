import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useCallback, useEffect, useRef, useState } from "react";
import { DOCUMENT_LIMITS } from "@ort/contracts/resume";
import { PublishedResume, ResumeCanvas } from "./App";
import type {
  ApplicationPopupChange,
  ApplicationPopupSnapshot,
  ApplicationPopupUpdate,
} from "./application-popup";
import { documentUsage } from "./resume-validation";
import "./application-popup.css";

type PopupState = ApplicationPopupSnapshot | null;

function title(kind: ApplicationPopupSnapshot["kind"]) {
  return {
    job: "Job description",
    url: "Source URL",
    "resume-view": "Resume preview",
    "resume-edit": "Edit tailored resume",
    cover: "Cover letter",
  }[kind];
}

export function ApplicationPopup() {
  const [snapshot, setSnapshot] = useState<PopupState>(null);
  const [error, setError] = useState("");
  const snapshotRef = useRef<PopupState>(null);
  const sentRef = useRef(0);
  const pendingEmitsRef = useRef<Promise<void>>(Promise.resolve());
  const finalUpdate = useCallback((): ApplicationPopupUpdate | null => {
    const current = snapshotRef.current;
    if (!current) return null;
    const change: ApplicationPopupChange | undefined =
      sentRef.current === 0
        ? undefined
        : current.kind === "job"
          ? { field: "job", value: current.job }
          : current.kind === "url"
            ? { field: "jobUrl", value: current.jobUrl }
            : current.kind === "resume-edit" && current.resume
              ? { field: "resume", value: current.resume }
              : current.kind === "cover"
                ? { field: "coverLetter", value: current.coverLetter ?? "" }
                : undefined;
    return { session: current.session, sequence: sentRef.current, change };
  }, []);
  const close = useCallback(async () => {
    try {
      await pendingEmitsRef.current;
      const update = finalUpdate();
      if (update)
        await emitTo("overlay", "ort:application-popup-close", update);
      const result = await invoke<{ ok: boolean; error?: { code: string } }>(
        "hide_application_popup",
      );
      if (result?.ok === false)
        throw new Error(result.error?.code ?? "Could not close popup");
    } catch (error) {
      setError(
        error instanceof Error
          ? error.message
          : "Could not close popup. Your edits remain here.",
      );
    }
  }, [finalUpdate]);

  const sendChange = useCallback((change: ApplicationPopupChange) => {
    const current = snapshotRef.current;
    if (!current) return;
    const next: ApplicationPopupSnapshot =
      change.field === "job"
        ? { ...current, job: change.value }
        : change.field === "jobUrl"
          ? { ...current, jobUrl: change.value }
          : change.field === "resume"
            ? { ...current, resume: change.value }
            : { ...current, coverLetter: change.value };
    // Keep controlled fields responsive while the authoritative overlay saves.
    snapshotRef.current = next;
    setSnapshot(next);
    const sequence = sentRef.current + 1;
    sentRef.current = sequence;
    pendingEmitsRef.current = pendingEmitsRef.current
      .catch(() => undefined)
      .then(() =>
        emitTo("overlay", "ort:application-popup-change", {
          session: current.session,
          sequence,
          change,
        }),
      )
      .catch((eventError: unknown) => {
        setError(
          eventError instanceof Error
            ? eventError.message
            : "Your edit is waiting to be sent.",
        );
      });
  }, []);

  useEffect(() => {
    let disposed = false;
    const stops: (() => void)[] = [];
    const connect = async () => {
      try {
        const popupWindow = getCurrentWebviewWindow();
        const unlisten = await popupWindow.listen<ApplicationPopupSnapshot>(
          "ort:application-popup-snapshot",
          (event) => {
            const incoming = event.payload;
            const current = snapshotRef.current;
            if (
              !incoming ||
              (current && incoming.generation < current.generation) ||
              (current &&
                incoming.generation === current.generation &&
                incoming.session === current.session &&
                incoming.revision < current.revision)
            )
              return;
            if (!current || incoming.generation > current.generation)
              sentRef.current = 0;
            const next =
              current &&
              incoming.session === current.session &&
              incoming.acknowledgedEditSequence < sentRef.current
                ? {
                    ...incoming,
                    job: current.job,
                    jobUrl: current.jobUrl,
                    resume: current.resume,
                    coverLetter: current.coverLetter,
                  }
                : incoming;
            snapshotRef.current = next;
            setSnapshot(next);
            setError("");
          },
        );
        if (disposed) {
          unlisten();
          return;
        }
        stops.push(unlisten);
        const stopFlush = await popupWindow.listen<{
          session: string;
          requestId: string;
        }>("ort:application-popup-flush", (event) => {
          const update = finalUpdate();
          if (!update || update.session !== event.payload.session) return;
          void emitTo("overlay", "ort:application-popup-flushed", {
            ...update,
            requestId: event.payload.requestId,
          }).catch(() =>
            setError("Could not send your edits. Please try again."),
          );
        });
        if (disposed) {
          stopFlush();
          return;
        }
        stops.push(stopFlush);
        await emitTo("overlay", "ort:application-popup-ready", {});
      } catch (connectError) {
        if (!disposed)
          setError(
            connectError instanceof Error
              ? connectError.message
              : "Could not connect popup.",
          );
      }
    };
    void connect();
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void close();
      }
    };
    globalThis.window.addEventListener("keydown", escape);
    return () => {
      globalThis.window.removeEventListener("keydown", escape);
      stops.forEach((stop) => stop());
    };
  }, [close, finalUpdate]);

  if (!snapshot)
    return (
      <main className="application-popup application-popup--loading">
        {error || "Opening…"}
      </main>
    );
  const canAddEntry = snapshot.resume
    ? documentUsage(snapshot.resume).entries < DOCUMENT_LIMITS.entries
    : false;
  return (
    <main className="application-popup" aria-label={title(snapshot.kind)}>
      <header className="application-popup__header">
        <h1>{title(snapshot.kind)}</h1>
        <button
          type="button"
          className="application-popup__close"
          onClick={() => void close()}
          aria-label="Close popup"
        >
          ×
        </button>
      </header>
      <section className="application-popup__body">
        {error && (
          <p className="application-popup__error" role="alert">
            {error}
          </p>
        )}
        {snapshot.kind === "job" && (
          <label className="application-popup__field">
            Job description
            <textarea
              autoFocus
              maxLength={131072}
              value={snapshot.job}
              disabled={snapshot.disabled}
              onChange={(event) =>
                sendChange({ field: "job", value: event.target.value })
              }
            />
          </label>
        )}
        {snapshot.kind === "url" && (
          <label className="application-popup__field">
            Source URL
            <input
              autoFocus
              type="url"
              maxLength={4096}
              value={snapshot.jobUrl}
              disabled={snapshot.disabled}
              onChange={(event) =>
                sendChange({ field: "jobUrl", value: event.target.value })
              }
            />
          </label>
        )}
        {snapshot.kind === "resume-view" && snapshot.resume && (
          <PublishedResume
            document={snapshot.resume}
            style={snapshot.style}
            contactDivider="dot"
          />
        )}
        {snapshot.kind === "resume-edit" && snapshot.resume && (
          <ResumeCanvas
            document={snapshot.resume}
            style={snapshot.style}
            contactDivider="dot"
            onContactDividerChange={() => undefined}
            showContactDivider={false}
            disabled={snapshot.disabled}
            canAddEntry={canAddEntry}
            onChange={(update) =>
              sendChange({ field: "resume", value: update(snapshot.resume!) })
            }
          />
        )}
        {snapshot.kind === "cover" && (
          <label className="application-popup__field">
            Cover letter
            <textarea
              autoFocus
              maxLength={12000}
              value={snapshot.coverLetter ?? ""}
              disabled={snapshot.disabled}
              onChange={(event) =>
                sendChange({ field: "coverLetter", value: event.target.value })
              }
            />
          </label>
        )}
      </section>
    </main>
  );
}
