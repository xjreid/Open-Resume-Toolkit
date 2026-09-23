import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { emitTo } from "@tauri-apps/api/event";
import type { CloseDecision } from "@ort/contracts/lifecycle";
import type { EditorState } from "./editor-state";
import { requestCloseStatus, resolveClose } from "./command-client";
import { closeDisposition } from "./close-policy";
import { subscribeToCloseRequests } from "./close-subscription";
import { probeOverlayClose } from "./overlay-close-probe";

export function useCloseGuard(editor: EditorState, otherUnsavedWork = false) {
  const [attempt, setAttempt] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [resolving, setResolving] = useState(false);
  const [connection, setConnection] = useState(0);
  const [overlayDirty, setOverlayDirty] = useState(false);
  const [overlayCheckFailed, setOverlayCheckFailed] = useState(false);
  const probeGeneration = useRef(0);
  const inFlight = useRef(false);
  const mounted = useRef(false);
  const subscription = useRef<ReturnType<
    typeof subscribeToCloseRequests
  > | null>(null);

  useEffect(() => {
    mounted.current = true;
    const current = subscribeToCloseRequests(
      {
        listen: (wake) =>
          getCurrentWebviewWindow().listen("ort:close-requested", wake),
        status: requestCloseStatus,
      },
      (result) => {
        if (result.ok) {
          const pending = result.value.pendingAttempt;
          const generation = ++probeGeneration.current;
          if (!pending) {
            setAttempt(null);
            setOverlayDirty(false);
            setOverlayCheckFailed(false);
            setError(null);
            return;
          }
          void probeOverlayClose(pending, {
            listen: async (reply) =>
              getCurrentWebviewWindow().listen<{
                attempt: string;
                dirty: boolean;
              }>("ort:overlay-close-reply", (event) => reply(event.payload)),
            emit: (id) =>
              emitTo("overlay", "ort:overlay-close-probe", { attempt: id }),
          })
            .then((unsaved) => {
              if (!mounted.current || generation !== probeGeneration.current)
                return;
              setOverlayDirty(unsaved);
              setOverlayCheckFailed(false);
              setAttempt(pending);
              setError(null);
            })
            .catch(() => {
              if (!mounted.current || generation !== probeGeneration.current)
                return;
              setOverlayDirty(true);
              setOverlayCheckFailed(true);
              setAttempt(pending);
              setError(null);
            });
        } else
          setError(
            "The app could not check its quit request. Retry the connection; your editor remains open.",
          );
      },
      () =>
        setError(
          "The quit listener could not connect. Retry the connection; your editor remains open.",
        ),
    );
    subscription.current = current;
    return () => {
      mounted.current = false;
      probeGeneration.current += 1;
      current.dispose();
    };
  }, [connection]);

  const resolve = useCallback(
    async (decision: CloseDecision) => {
      if (!attempt || inFlight.current) return;
      inFlight.current = true;
      setResolving(true);
      subscription.current?.pause();
      const result = await resolveClose(attempt, decision);
      if (!mounted.current) return;
      inFlight.current = false;
      if (decision === "cancel")
        void emitTo("overlay", "ort:overlay-close-cancelled", {}).catch(
          () => {},
        );
      if (result.ok && decision === "quit") return; // Remain frozen until native exit.
      setResolving(false);
      if (result.ok) {
        setAttempt(null);
        setError(null);
        subscription.current?.resume();
      } else if (decision === "cancel") {
        // Cancel never authorizes native exit. Let the user recover/copy edits
        // even if the bridge is down instead of trapping them in a modal.
        setAttempt(null);
        setError(
          "Quit was cancelled in this editor, but the desktop connection failed. Your edits are still here; retry the connection before quitting again.",
        );
      } else {
        setError(
          "Quit was not confirmed. Your editor remains open. Retry the connection before trying again.",
        );
      }
    },
    [attempt],
  );

  useEffect(() => {
    if (
      attempt &&
      !resolving &&
      !error &&
      closeDisposition(editor, otherUnsavedWork || overlayDirty) === "quit"
    ) {
      void resolve("quit");
    }
  }, [
    attempt,
    resolving,
    error,
    editor,
    otherUnsavedWork,
    overlayDirty,
    resolve,
  ]);

  return {
    pending: attempt !== null,
    overlayDirty,
    overlayCheckFailed,
    resolving,
    error,
    cancel: () => void resolve("cancel"),
    discard: () => void resolve("quit"),
    retry: () => {
      setError(null);
      setConnection((value) => value + 1);
    },
  };
}
