// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { createResumeDocument } from "./resume-editor";
import { ApplicationOverlay } from "./ApplicationOverlay";
import type { UseApplicationPopupOptions } from "./application-popup";

const { listeners, popup } = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  popup: {
    options: null as UseApplicationPopupOptions | null,
    open: vi.fn(),
    close: vi.fn(),
  },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ emitTo: vi.fn() }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    listen: (name: string, handler: (event: { payload: unknown }) => void) => {
      listeners.set(name, handler);
      return Promise.resolve(() => listeners.delete(name));
    },
    startDragging: () => Promise.resolve(),
  }),
}));
vi.mock("./application-popup", () => ({
  useApplicationPopup: (options: UseApplicationPopupOptions) => {
    popup.options = options;
    return {
      open: popup.open,
      close: popup.close,
      flush: () => Promise.resolve(),
    };
  },
}));
const context = {
  publishedRevision: 1,
  aiLabel: "Balanced: Gemini test",
  aiReady: true,
  aiBusy: false,
  selectedKeyId: "key-one",
  preset: "balanced",
  presetOptions: [
    { preset: "economy", label: "Economy: Gemini small", model: "small" },
    { preset: "balanced", label: "Balanced: Gemini test", model: "test" },
  ],
  browserConnected: false,
};
function workspace() {
  return {
    schemaVersion: 1,
    publishedRevision: 1,
    jobDescription: "Job",
    jobUrl: "",
    roleInfo: { company: "Example", title: "Engineer", location: "" },
    resume: createResumeDocument(),
    changePoints: ["Prioritize the documented Rust experience."],
    alerts: [],
    alertsTruncated: false,
    dismissedAlertIds: [],
    ignoreAllAlerts: false,
    coverLetter: null,
    question: "",
    answer: "",
    approvedAnswers: [],
    style: "technical",
  };
}
function reply(value: unknown) {
  return { ok: true, value };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
const cleanups: (() => Promise<void>)[] = [];
async function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<ApplicationOverlay />));
  cleanups.push(async () => {
    await act(async () => root.unmount());
    host.remove();
  });
  const button = (label: string) => {
    const element = [...host.querySelectorAll("button")].find(
      (item) =>
        item.textContent?.trim() === label ||
        item.getAttribute("aria-label") === label,
    );
    if (!element) throw new Error(`Missing button: ${label}`);
    return element;
  };
  return { host, button };
}
afterEach(async () => {
  for (const cleanup of cleanups.splice(0)) await cleanup();
  vi.useRealTimers();
  vi.clearAllMocks();
  listeners.clear();
  popup.options = null;
});

it("keeps Stage 1 compact and saves popup job edits before tailoring", async () => {
  const calls: string[] = [];
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    calls.push(name);
    const input = args as Record<string, unknown>;
    if (name === "application_context") return reply(context);
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_stage_one")
      return reply({ revision: 1, draft: input.draft });
    if (name === "start_application")
      return reply({ revision: 1, workspace: workspace() });
    if (name === "prepare_application_exports")
      return reply({ revision: 1, pdfReady: true, docxReady: true });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  expect(host.querySelector("textarea")).toBeNull();
  expect(host.textContent).not.toContain("Resume style");
  expect(button("Capture").disabled).toBe(true);
  expect(button("Tailor").disabled).toBe(true);
  await act(async () => button("View Job Description").click());
  expect(popup.open).toHaveBeenCalledWith("job");
  await act(async () => popup.options?.onJobChange("Rust engineer required"));
  expect(button("Tailor").disabled).toBe(false);
  expect(host.textContent).toContain("Complete");
  await act(async () => button("Tailor").click());
  expect(calls.indexOf("start_application")).toBeGreaterThan(
    calls.indexOf("save_application_stage_one"),
  );
  expect(invoke).toHaveBeenCalledWith("start_application", {
    jobDescription: "Rust engineer required",
    jobUrl: "",
    style: "technical",
  });
  expect(host.textContent).toContain("Example");
  expect(
    host.querySelector('ul[aria-label="Tailoring notes"]')?.textContent,
  ).toContain("documented Rust");
});

it("requires an active key even when a job exists and routes connected capture + presets", async () => {
  let activeContext = {
    ...context,
    aiReady: false,
    selectedKeyId: null as string | null,
    browserConnected: true,
  };
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(activeContext);
    if (name.startsWith("load_application_")) return reply(null);
    return reply(true);
  });
  const { host, button } = await mount();
  await act(async () => popup.options?.onJobChange("A job"));
  expect(button("Tailor").disabled).toBe(true);
  expect(button("Capture").disabled).toBe(false);
  await act(async () => button("Capture").click());
  expect(invoke).toHaveBeenCalledWith("request_application_capture", {
    target: "job",
  });
  activeContext = { ...context, browserConnected: true };
  await act(async () => window.dispatchEvent(new Event("focus")));
  const selector = host.querySelector<HTMLSelectElement>(
    '[aria-label="AI model preset"]',
  )!;
  expect(selector.options[0].text).toBe("Economy: Gemini small");
  await act(async () => {
    selector.value = "economy";
    selector.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(invoke).toHaveBeenCalledWith("set_ai_key_preset", {
    request: { credentialId: "key-one", preset: "economy" },
  });
});

it("autosaves without losing newer typing and only exports the latest prepared revision", async () => {
  vi.useFakeTimers();
  const firstSave = deferred<unknown>();
  const newExports = deferred<unknown>();
  let saves = 0;
  let latest = workspace();
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    const input = args as Record<string, unknown>;
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: latest });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return input.expectedRevision === 1 ? reply({}) : newExports.promise;
    if (name === "save_application_workspace") {
      latest = input.workspace as ReturnType<typeof workspace>;
      saves += 1;
      if (saves === 1) return firstSave.promise;
      return reply({ revision: 3, workspace: latest });
    }
    return reply(true);
  });
  const { host, button } = await mount();
  expect(button("Download").disabled).toBe(false);
  await act(async () => button("Edit").click());
  expect(popup.open).toHaveBeenCalledWith("resume-edit");
  await act(async () =>
    popup.options?.onResumeChange({ ...latest.resume, title: "First edit" }),
  );
  expect(button("Download").disabled).toBe(true);
  expect(button("Drag resume PDF to upload").disabled).toBe(true);
  await act(async () => vi.advanceTimersByTimeAsync(180));
  const firstDocument = latest;
  await act(async () =>
    popup.options?.onResumeChange({ ...latest.resume, title: "Latest edit" }),
  );
  await act(async () =>
    firstSave.resolve(reply({ revision: 2, workspace: firstDocument })),
  );
  expect(saves).toBe(2);
  expect(latest.resume.title).toBe("Latest edit");
  expect(popup.options?.resume?.title).toBe("Latest edit");
  expect(invoke).toHaveBeenCalledWith("save_application_workspace", {
    expectedRevision: 2,
    workspace: latest,
  });
  expect(button("Download").disabled).toBe(true);
  await act(async () => newExports.resolve(reply({})));
  expect(button("Download").disabled).toBe(false);
  const preparationCalls = vi
    .mocked(invoke)
    .mock.calls.filter(
      ([name]) => name === "prepare_application_exports",
    ).length;
  const format = host.querySelector<HTMLSelectElement>(
    '[aria-label="Download format"]',
  )!;
  await act(async () => {
    format.value = "docx";
    format.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(button("Download").disabled).toBe(false);
  expect(
    vi
      .mocked(invoke)
      .mock.calls.filter(([name]) => name === "prepare_application_exports"),
  ).toHaveLength(preparationCalls);
  await act(async () => button("Download").click());
  expect(invoke).toHaveBeenCalledWith("download_application_export", {
    expectedRevision: 3,
    kind: "resume",
    format: "docx",
  });
  await act(async () =>
    button("Drag resume DOCX to upload").dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0 }),
    ),
  );
  expect(invoke).toHaveBeenCalledWith("drag_application_export", {
    expectedRevision: 3,
    kind: "resume",
    format: "docx",
  });
  await act(async () =>
    listeners.get("ort:overlay-close-probe")?.({
      payload: { attempt: "clean" },
    }),
  );
  expect(emitTo).toHaveBeenCalledWith("main", "ort:overlay-close-reply", {
    attempt: "clean",
    dirty: false,
  });
});

it("shows working/cancel in the persistent header and retains dirty edits after save failure", async () => {
  vi.useFakeTimers();
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context")
      return reply({ ...context, aiBusy: true });
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: workspace() });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_workspace")
      return { ok: false, error: { code: "REVISION_CONFLICT" } };
    return reply(true);
  });
  const { host, button } = await mount();
  expect(host.querySelector("header")?.textContent).toContain("Working");
  await act(async () => button("Cancel").click());
  expect(invoke).toHaveBeenCalledWith("cancel_application_generation", {});
  await act(async () =>
    popup.options?.onResumeChange({
      ...workspace().resume,
      title: "Keep this edit",
    }),
  );
  await act(async () => vi.advanceTimersByTimeAsync(180));
  expect(host.textContent).toContain("Edits could not be saved");
  expect(popup.options?.resume?.title).toBe("Keep this edit");
  expect(button("Download").disabled).toBe(true);
  await act(async () =>
    listeners.get("ort:overlay-close-probe")?.({
      payload: { attempt: "dirty" },
    }),
  );
  expect(emitTo).toHaveBeenCalledWith("main", "ort:overlay-close-reply", {
    attempt: "dirty",
    dirty: true,
  });
});
