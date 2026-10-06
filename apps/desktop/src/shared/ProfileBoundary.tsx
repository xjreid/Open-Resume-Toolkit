import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Fragment, useEffect, useState, type ReactNode } from "react";
// Remount every profile owner, including hidden pages and secondary windows.
export function ProfileBoundary({ children }: { children: ReactNode }) {
  const [generation, setGeneration] = useState(0);
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void getCurrentWebviewWindow()
      .listen("ort:profile-replaced", () => {
        if (active) setGeneration((value) => value + 1);
      })
      .then((unlisten) => {
        if (active) stop = unlisten;
        else unlisten();
      });
    return () => {
      active = false;
      stop?.();
    };
  }, []);
  return <Fragment key={generation}>{children}</Fragment>;
}
