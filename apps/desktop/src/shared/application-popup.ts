import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useCallback, useEffect, useRef } from "react";
import type { DocumentStyle } from "@ort/contracts/export";
import type { ResumeDocument } from "@ort/contracts/resume";

export type ApplicationPopupKind =
  | "job"
  | "url"
  | "resume-view"
  | "resume-edit"
  | "cover";
export type ApplicationPopupSnapshot = {
  session: string;
  generation: number;
  revision: number;
  acknowledgedEditSequence: number;
  kind: ApplicationPopupKind;
  job: string;
  jobUrl: string;
  resume: ResumeDocument | null;
  style: DocumentStyle;
  coverLetter: string | null;
  disabled: boolean;
};
export type ApplicationPopupChange =
  | { field: "job"; value: string }
  | { field: "jobUrl"; value: string }
  | { field: "resume"; value: ResumeDocument }
  | { field: "coverLetter"; value: string };
export type ApplicationPopupUpdate = {
  session: string;
  sequence: number;
  change?: ApplicationPopupChange;
};
export type UseApplicationPopupOptions = Omit<
  ApplicationPopupSnapshot,
  "session" | "generation" | "revision" | "acknowledgedEditSequence" | "kind"
> & {
  onJobChange: (value: string) => void;
  onUrlChange: (value: string) => void;
  onResumeChange: (value: ResumeDocument) => void;
  onCoverChange: (value: string) => void;
  onError: (message: string) => void;
};
async function popupCommand(name: string, args: Record<string, unknown> = {}) {
  const result = await invoke<{ ok: boolean; error?: { code: string } }>(
    name,
    args,
  );
  if (result?.ok === false)
    throw new Error(result.error?.code ?? "Popup unavailable");
}

/** The overlay owns persistence; popups submit sequenced edits and final flushes. */
export function useApplicationPopup(options: UseApplicationPopupOptions) {
  const optionsRef = useRef(options);
  const activeRef = useRef<{
    session: string;
    kind: ApplicationPopupKind;
    generation: number;
    acknowledgedEditSequence: number;
  } | null>(null);
  const revisionRef = useRef(0);
  const generationRef = useRef(Date.now());
  const operations = useRef<Promise<void>>(Promise.resolve());
  const pendingFlushes = useRef(
    new Map<
      string,
      { resolve: () => void; reject: (error: Error) => void; timer: number }
    >(),
  );
  optionsRef.current = options;

  const accept = useCallback((update: ApplicationPopupUpdate) => {
    const active = activeRef.current;
    if (
      !active ||
      update.session !== active.session ||
      !update.change ||
      update.sequence <= active.acknowledgedEditSequence
    )
      return;
    active.acknowledgedEditSequence = update.sequence;
    const current = optionsRef.current;
    switch (update.change.field) {
      case "job":
        optionsRef.current = { ...current, job: update.change.value };
        current.onJobChange(update.change.value);
        break;
      case "jobUrl":
        optionsRef.current = { ...current, jobUrl: update.change.value };
        current.onUrlChange(update.change.value);
        break;
      case "resume":
        optionsRef.current = { ...current, resume: update.change.value };
        current.onResumeChange(update.change.value);
        break;
      case "coverLetter":
        optionsRef.current = { ...current, coverLetter: update.change.value };
        current.onCoverChange(update.change.value);
        break;
    }
  }, []);
  const sendSnapshot = useCallback(async () => {
    const active = activeRef.current;
    if (!active) return;
    const { job, jobUrl, resume, style, coverLetter, disabled } =
      optionsRef.current;
    await emitTo("application-popup", "ort:application-popup-snapshot", {
      ...active,
      revision: ++revisionRef.current,
      job,
      jobUrl,
      resume,
      style,
      coverLetter,
      disabled,
    } satisfies ApplicationPopupSnapshot);
  }, []);
  const report = useCallback(
    (error: unknown) =>
      optionsRef.current.onError(
        error instanceof Error ? error.message : "Could not connect the popup.",
      ),
    [],
  );

  useEffect(() => {
    const window = getCurrentWebviewWindow();
    const subscriptions = [
      window.listen(
        "ort:application-popup-ready",
        () => void sendSnapshot().catch(report),
      ),
      window.listen<ApplicationPopupUpdate>(
        "ort:application-popup-change",
        (event) => {
          accept(event.payload);
          void sendSnapshot().catch(report);
        },
      ),
      window.listen<ApplicationPopupUpdate>(
        "ort:application-popup-close",
        (event) => {
          // The final value is included, so a delayed typing event cannot be lost.
          accept(event.payload);
          if (event.payload.session === activeRef.current?.session)
            activeRef.current = null;
        },
      ),
      window.listen<ApplicationPopupUpdate & { requestId: string }>(
        "ort:application-popup-flushed",
        (event) => {
          if (event.payload.session !== activeRef.current?.session) return;
          accept(event.payload);
          const pending = pendingFlushes.current.get(event.payload.requestId);
          if (pending) {
            globalThis.window.clearTimeout(pending.timer);
            pendingFlushes.current.delete(event.payload.requestId);
            pending.resolve();
          }
        },
      ),
    ];
    for (const subscription of subscriptions) void subscription.catch(report);
    return () => {
      for (const subscription of subscriptions)
        void subscription.then((stop) => stop()).catch(() => {});
      for (const pending of pendingFlushes.current.values()) {
        globalThis.window.clearTimeout(pending.timer);
        pending.reject(new Error("Popup disconnected"));
      }
      pendingFlushes.current.clear();
    };
  }, [accept, sendSnapshot, report]);
  useEffect(() => {
    void sendSnapshot().catch(report);
  }, [
    options.job,
    options.jobUrl,
    options.resume,
    options.style,
    options.coverLetter,
    options.disabled,
    sendSnapshot,
    report,
  ]);

  const flush = useCallback(async () => {
    const active = activeRef.current;
    if (!active) return;
    const requestId = crypto.randomUUID();
    await new Promise<void>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        pendingFlushes.current.delete(requestId);
        reject(
          new Error(
            "The popup did not respond. Close it and try again; your edits remain visible there.",
          ),
        );
      }, 2000);
      pendingFlushes.current.set(requestId, { resolve, reject, timer });
      void emitTo("application-popup", "ort:application-popup-flush", {
        session: active.session,
        requestId,
      }).catch((error: unknown) => {
        window.clearTimeout(timer);
        pendingFlushes.current.delete(requestId);
        reject(error);
      });
    });
  }, []);
  // Serialize native show/hide operations so a late show cannot undo a close.
  const enqueue = useCallback((action: () => Promise<void>) => {
    const next = operations.current.catch(() => {}).then(action);
    operations.current = next;
    return next;
  }, []);
  const open = useCallback(
    (kind: ApplicationPopupKind) =>
      enqueue(async () => {
        await flush();
        activeRef.current = {
          session: crypto.randomUUID(),
          kind,
          generation: ++generationRef.current,
          acknowledgedEditSequence: 0,
        };
        revisionRef.current = 0;
        try {
          await popupCommand("show_application_popup", { kind });
          await sendSnapshot();
        } catch (error) {
          activeRef.current = null;
          throw error;
        }
      }).catch(report),
    [enqueue, flush, sendSnapshot, report],
  );
  const close = useCallback(
    () =>
      enqueue(async () => {
        await flush();
        await popupCommand("hide_application_popup");
        activeRef.current = null;
      }),
    [enqueue, flush],
  );
  return { open, close, flush };
}
