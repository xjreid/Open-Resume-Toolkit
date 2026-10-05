import { act } from "react";
import { JSDOM } from "jsdom";
import { expect, it, vi } from "vitest";
import { AppShell, type WorkspaceDestination } from "../src/shared/AppShell";

it("opens a new workspace at the top without discarding mounted work", async () => {
  const dom = new JSDOM('<div id="root"></div>');
  vi.stubGlobal("window", dom.window);
  vi.stubGlobal("document", dom.window.document);
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  const { createRoot } = await import("react-dom/client");
  const root = createRoot(dom.window.document.getElementById("root")!);
  const surface = (
    destination: WorkspaceDestination,
    overlayVisible = false,
  ) => (
    <AppShell
      destination={destination}
      onNavigate={() => {}}
      onOpenApplication={() => {}}
      overlayVisible={overlayVisible}
      navigationBlocked={false}
      status={null}
    >
      <input aria-label="Work in progress" defaultValue="Keep this draft" />
    </AppShell>
  );
  try {
    await act(async () => root.render(surface("resume")));
    const content = document.querySelector<HTMLDivElement>(".app-content")!;
    const draft = document.querySelector<HTMLInputElement>("input")!;
    draft.value = "Unsaved work";
    content.scrollTop = 350;
    content.scrollLeft = 24;
    await act(async () => root.render(surface("resume", true)));
    expect(content.scrollTop).toBe(350);
    await act(async () => root.render(surface("ai")));
    expect(content.scrollTop).toBe(0);
    expect(content.scrollLeft).toBe(0);
    expect(document.querySelector("input")).toBe(draft);
    expect(draft.value).toBe("Unsaved work");
    expect(content.querySelector(".workspace-page-heading")).toBeNull();
  } finally {
    await act(async () => root.unmount());
    dom.window.close();
    vi.unstubAllGlobals();
  }
});
