// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { BrowserConnectionSettings } from "./BrowserConnectionSettings";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
const container = document.createElement("div");
let root: ReturnType<typeof createRoot>;
afterEach(async () => {
  if (root) await act(async () => root.unmount());
  invoke.mockReset();
});
it("enables and disables explicitly and refreshes native status", async () => {
  let connected = false;
  invoke.mockImplementation(async (command: string) => {
    if (command === "browser_connection_status")
      return { ok: true, value: { available: true, connected } };
    if (command === "connect_development_browser") connected = true;
    else if (command === "disconnect_development_browser") connected = false;
    else throw new Error("Unexpected command");
    return { ok: true, value: true };
  });
  root = createRoot(container);
  await act(async () => root.render(<BrowserConnectionSettings />));
  expect(container.textContent).toContain("Development connection disabled.");
  await act(async () => container.querySelector("button")!.click());
  expect(container.textContent).toContain("Development connection enabled.");
  await act(async () => container.querySelector("button")!.click());
  expect(container.textContent).toContain("Development connection disabled.");
});
it("shows an automatically enabled connection and can disable it for this run", async () => {
  let connected = true;
  invoke.mockImplementation(async (command: string) => {
    if (command === "browser_connection_status")
      return { ok: true, value: { available: true, connected } };
    if (command === "disconnect_development_browser") {
      connected = false;
      return { ok: true, value: true };
    }
    throw new Error("Unexpected command");
  });
  root = createRoot(container);
  await act(async () => root.render(<BrowserConnectionSettings />));
  expect(container.querySelector("button")?.textContent).toBe(
    "Disable development connection",
  );
  await act(async () => container.querySelector("button")!.click());
  expect(container.textContent).toContain("Development connection disabled.");
});
it("does not claim readiness when setup fails or the feature is absent", async () => {
  invoke
    .mockResolvedValueOnce({
      ok: true,
      value: { available: true, connected: false },
    })
    .mockResolvedValueOnce({
      ok: false,
      error: { code: "DEV_BRIDGE_SETUP_REQUIRED" },
    });
  root = createRoot(container);
  await act(async () => root.render(<BrowserConnectionSettings />));
  await act(async () => container.querySelector("button")!.click());
  expect(container.querySelector('[role="alert"]')?.textContent).toContain(
    "setup failed",
  );
  expect(container.textContent).toContain("Development connection disabled.");
  await act(async () => root.unmount());
  invoke.mockResolvedValue({
    ok: true,
    value: { available: false, connected: false },
  });
  root = createRoot(container);
  await act(async () => root.render(<BrowserConnectionSettings />));
  expect(container.querySelector("button")).toBeNull();
  expect(container.textContent).toContain("unavailable in this app build");
});
