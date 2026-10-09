import { useCallback, useEffect, useRef, useState } from "react";
import type { RuntimeReadiness } from "@ort/contracts/wire";
import { invokeDesktop as invoke } from "./desktop-client";
import { planErrorMessage } from "./chatgpt-plan-presentation";

// Installation is global to the machine, independent of the enabled provider
// or memory-only account session. Check on page entry, focus and installation.
export function useCodexRuntimeReadiness(visible: boolean) {
  const [readiness, setReadiness] = useState<RuntimeReadiness | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState("");
  const generation = useRef(0);
  const active = useRef(false);
  const refresh = useCallback(async () => {
    if (!active.current) return;
    const request = ++generation.current;
    setChecking(true);
    setError("");
    try {
      const response = await invoke("check_codex_runtime");
      if (!active.current || request !== generation.current) return;
      if (response.ok) setReadiness(response.value);
      else {
        setReadiness(null);
        setError(planErrorMessage(response.error.code));
      }
    } catch {
      if (!active.current || request !== generation.current) return;
      setReadiness(null);
      setError(planErrorMessage("PLAN_RUNTIME_CHECK_FAILED"));
    } finally {
      if (active.current && request === generation.current) setChecking(false);
    }
  }, []);
  useEffect(() => {
    active.current = visible;
    if (!visible) return;
    void refresh();
    const onFocus = () => void refresh();
    window.addEventListener("focus", onFocus);
    return () => {
      active.current = false;
      generation.current++;
      window.removeEventListener("focus", onFocus);
    };
  }, [visible, refresh]);
  return { readiness, checking, error, refresh };
}
