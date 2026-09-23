// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { useCloseGuard } from "../src/shared/use-close-guard";
import { initialEditorState } from "../src/shared/editor-state";
const native = vi.hoisted(() => ({
  status: vi.fn(),
  resolve: vi.fn(),
  listen: vi.fn(),
  reply: vi.fn(),
}));
vi.mock("../src/shared/command-client", () => ({
  requestCloseStatus: native.status,
  resolveClose: native.resolve,
}));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ listen: native.listen }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  emitTo: vi.fn(
    async (_target: string, event: string, payload: { attempt?: string }) => {
      if (event === "ort:overlay-close-probe")
        native.reply({ payload: { attempt: payload.attempt, dirty: true } });
    },
  ),
}));
it("resolves Keep editing and recovers a failed quit response instead of remaining frozen", async () => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  let pending: string | null = "attempt-1";
  let wake!: () => void;
  native.listen.mockImplementation(async (event, callback) => {
    if (event === "ort:close-requested") wake = callback;
    if (event === "ort:overlay-close-reply") native.reply = callback;
    return () => {};
  });
  native.status.mockImplementation(async () => ({
    ok: true,
    value: { pendingAttempt: pending },
  }));
  native.resolve.mockImplementation(async () => {
    pending = null;
    return { ok: true, value: { pendingAttempt: null } };
  });
  let close!: ReturnType<typeof useCloseGuard>;
  function Test() {
    close = useCloseGuard(initialEditorState);
    return null;
  }
  const host = document.createElement("div");
  const root = createRoot(host);
  try {
    await act(async () => root.render(<Test />));
    expect(close.pending).toBe(true);
    expect(close.overlayDirty).toBe(true);
    expect(native.resolve).not.toHaveBeenCalled();
    await act(async () => close.cancel());
    expect(close.pending).toBe(false);
    expect(close.resolving).toBe(false);
    pending = "attempt-2";
    await act(async () => wake());
    native.resolve.mockResolvedValue({
      ok: false,
      error: { code: "EXPORT_BUSY" },
    });
    await act(async () => close.discard());
    expect(close.resolving).toBe(false);
    expect(close.error).toContain("Quit was not confirmed");
  } finally {
    await act(async () => root.unmount());
    vi.unstubAllGlobals();
  }
});
