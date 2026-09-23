import { normalizeSelection, sanitizeUrl } from "./selection.js";

declare const chrome: {
  runtime: {
    onMessage: {
      addListener: (
        listener: (
          message: unknown,
          sender: unknown,
          reply: (response: unknown) => void,
        ) => boolean | void,
      ) => void;
    };
    sendNativeMessage: (
      name: string,
      message: unknown,
      callback: (response?: unknown) => void,
    ) => void;
    lastError?: { message: string };
  };
  scripting: {
    executeScript: (
      options: { target: { tabId: number }; func: () => unknown },
      callback: (results?: { result: unknown }[]) => void,
    ) => void;
  };
  tabs: {
    query: (
      query: { active: boolean; currentWindow: boolean },
      callback: (tabs: { id?: number; url?: string }[]) => void,
    ) => void;
  };
};

type Request = { kind: "capture"; target: "job" | "question" };
type Selection = { text: string; url: string; title: string };
const HOST = "com.openresumetoolkit.dev";

function selectedText(): Selection {
  return {
    text: String(window.getSelection() ?? ""),
    url: location.href,
    title: document.title,
  };
}

function capture(
  target: Request["target"],
): Promise<{ ok: boolean; message: string }> {
  return new Promise((resolve) =>
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const tab = tabs[0];
      if (!tab?.id || !tab.url?.startsWith("http")) {
        resolve({
          ok: false,
          message: "Open a normal job page and select the text first.",
        });
        return;
      }
      chrome.scripting.executeScript(
        { target: { tabId: tab.id }, func: selectedText },
        (results) => {
          if (chrome.runtime.lastError || !results?.[0]?.result) {
            resolve({
              ok: false,
              message: "This page does not allow selection capture.",
            });
            return;
          }
          try {
            const raw = results[0].result as Selection;
            const text = normalizeSelection(raw.text);
            const url = sanitizeUrl(raw.url);
            const title = raw.title.normalize("NFC").slice(0, 500);
            const message = {
              protocolVersion: 1,
              requestId: crypto.randomUUID(),
              sentAt: new Date().toISOString(),
              kind: "capture.selection",
              payload: {
                text,
                url,
                title,
                target,
                browser: navigator.userAgent.includes("Edg/")
                  ? "edge"
                  : "chrome",
              },
            };
            if (
              new TextEncoder().encode(JSON.stringify(message)).length >
              256 * 1024
            )
              throw new Error("Selection exceeds the bridge limit.");
            chrome.runtime.sendNativeMessage(HOST, message, (response) => {
              if (chrome.runtime.lastError) {
                resolve({
                  ok: false,
                  message:
                    "Desktop connection unavailable. Open ORT and repair Browser connections.",
                });
                return;
              }
              const value = response as
                | { ok?: boolean; error?: { code?: string } }
                | undefined;
              resolve(
                value?.ok === true
                  ? {
                      ok: true,
                      message: "Selection sent. Review it in the ORT overlay.",
                    }
                  : {
                      ok: false,
                      message:
                        value?.error?.code === "PROTOCOL_INCOMPATIBLE"
                          ? "Update or repair the ORT browser connection."
                          : "ORT did not accept the selection. Your page is unchanged.",
                    },
              );
            });
          } catch (error) {
            resolve({
              ok: false,
              message:
                error instanceof Error ? error.message : "Invalid selection.",
            });
          }
        },
      );
    }),
  );
}

chrome.runtime.onMessage.addListener((message: unknown, _sender, reply) => {
  const request = message as Request;
  if (
    request?.kind !== "capture" ||
    !["job", "question"].includes(request.target)
  )
    return;
  void capture(request.target).then(reply);
  return true;
});
