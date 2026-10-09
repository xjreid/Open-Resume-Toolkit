import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type * as Wire from "@ort/contracts/wire";
import { invokeDesktop as invoke } from "./desktop-client";
import type { DesktopResponse } from "./desktop-client";
import { planErrorMessage } from "./chatgpt-plan-presentation";

/** One polling owner and one mutation at a time. Late snapshots cannot undo an action. */
export function useChatGptPlan(
  visible: boolean,
  status: Wire.PlanStatus | null,
  onStatus: (status: Wire.PlanStatus) => void,
) {
  const [working, setWorking] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [notice, setNotice] = useState("");
  const current = useRef(status);
  const publish = useRef(onStatus);
  current.current = status;
  publish.current = onStatus;
  const mounted = useRef(true);
  const generation = useRef(0);
  const polling = useRef(false);
  const mutating = useRef(false);
  const pendingRefresh = useRef<boolean | null>(null);
  const apply = useCallback((next: Wire.PlanStatus) => {
    current.current = next;
    publish.current(next);
  }, []);

  const refresh = useCallback(
    async (usage = false) => {
      if (!mounted.current) return;
      if (polling.current || mutating.current) {
        pendingRefresh.current = (pendingRefresh.current ?? false) || usage;
        return;
      }
      polling.current = true;
      setRefreshing(true);
      const version = generation.current;
      try {
        const response = await invoke("load_chatgpt_plan", {
          request: { refreshUsage: usage },
        });
        if (!mounted.current || version !== generation.current) return;
        if (response.ok) apply(response.value);
        else setNotice(planErrorMessage(response.error.code));
      } catch {
        if (mounted.current && version === generation.current)
          setNotice(
            "The native Codex connection is unavailable. Refresh to try again.",
          );
      } finally {
        polling.current = false;
        if (mounted.current) setRefreshing(false);
        const pending = pendingRefresh.current;
        pendingRefresh.current = null;
        if (pending !== null) void refresh(pending);
      }
    },
    [apply],
  );

  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    const wake = () => {
      if (!active) return;
      if (!mutating.current) generation.current++;
      // A model event reloads settings immediately; fresh quota has its own
      // polling cycle and must not delay synchronizing the selected controls.
      void refresh(false);
    };
    window.addEventListener("focus", wake);
    void listen("ort:ai-model-changed", wake)
      .then((unlisten) => {
        if (active) stop = unlisten;
        else unlisten();
      })
      .catch(() => {});
    return () => {
      active = false;
      window.removeEventListener("focus", wake);
      stop?.();
    };
  }, [visible, refresh]);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      generation.current++;
    };
  }, []);
  useEffect(() => {
    void refresh(visible);
    if (!visible) return;
    const timer = window.setInterval(() => void refresh(true), 30_000);
    return () => {
      clearInterval(timer);
    };
  }, [visible, refresh]);
  useEffect(() => {
    if (!visible || !status?.loginPending) return;
    const timer = window.setInterval(() => void refresh(false), 1_500);
    return () => clearInterval(timer);
  }, [visible, status?.loginPending, refresh]);
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void listen<boolean>("ort:ai-operation-state", (event) => {
      if (!active || typeof event?.payload !== "boolean") return;
      if (!mutating.current) generation.current++;
      if (current.current)
        apply({ ...current.current, operationActive: event.payload });
      if (!event.payload && visible) void refresh(true);
    }).then((unlisten) => {
      if (active) stop = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      stop?.();
    };
  }, [visible, apply, refresh]);

  async function mutate(
    action: () => Promise<DesktopResponse<Wire.PlanStatus>>,
  ) {
    if (mutating.current) return;
    mutating.current = true;
    const version = ++generation.current;
    setWorking(true);
    setNotice("");
    try {
      const response = await action();
      if (!mounted.current || version !== generation.current) return;
      if (response.ok) apply(response.value);
      else setNotice(planErrorMessage(response.error.code));
    } catch {
      if (mounted.current && version === generation.current)
        setNotice("The action could not finish. Refresh and try again.");
    } finally {
      mutating.current = false;
      if (mounted.current) setWorking(false);
      const pending = pendingRefresh.current;
      pendingRefresh.current = null;
      if (pending !== null) void refresh(pending);
    }
  }
  function act(action: "connect" | "cancel" | "disconnect") {
    return mutate(() =>
      action === "connect"
        ? invoke("connect_chatgpt_plan")
        : action === "cancel"
          ? invoke("cancel_chatgpt_login")
          : invoke("disconnect_chatgpt_plan"),
    );
  }
  function save(change: Partial<Wire.PlanSettings>) {
    const baseline = current.current;
    if (!baseline) return;
    const next = { ...baseline.settings, ...change };
    return mutate(() =>
      invoke("save_chatgpt_plan", {
        request: {
          expectedRevision: baseline.revision,
          enabled: next.enabled,
          model: next.model,
          reasoning: next.reasoning,
          reserveEnabled: next.reserveEnabled,
          reservePercent: next.reservePercent,
        },
      }),
    );
  }
  return { working, refreshing, notice, refresh, act, save, setNotice };
}
