import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { CloseDecision } from "@ort/contracts/lifecycle";
import type { EditorState } from "./editor-state";
import { requestCloseStatus, resolveClose } from "./command-client";
import { closeDisposition } from "./close-policy";
import { subscribeToCloseRequests } from "./close-subscription";

export function useCloseGuard(editor: EditorState) {
  const [attempt, setAttempt] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [resolving, setResolving] = useState(false);
  const [connection, setConnection] = useState(0);
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
          setAttempt(result.value.pendingAttempt);
          setError(null);
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
      closeDisposition(editor) === "quit"
    ) {
      void resolve("quit");
    }
  }, [attempt, resolving, error, editor, resolve]);

  return {
    pending: attempt !== null,
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
