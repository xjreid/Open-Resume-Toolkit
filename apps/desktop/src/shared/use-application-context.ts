import {
  useEffect,
  useRef,
  type MutableRefObject,
  type Dispatch,
  type SetStateAction,
} from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { DesktopCommands } from "@ort/contracts/wire";
import { desktopCommand as command } from "./desktop-client";
import { ContextRefresh } from "./context-refresh";

type Value<K extends keyof DesktopCommands> = DesktopCommands[K]["value"];
type Options = {
  epoch: MutableRefObject<number>;
  setContext: Dispatch<SetStateAction<Value<"application_context"> | null>>;
  setCapture: Dispatch<SetStateAction<Value<"load_application_capture">>>;
  setMode: Dispatch<SetStateAction<Value<"application_capture_status">>>;
  onError: (error: unknown) => void;
};
function updateIfChanged<T>(set: Dispatch<SetStateAction<T>>, next: T) {
  set((previous) =>
    JSON.stringify(previous) === JSON.stringify(next) ? previous : next,
  );
}

export function useApplicationContext(options: Options) {
  const latest = useRef(options);
  latest.current = options;
  useEffect(() => {
    let active = true;
    let visible = false;
    let visibilityEpoch = 0;
    const overlay = getCurrentWebviewWindow();
    const refresh = new ContextRefresh(async () => {
      const epoch = ++latest.current.epoch.current;
      try {
        const [capture, context, mode] = await Promise.all([
          command("load_application_capture"),
          command("application_context"),
          command("application_capture_status"),
        ]);
        if (!active || !visible || epoch !== latest.current.epoch.current)
          return;
        updateIfChanged(latest.current.setCapture, capture);
        updateIfChanged<Value<"application_context"> | null>(
          latest.current.setContext,
          context,
        );
        updateIfChanged(latest.current.setMode, mode);
      } catch (error) {
        if (active && visible && epoch === latest.current.epoch.current)
          latest.current.onError(error);
      }
    });
    const setVisible = (next: boolean) => {
      if (!active) return;
      visible = next;
      if (!next) latest.current.epoch.current++;
      refresh.setVisible(next);
    };
    const focus = () => {
      const epoch = ++visibilityEpoch;
      void overlay
        .isVisible()
        .then((next) => {
          if (epoch === visibilityEpoch) setVisible(next);
        })
        .catch(() => {
          if (epoch === visibilityEpoch) setVisible(false);
        });
    };
    const subscriptions = Promise.all([
      overlay.listen<boolean>("ort:overlay-visibility", (event) => {
        visibilityEpoch++;
        setVisible(event.payload);
      }),
      overlay.listen("ort:browser-capture", refresh.refresh),
      overlay.listen("ort:capture-mode", refresh.refresh),
      overlay.listen("ort:ai-model-changed", refresh.refresh),
    ])
      .then((stops) => {
        focus();
        return stops;
      })
      .catch(() => {
        focus();
        return [];
      });
    window.addEventListener("focus", focus);
    window.addEventListener("blur", focus);
    return () => {
      active = false;
      refresh.stop();
      window.removeEventListener("focus", focus);
      window.removeEventListener("blur", focus);
      void subscriptions.then((stops) => stops.forEach((stop) => stop()));
    };
  }, []);
}
