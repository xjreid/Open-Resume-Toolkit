export {};
declare const chrome: {
  runtime: {
    sendMessage: (
      message: unknown,
      callback: (response?: unknown) => void,
    ) => void;
    lastError?: { message: string };
  };
};
type CaptureResponse = { ok: boolean; message: string };
const statusNode = document.getElementById("status")!;
for (const target of ["job", "question"] as const) {
  document.getElementById(target)!.addEventListener("click", () => {
    statusNode.textContent = "Connecting…";
    for (const button of document.querySelectorAll("button"))
      button.disabled = true;
    chrome.runtime.sendMessage({ kind: "capture", target }, (response) => {
      const result = response as CaptureResponse | undefined;
      statusNode.textContent = chrome.runtime.lastError
        ? "Capture unavailable. Reopen ORT and try again."
        : (result?.message ?? "No response from browser connection.");
      for (const button of document.querySelectorAll("button"))
        button.disabled = false;
    });
  });
}
