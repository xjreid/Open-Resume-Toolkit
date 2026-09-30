import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Status = { available: boolean; connected: boolean };
type Response<T> =
  | { ok: true; value: T }
  | { ok: false; error: { code: string } };

export function BrowserConnectionSettings() {
  const [status, setStatus] = useState<Status>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function refresh() {
    const response = await invoke<Response<Status>>(
      "browser_connection_status",
    );
    if (!response.ok) throw new Error(response.error.code);
    setStatus(response.value);
  }
  useEffect(() => {
    void refresh().catch(() =>
      setError("Could not check the browser connection."),
    );
  }, []);
  async function change() {
    setBusy(true);
    setError("");
    try {
      const response = await invoke<Response<boolean>>(
        status?.connected
          ? "disconnect_development_browser"
          : "connect_development_browser",
      );
      if (!response.ok) throw new Error(response.error.code);
      await refresh();
    } catch {
      setError(
        "Connection setup failed. Run the development host setup command, then try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <section aria-labelledby="browser-connection-title" aria-busy={busy}>
      <h3 id="browser-connection-title">Chrome development connection</h3>
      <p role="status">
        {status
          ? status.connected
            ? "Development connection enabled."
            : "Development connection disabled."
          : "Checking connection…"}
      </p>
      {status?.available ? (
        <>
          <p>
            Enable this connection while testing the BETA extension. Press
            Capture on the ORT overlay, then click the top-left and bottom-right
            corners of the text in Chrome. ORT will ask you to review each
            capture.
          </p>
          <p>
            This development connection trusts programs running under your macOS
            account. It turns off when you quit ORT.
          </p>
          <button type="button" disabled={busy} onClick={() => void change()}>
            {status.connected
              ? "Disable development connection"
              : "Enable development connection"}
          </button>
        </>
      ) : status ? (
        <p>
          Browser capture is unavailable in this app build. Open the updated ORT
          development app to test Chrome capture.
        </p>
      ) : null}
      {error && <p role="alert">{error}</p>}
      <p>Edge support will come later.</p>
    </section>
  );
}
