// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { createResumeDocument } from "./resume-editor";
import { ApplicationOverlay } from "./ApplicationOverlay";
import type { UseApplicationPopupOptions } from "./application-popup";

const { listeners, popup, dragWindow } = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  dragWindow: { setPosition: vi.fn(async (_position: unknown) => {}) },
  popup: {
    options: null as UseApplicationPopupOptions | null,
    open: vi.fn(),
    close: vi.fn(async () => {}),
  },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ emitTo: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({
  availableMonitors: async () => [
    {
      workArea: {
        position: { x: 0, y: 0 },
        size: { width: 1000, height: 800 },
      },
    },
  ],
}));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    listen: (name: string, handler: (event: { payload: unknown }) => void) => {
      listeners.set(name, handler);
      return Promise.resolve(() => listeners.delete(name));
    },
    startDragging: () => Promise.resolve(),
    outerPosition: async () => ({ x: 100, y: 100 }),
    outerSize: async () => ({ width: 360, height: 700 }),
    scaleFactor: async () => 1,
    setPosition: dragWindow.setPosition,
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

it("clamps header dragging before moving the native window", async () => {
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(context);
    if (name.startsWith("load_application_")) return reply(null);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host } = await mount();
  const header = host.querySelector<HTMLElement>(".application-header")!;
  header.setPointerCapture = vi.fn();
  header.hasPointerCapture = vi.fn(() => true);
  const pointer = (type: string, screenX: number, screenY: number) => {
    const event = new Event(type, { bubbles: true });
    Object.assign(event, {
      pointerId: 1,
      button: 0,
      screenX,
      screenY,
      clientX: 10,
      clientY: 10,
    });
    header.dispatchEvent(event);
  };
  await act(async () => pointer("pointerdown", 100, 100));
  await act(async () => pointer("pointermove", 2000, 2000));
  const position = dragWindow.setPosition.mock.lastCall?.[0];
  expect(position).toMatchObject({ x: 640, y: 100 });
});

it("refreshes its model preset when the main app changes it", async () => {
  let current = context;
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(current);
    if (name.startsWith("load_application_")) return reply(null);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host } = await mount();
  current = { ...context, preset: "economy" };
  await act(async () =>
    listeners.get("ort:ai-preset-changed")?.({ payload: null }),
  );
  expect(
    host.querySelector<HTMLSelectElement>('[aria-label="AI model preset"]')
      ?.value,
  ).toBe("economy");
});

it("uses one cover editor and copies the current cover letter", async () => {
  const current = { ...workspace(), coverLetter: "Updated cover letter" };
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports") return reply({});
    throw new Error(`Unexpected command: ${name}`);
  });
  const writeText = vi.fn(async () => {});
  const prior = Object.getOwnPropertyDescriptor(navigator, "clipboard");
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText },
  });
  try {
    const { host, button } = await mount();
    await act(async () => button("Cover letter").click());
    expect(host.textContent).not.toContain(
      "A letter grounded in your experience and this role.",
    );
    await act(async () => button("View and edit").click());
    expect(popup.open).toHaveBeenCalledWith("cover");
    await act(async () => button("Copy").click());
    expect(writeText).toHaveBeenCalledWith("Updated cover letter");
  } finally {
    if (prior) Object.defineProperty(navigator, "clipboard", prior);
    else Reflect.deleteProperty(navigator, "clipboard");
  }
});

it("refines an answer and saves only its final version on reset", async () => {
  vi.useFakeTimers();
  let current = { ...workspace(), question: "Why this role?", answer: "First answer" };
  let revision = 1;
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    const input = args as Record<string, unknown>;
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports") return reply({});
    if (name === "refine_application_answer") {
      current = { ...current, answer: "Final answer" };
      revision += 1;
      return reply({ revision, workspace: current });
    }
    if (name === "save_application_workspace") {
      current = input.workspace as typeof current;
      revision += 1;
      return reply({ revision, workspace: current });
    }
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => button("Answers").click());
  expect(host.textContent).not.toContain("Application answers");
  expect(host.textContent).not.toContain("Character limit");
  const instructions = [...host.querySelectorAll("textarea")].find(
    (item) => item.parentElement?.textContent?.includes("Refinement instructions"),
  )!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!
      .set!.call(instructions, "Make it concise");
    instructions.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => button("Refine answer").click());
  expect(invoke).toHaveBeenCalledWith("refine_application_answer", {
    expectedRevision: 1,
    instruction: "Make it concise",
  });
  await act(async () => button("Reset question").click());
  expect(host.querySelector('[role="dialog"][aria-label="Reset question"]')).toBeNull();
  await act(async () => vi.advanceTimersByTimeAsync(180));
  expect(current.approvedAnswers).toEqual([
    { question: "Why this role?", answer: "Final answer" },
  ]);
  expect(current.question).toBe("");
  expect(current.answer).toBe("");
  expect(button("Generate answer")).toBeTruthy();
});

it("finishes directly and saves the final answer with tracker details", async () => {
  let current = { ...workspace(), question: "Why us?", answer: "Final response" };
  let revision = 1;
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    const input = args as Record<string, unknown>;
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports") return reply({});
    if (name === "save_application_workspace") {
      current = input.workspace as typeof current;
      revision += 1;
      return reply({ revision, workspace: current });
    }
    if (name === "finish_application") return reply(true);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => button("Finish Application").click());
  expect(current.approvedAnswers).toEqual([
    { question: "Why us?", answer: "Final response" },
  ]);
  expect(host.querySelector('[role="dialog"]')).toBeNull();
  expect(invoke).toHaveBeenCalledWith("finish_application", {
    expectedRevision: 2,
    selection: {
      entry: expect.objectContaining({
        company: "Example",
        title: "Engineer",
        status: "applied",
        dateApplied: expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
      }),
    },
  });
});

it("edits tracker details before finishing and saves all materials", async () => {
  const current = {
    ...workspace(),
    coverLetter: "Final cover letter",
    approvedAnswers: [{ question: "Why?", answer: "Because." }],
  };
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports") return reply({});
    if (name === "finish_application") return reply(true);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  expect(
    host.querySelector(".application-finish-actions")?.querySelectorAll("button"),
  ).toHaveLength(3);
  await act(async () => button("Edit tracker details").click());
  const dialog = host.querySelector('[role="dialog"][aria-label="Edit tracker details"]')!;
  expect(dialog.textContent).toContain("Date applied");
  expect(dialog.textContent).toContain("Link or source");
  expect(dialog.querySelectorAll('input[type="checkbox"]')).toHaveLength(0);
  const company = dialog.querySelector<HTMLInputElement>('input[maxlength="200"]')!;
  expect(company.value).toBe("Example");
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!
      .set!.call(company, "Edited Company");
    company.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => button("Finish").click());
  expect(invoke).toHaveBeenCalledWith("finish_application", {
    expectedRevision: 1,
    selection: {
      entry: expect.objectContaining({ company: "Edited Company" }),
    },
  });
});

it("confirms discarding the application without a tracker entry", async () => {
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: workspace() });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports") return reply({});
    if (name === "finish_application") return reply(true);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => button("End application without saving").click());
  expect(host.querySelector('[role="dialog"]')).toBeTruthy();
  expect(invoke).not.toHaveBeenCalledWith("finish_application", expect.anything());
  await act(async () => button("Back").click());
  expect(host.querySelector('[role="dialog"]')).toBeNull();
  await act(async () => button("End application without saving").click());
  await act(async () => button("End without saving").click());
  expect(invoke).toHaveBeenCalledWith("finish_application", {
    expectedRevision: 1,
    selection: null,
  });
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
  expect(host.textContent).not.toContain("Capture the opportunity");
  expect(host.textContent).not.toContain("Ready for your next role");
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
  expect(host.textContent).not.toContain("02 / Your application");
  expect(
    host.querySelector('ul[aria-label="Tailoring notes"]')?.textContent,
  ).toContain("documented Rust");
});

it("places Finish Application directly below the header when no role was found", async () => {
  const draft = workspace();
  draft.roleInfo = { company: "", title: "", location: "" };
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: draft });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports") return reply({});
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  expect(button("Finish Application")).toBeTruthy();
  expect(
    host.querySelector(".application-role")?.firstElementChild?.className,
  ).toBe("application-finish-actions");
  expect(host.textContent).not.toContain("Your next opportunity");
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

it("shows working/stop in the persistent header and retains dirty edits after save failure", async () => {
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
  await act(async () => button("Stop").click());
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
