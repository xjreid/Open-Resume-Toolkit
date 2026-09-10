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
}));
vi.mock("../src/shared/command-client", () => ({
  requestCloseStatus: native.status,
  resolveClose: native.resolve,
}));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ listen: native.listen }),
}));
it("resolves Keep editing and recovers a failed quit response instead of remaining frozen", async () => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  let pending: string | null = "attempt-1";
  let wake!: () => void;
  native.listen.mockImplementation(async (_event, callback) => {
    wake = callback;
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
    close = useCloseGuard({ ...initialEditorState, status: "saving" });
    return null;
  }
  const host = document.createElement("div");
  const root = createRoot(host);
  try {
    await act(async () => root.render(<Test />));
    expect(close.pending).toBe(true);
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
