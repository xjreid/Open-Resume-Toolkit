// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { createResumeDocument } from "./resume-editor";
import { ApplicationOverlay } from "./ApplicationOverlay";
import type * as Wire from "@ort/contracts/wire";
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
const idleCapture = { phase: "idle", sessionId: null, error: null };
const context = {
  profileId: "01992187-74f7-7000-8000-000000000001",
  selectedKeyReady: true,
  publishedRevision: 1,
  aiLabel: "Gemini test",
  aiReady: true,
  aiBusy: false,
  selectedKeyId: "019a0000-0000-7000-8000-000000000006",
  model: "test",
  modelOptions: [{ model: "small" }, { model: "test" }],
  browserConnected: false,
};
function workspace(): Wire.ApplicationWorkspace {
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

const qualificationAlert = {
  jobStart: 0,
  jobEnd: 15,
  publishedRevision: 1,
  validationVersion: 1,
  mandatoryReason: "Required",
  target: "Python",
  id: "qualification-python",
  kind: "not_found" as const,
  category: "named_skill_or_technology",
  requirement: "Python",
  jobExcerpt: "Python required",
  resumeEvidence: null,
};

it.each([false, true])(
  "removes the empty qualification alert panel (truncated: %s)",
  async (alertsTruncated) => {
    vi.mocked(invoke).mockImplementation(async (name) => {
      if (name === "application_capture_status") return reply(idleCapture);
      if (name === "application_context") return reply(context);
      if (name === "load_application_workspace")
        return reply({
          revision: 1,
          workspace: { ...workspace(), alertsTruncated },
        });
      if (name.startsWith("load_application_")) return reply(null);
      if (name === "prepare_application_exports")
        return reply({
          revision: 1,
          pdfReady: true,
          docxReady: true,
          pageCount: 1,
        });
      throw new Error(`Unexpected command: ${name}`);
    });
    const { host } = await mount();
    expect(
      host.querySelector('[aria-label="Qualification alerts"]'),
    ).toBeNull();
    expect(host.textContent).not.toContain("Qualification gaps");
    expect(host.textContent).not.toContain(
      "No required qualification alerts to show",
    );
    expect(host.textContent).not.toContain("Ignore all");
  },
);

it("shows a static list even when saved alerts were previously hidden or dismissed", async () => {
  const current = {
    ...workspace(),
    alerts: [qualificationAlert],
    dismissedAlertIds: [qualificationAlert.id],
    ignoreAllAlerts: true,
  };
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host } = await mount();
  const panel = host.querySelector('[aria-label="Qualification alerts"]')!;
  expect(panel.querySelector(".application-alert")?.textContent).toBe(
    "Missing Python",
  );
  expect(panel.querySelector("button")).toBeNull();
  expect(panel.querySelector("blockquote")).toBeNull();
  expect(panel.textContent).not.toContain("Python required");
  expect(panel.textContent).not.toContain("Resume evidence");
  expect(invoke).not.toHaveBeenCalledWith(
    "save_application_workspace",
    expect.anything(),
  );
});

it.each(["startup", "browser event"])(
  "automatically replaces job fields from a %s capture without a dialog",
  async (delivery) => {
    vi.useFakeTimers();
    const capture = {
      revision: 1,
      capture: {
        kind: "capture.selection",
        protocolVersion: 2,
        sentAt: "2026-09-15T00:00:00Z",
        requestId: "019a0000-0000-7000-8000-000000000010",
        payload: {
          browser: "chrome",
          target: "job",
          text: "Captured description\n".repeat(60),
          url: "https" + "://example.test/new-job",
          title: "New job",
        },
      },
    };
    let pending: typeof capture | null =
      delivery === "startup" ? capture : null;
    let stageOne = {
      revision: 4,
      draft: {
        jobDescription: "Previous description",
        jobUrl: "https" + "://example.test/old-job",
        style: "technical",
      },
    };
    vi.mocked(invoke).mockImplementation(async (name, args) => {
      if (name === "application_context") return reply(context);
      if (name === "application_capture_status") return reply(idleCapture);
      if (name === "load_application_workspace") return reply(null);
      if (name === "load_application_stage_one") return reply(stageOne);
      if (name === "load_application_capture") return reply(pending);
      if (name === "apply_application_job_capture") {
        pending = null;
        stageOne = {
          revision: 5,
          draft: {
            ...stageOne.draft,
            jobDescription: capture.capture.payload.text,
            jobUrl: capture.capture.payload.url,
          },
        };
        return reply(stageOne);
      }
      if (name === "save_application_stage_one")
        return reply({
          revision: 6,
          draft: (args as Record<string, unknown>).draft,
        });
      throw new Error(`Unexpected command: ${name}`);
    });
    const { host } = await mount();
    if (delivery === "browser event") {
      pending = capture;
      await act(async () =>
        listeners.get("ort:browser-capture")?.({ payload: null }),
      );
    }
    const description = host.querySelector<HTMLTextAreaElement>(
      'textarea[aria-label="Job Description"]',
    )!;
    expect(description.value).toBe(capture.capture.payload.text);
    expect(
      host.querySelector<HTMLInputElement>('input[aria-label="Job URL"]')
        ?.value,
    ).toBe(capture.capture.payload.url);
    expect(host.querySelector('[role="dialog"]')).toBeNull();
    expect(invoke).toHaveBeenCalledWith("apply_application_job_capture", {
      requestId: "019a0000-0000-7000-8000-000000000010",
      expectedRevision: 4,
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "start_application",
      expect.anything(),
    );
    expect(dragWindow.setPosition).not.toHaveBeenCalled();
    await editField(description, "Edited captured description");
    await act(async () => vi.advanceTimersByTimeAsync(1500));
    expect(description.value).toBe("Edited captured description");
    expect(
      vi
        .mocked(invoke)
        .mock.calls.filter(
          ([name]) => name === "apply_application_job_capture",
        ),
    ).toHaveLength(1);
    expect(invoke).toHaveBeenCalledWith("save_application_stage_one", {
      expectedProfileId: context.profileId,
      expectedRevision: 5,
      draft: {
        ...stageOne.draft,
        jobDescription: "Edited captured description",
      },
    });
  },
);

it("waits for an in-flight job save before applying the capture at its new revision", async () => {
  vi.useFakeTimers();
  const saving = deferred<unknown>();
  const capture = {
    revision: 1,
    capture: {
      kind: "capture.selection",
      protocolVersion: 2,
      sentAt: "2026-09-15T00:00:00Z",
      requestId: "019a0000-0000-7000-8000-000000000011",
      payload: {
        browser: "chrome",
        target: "job",
        text: "New capture",
        url: "https" + "://example.test/new",
        title: "New job",
      },
    },
  };
  let pending: typeof capture | null = null;
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(context);
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "load_application_capture") return reply(pending);
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_stage_one") return saving.promise;
    if (name === "apply_application_job_capture") {
      pending = null;
      return reply({
        revision: 2,
        draft: {
          jobDescription: "New capture",
          jobUrl: "https" + "://example.test/new",
          style: "technical",
        },
      });
    }
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host } = await mount();
  const field = host.querySelector<HTMLTextAreaElement>(
    'textarea[aria-label="Job Description"]',
  )!;
  await editField(field, "Earlier manual edit");
  await act(async () => vi.advanceTimersByTimeAsync(300));
  pending = capture;
  await act(async () =>
    listeners.get("ort:browser-capture")?.({ payload: null }),
  );
  expect(invoke).not.toHaveBeenCalledWith(
    "apply_application_job_capture",
    expect.anything(),
  );
  await act(async () =>
    saving.resolve(
      reply({
        revision: 1,
        draft: {
          jobDescription: "Earlier manual edit",
          jobUrl: "",
          style: "technical",
        },
      }),
    ),
  );
  expect(invoke).toHaveBeenCalledWith("apply_application_job_capture", {
    requestId: "019a0000-0000-7000-8000-000000000011",
    expectedRevision: 1,
  });
  await act(async () => vi.advanceTimersByTimeAsync(1000));
  expect(field.value).toBe("New capture");
  expect(
    vi
      .mocked(invoke)
      .mock.calls.filter(([name]) => name === "save_application_stage_one"),
  ).toHaveLength(1);
});

it.each(["", "Original"])(
  "preserves reverting field edits before and during an autosave (original: %s)",
  async (original) => {
    vi.useFakeTimers();
    const saving = deferred<unknown>();
    let saves = 0;
    vi.mocked(invoke).mockImplementation(async (name, args) => {
      if (name === "application_context") return reply(context);
      if (name === "application_capture_status") return reply(idleCapture);
      if (name === "load_application_stage_one")
        return reply({
          revision: 1,
          draft: { jobDescription: original, jobUrl: "", style: "technical" },
        });
      if (name.startsWith("load_application_")) return reply(null);
      if (name === "save_application_stage_one") {
        if (++saves === 1) return saving.promise;
        return reply({
          revision: 3,
          draft: (args as Record<string, unknown>).draft,
        });
      }
      throw new Error(`Unexpected command: ${name}`);
    });
    const { host } = await mount();
    const field = host.querySelector<HTMLTextAreaElement>(
      'textarea[aria-label="Job Description"]',
    )!;
    await editField(field, "Transient edit");
    await editField(field, original);
    await act(async () => vi.advanceTimersByTimeAsync(300));
    expect(saves).toBe(0);
    expect(host.textContent).toContain("Your details are saved locally.");
    await editField(field, "Edit being saved");
    await act(async () => vi.advanceTimersByTimeAsync(300));
    await editField(field, original);
    await act(async () =>
      saving.resolve(
        reply({
          revision: 2,
          draft: {
            jobDescription: "Edit being saved",
            jobUrl: "",
            style: "technical",
          },
        }),
      ),
    );
    expect(invoke).toHaveBeenCalledWith("save_application_stage_one", {
      expectedProfileId: context.profileId,
      expectedRevision: 2,
      draft: { jobDescription: original, jobUrl: "", style: "technical" },
    });
    expect(field.value).toBe(original);
    expect(host.textContent).toContain("Your details are saved locally.");
  },
);

it("keeps the original job and captured payload on failure, with an inline retry", async () => {
  const capture = {
    revision: 1,
    capture: {
      kind: "capture.selection",
      protocolVersion: 2,
      sentAt: "2026-09-15T00:00:00Z",
      requestId: "019a0000-0000-7000-8000-000000000012",
      payload: {
        browser: "chrome",
        target: "job",
        text: "Replacement",
        url: "https" + "://example.test/job",
        title: "Job",
      },
    },
  };
  let pending: typeof capture | null = capture;
  let attempts = 0;
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context") return reply(context);
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "load_application_workspace") return reply(null);
    if (name === "load_application_capture") return reply(pending);
    if (name === "load_application_stage_one")
      return reply({
        revision: 1,
        draft: { jobDescription: "Original", jobUrl: "", style: "technical" },
      });
    if (name === "apply_application_job_capture") {
      if (++attempts === 1)
        return {
          ok: false,
          error: {
            code: "STORAGE_UNAVAILABLE",
            messageKey: "errors.storage",
            retryable: true,
            details: {},
          },
        };
      pending = null;
      return reply({
        revision: 2,
        draft: {
          jobDescription: "Replacement",
          jobUrl: "https" + "://example.test/job",
          style: "technical",
        },
      });
    }
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  const field = host.querySelector<HTMLTextAreaElement>(
    'textarea[aria-label="Job Description"]',
  )!;
  expect(field.value).toBe("Original");
  expect(host.querySelector('[role="dialog"]')).toBeNull();
  expect(button("Tailor").disabled).toBe(true);
  await act(async () =>
    listeners.get("ort:browser-capture")?.({ payload: null }),
  );
  expect(attempts).toBe(1);
  await act(async () => button("Retry capture").click());
  expect(field.value).toBe("Replacement");
  expect(pending).toBeNull();
  expect(host.textContent).not.toContain("Retry capture");
});

it.each([
  [
    {
      ...qualificationAlert,
      target: "C",
      requirement: "Candidates must demonstrate C programming skills",
    },
    "Missing C language",
  ],
  [
    {
      ...qualificationAlert,
      target: "Spanish",
      category: "language_proficiency",
    },
    "Missing Spanish proficiency",
  ],
  [
    {
      ...qualificationAlert,
      target: "10+ years experience",
      category: "experience_duration",
    },
    "Need 10+ years experience",
  ],
  [
    {
      ...qualificationAlert,
      target: "2027",
      category: "graduation_date",
      kind: "confirmed_mismatch",
      resumeEvidence: {
        fieldId: "01992187-74f7-7000-8000-000000000001",
        value: "2028",
      },
    },
    "Need graduation in 2027",
  ],
])("renders a brief qualification point %j", async (alert, expected) => {
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({
        revision: 1,
        workspace: { ...workspace(), alerts: [alert] },
      });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host } = await mount();
  expect(host.querySelector(".application-alert")?.textContent).toBe(expected);
});

it("displays alerts immediately from a successful tailoring response", async () => {
  const tailored = { ...workspace(), alerts: [qualificationAlert] };
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_stage_one")
      return reply({
        revision: 1,
        draft: (args as Record<string, unknown>).draft,
      });
    if (name === "start_application")
      return reply({ revision: 1, workspace: tailored });
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => popup.options?.onJobChange("Python required"));
  await act(async () => button("Tailor").click());
  expect(host.querySelector(".application-alert")?.textContent).toContain(
    "Python",
  );
});
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
const cleanups: (() => Promise<void>)[] = [];
async function editField(
  field: HTMLInputElement | HTMLTextAreaElement,
  value: string,
) {
  await act(async () => {
    const prototype =
      field instanceof HTMLTextAreaElement
        ? HTMLTextAreaElement.prototype
        : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(prototype, "value")!.set!.call(
      field,
      value,
    );
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
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
    if (name === "application_capture_status") return reply(idleCapture);
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

it("refreshes its selected model when the main app changes it", async () => {
  let current = context;
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(current);
    if (name.startsWith("load_application_")) return reply(null);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host } = await mount();
  current = { ...context, model: "small" };
  await act(async () =>
    listeners.get("ort:ai-model-changed")?.({ payload: null }),
  );
  expect(
    host.querySelector<HTMLSelectElement>('[aria-label="AI model"]')?.value,
  ).toBe("small");
});

it("keeps an unavailable selected model visible without implying a replacement", async () => {
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context")
      return reply({
        ...context,
        model: "previous-model",
        aiReady: false,
        selectedKeyReady: false,
      });
    if (name.startsWith("load_application_")) return reply(null);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host } = await mount();
  const selector = host.querySelector<HTMLSelectElement>(
    '[aria-label="AI model"]',
  )!;
  expect(selector.value).toBe("previous-model");
  expect(selector.selectedOptions[0].disabled).toBe(true);
  expect(
    [...selector.options].map((option) => option.textContent?.trim()),
  ).toEqual(["previous-model", "small", "test"]);
  expect(invoke).not.toHaveBeenCalledWith(
    "set_ai_key_model",
    expect.anything(),
  );
});

it("keeps Answers available without a question capture button", async () => {
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context")
      return reply({ ...context, browserConnected: true });
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: workspace() });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => button("Answers").click());
  expect(host.textContent).not.toContain("Capture question");
  expect(host.textContent).toContain("Question");
  expect(invoke).not.toHaveBeenCalledWith(
    "request_application_capture",
    expect.anything(),
  );
});

it("opens the cover PDF with View and the current cover text with Edit", async () => {
  vi.useFakeTimers();
  const current = { ...workspace(), coverLetter: "Updated cover letter" };
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    if (name === "save_application_workspace")
      return reply({
        revision: 2,
        workspace: (args as Record<string, unknown>).workspace,
      });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => button("Cover letter").click());
  expect(button("View").disabled).toBe(false);
  await act(async () => button("View").click());
  expect(popup.open).toHaveBeenCalledWith("cover-view");
  expect(popup.options?.workspaceRevision).toBe(1);
  await act(async () => button("Edit").click());
  expect(popup.open).toHaveBeenCalledWith("cover");
  expect(popup.options?.coverLetter).toBe("Updated cover letter");
  await act(async () =>
    popup.options?.onCoverChange("Freshly edited cover letter"),
  );
  expect(button("View").disabled).toBe(true);
  await act(async () => vi.advanceTimersByTimeAsync(180));
  await act(async () => button("View").click());
  expect(invoke).toHaveBeenCalledWith("prepare_application_exports", {
    expectedRevision: 2,
    kind: "cover_letter",
  });
  expect(popup.options?.workspaceRevision).toBe(2);
  expect(popup.options?.coverLetter).toBe("Freshly edited cover letter");
  expect(popup.open).toHaveBeenLastCalledWith("cover-view");
  expect(host.textContent).not.toContain("View and edit");
  expect(
    [...host.querySelectorAll("button")].some(
      (button) => button.textContent?.trim() === "Copy",
    ),
  ).toBe(false);
});

it("refines an answer and saves only its final version on reset", async () => {
  vi.useFakeTimers();
  let current = {
    ...workspace(),
    question: "Why this role?",
    answer: "First answer",
  };
  let revision = 1;
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    const input = args as Record<string, unknown>;
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
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
  expect(host.querySelector(".application-question-title")?.textContent).toBe(
    "Why this role?",
  );
  expect(
    host.querySelector('textarea[placeholder="Paste an application question"]'),
  ).toBeNull();
  const instructions = [...host.querySelectorAll("textarea")].find((item) =>
    item.parentElement?.textContent?.includes("Refinement instructions"),
  )!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLTextAreaElement.prototype,
      "value",
    )!.set!.call(instructions, "Make it concise");
    instructions.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => button("Refine answer").click());
  expect(invoke).toHaveBeenCalledWith("refine_application_answer", {
    expectedRevision: 1,
    instruction: "Make it concise",
  });
  await act(async () => button("Reset question").click());
  expect(
    host.querySelector('[role="dialog"][aria-label="Reset question"]'),
  ).toBeNull();
  await act(async () => vi.advanceTimersByTimeAsync(180));
  expect(current.approvedAnswers).toEqual([
    { question: "Why this role?", answer: "Final answer" },
  ]);
  expect(current.question).toBe("");
  expect(current.answer).toBe("");
  expect(button("Generate answer")).toBeTruthy();
  expect(host.querySelector(".application-question-title")).toBeNull();
  expect(
    host.querySelector('textarea[placeholder="Paste an application question"]'),
  ).toBeTruthy();
});

it("finishes directly and saves the final answer with tracker details", async () => {
  let current = {
    ...workspace(),
    question: "Why us?",
    answer: "Final response",
  };
  let revision = 1;
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    const input = args as Record<string, unknown>;
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
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
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    if (name === "finish_application") return reply(true);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  expect(
    host
      .querySelector(".application-finish-actions")
      ?.querySelectorAll("button"),
  ).toHaveLength(3);
  await act(async () => button("Edit tracker details").click());
  const dialog = host.querySelector(
    '[role="dialog"][aria-label="Edit tracker details"]',
  )!;
  expect(dialog.textContent).toContain("Date applied");
  expect(dialog.textContent).toContain("Link or source");
  expect(dialog.querySelectorAll('input[type="checkbox"]')).toHaveLength(0);
  const company = dialog.querySelector<HTMLInputElement>(
    'input[maxlength="200"]',
  )!;
  expect(company.value).toBe("Example");
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )!.set!.call(company, "Edited Company");
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

it("saves tracker details on Back, restores them after remount, and uses them on direct finish", async () => {
  const originalJobUrl = ["https:", "", "example.org", "original-job"].join(
    "/",
  );
  let current: Wire.ApplicationWorkspace = {
    ...workspace(),
    jobUrl: originalJobUrl,
  };
  let revision = 1;
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision, workspace: current });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({ revision, pdfReady: true, docxReady: true, pageCount: 1 });
    if (name === "save_application_workspace") {
      current = (args as { workspace: Wire.ApplicationWorkspace }).workspace;
      return reply({ revision: ++revision, workspace: current });
    }
    if (name === "finish_application") return reply(true);
    throw new Error(`Unexpected command: ${name}`);
  });
  const first = await mount();
  await act(async () => first.button("Edit tracker details").click());
  const field = (host: HTMLElement, name: string) =>
    [...host.querySelectorAll(".tracker-fields label")]
      .find((label) => label.firstChild?.textContent?.trim() === name)!
      .querySelector<HTMLInputElement>("input")!;
  const edits = {
    Company: "Edited Company",
    "Job title": "Senior Engineer",
    Location: "Boston",
    "Date applied": "2026-10-05",
    "Link or source": "Recruiter referral",
  };
  for (const [name, value] of Object.entries(edits))
    await editField(field(first.host, name), value);
  await act(async () => {
    const status = first.host.querySelector<HTMLSelectElement>(
      ".tracker-fields select",
    )!;
    status.value = "other";
    status.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await editField(field(first.host, "Custom status"), "Recruiter follow-up");
  await act(async () => first.button("Back").click());
  expect(first.host.querySelector('[role="dialog"]')).toBeNull();
  expect(current.trackerMetadata).toEqual({
    company: "Edited Company",
    title: "Senior Engineer",
    location: "Boston",
    dateApplied: "2026-10-05",
    status: "other",
    customStatus: "Recruiter follow-up",
    sourceUrl: "Recruiter referral",
  });
  expect(current.jobUrl).toBe(originalJobUrl);
  expect(current.jobDescription).toBe("Job");
  expect(current.roleInfo).toEqual({
    company: "Edited Company",
    title: "Senior Engineer",
    location: "Boston",
  });
  expect(invoke).not.toHaveBeenCalledWith(
    "finish_application",
    expect.anything(),
  );
  await cleanups.shift()!();
  const reopened = await mount();
  await act(async () => reopened.button("Edit tracker details").click());
  for (const [name, value] of Object.entries(edits))
    expect(field(reopened.host, name).value).toBe(value);
  expect(
    reopened.host.querySelector<HTMLSelectElement>(".tracker-fields select")!
      .value,
  ).toBe("other");
  expect(field(reopened.host, "Custom status").value).toBe(
    "Recruiter follow-up",
  );
  await act(async () => reopened.button("Back").click());
  await act(async () => reopened.button("Finish Application").click());
  expect(invoke).toHaveBeenCalledWith("finish_application", {
    expectedRevision: revision,
    selection: { entry: expect.objectContaining(current.trackerMetadata) },
  });
});

it("keeps tracker edits open when Back cannot save them", async () => {
  const saving = deferred<unknown>();
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: workspace() });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    if (name === "save_application_workspace") return saving.promise;
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => button("Edit tracker details").click());
  await editField(
    host.querySelector<HTMLInputElement>(".tracker-fields input")!,
    "Unsaved Company",
  );
  await act(async () => button("Back").click());
  expect(button("Back").disabled).toBe(true);
  expect(
    host.querySelector<HTMLInputElement>(".tracker-fields input")!.disabled,
  ).toBe(true);
  await act(async () =>
    saving.resolve({
      ok: false,
      error: {
        code: "SAVE_FAILED",
        messageKey: "errors.saveFailed",
        retryable: true,
        details: {},
      },
    }),
  );
  const dialog = host.querySelector(
    '[role="dialog"][aria-label="Edit tracker details"]',
  )!;
  expect(dialog).not.toBeNull();
  expect(dialog.querySelector("input")!.value).toBe("Unsaved Company");
  expect(dialog.querySelector('[role="alert"]')!.textContent).toContain(
    "SAVE_FAILED",
  );
  expect(button("Back").disabled).toBe(false);
});

it("confirms discarding the application without a tracker entry", async () => {
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: workspace() });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    if (name === "finish_application") return reply(true);
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => button("End application without saving").click());
  expect(host.querySelector('[role="dialog"]')).toBeTruthy();
  expect(invoke).not.toHaveBeenCalledWith(
    "finish_application",
    expect.anything(),
  );
  await act(async () => button("Back").click());
  expect(host.querySelector('[role="dialog"]')).toBeNull();
  await act(async () => button("End application without saving").click());
  await act(async () => button("End without saving").click());
  expect(invoke).toHaveBeenCalledWith("finish_application", {
    expectedRevision: 1,
    selection: null,
  });
});

it("edits the job fields directly and saves them before tailoring", async () => {
  const calls: string[] = [];
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    calls.push(name);
    const input = args as Record<string, unknown>;
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_stage_one")
      return reply({ revision: 1, draft: input.draft });
    if (name === "start_application")
      return reply({ revision: 1, workspace: workspace() });
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  const description = host.querySelector<HTMLTextAreaElement>(
    'textarea[aria-label="Job Description"]',
  )!;
  const url = host.querySelector<HTMLInputElement>(
    'input[aria-label="Job URL"]',
  )!;
  expect(description).toBeTruthy();
  expect(url.type).toBe("url");
  expect(host.textContent).not.toContain("Capture the opportunity");
  expect(host.textContent).not.toContain("Ready for your next role");
  expect(host.textContent).not.toContain("Resume style");
  expect(button("Capture").disabled).toBe(true);
  expect(button("Tailor").disabled).toBe(true);
  await editField(description, "Rust engineer required");
  await editField(url, "https" + "://example.test/job");
  expect(button("Tailor").disabled).toBe(false);
  expect(popup.open).not.toHaveBeenCalled();
  expect(host.textContent).not.toContain("Complete");
  await act(async () => button("Tailor").click());
  expect(calls.indexOf("start_application")).toBeGreaterThan(
    calls.indexOf("save_application_stage_one"),
  );
  expect(invoke).toHaveBeenCalledWith("start_application", {
    jobDescription: "Rust engineer required",
    jobUrl: "https" + "://example.test/job",
    style: "technical",
  });
  expect(host.textContent).toContain("Example");
  expect(host.textContent).not.toContain("02 / Your application");
  expect(
    host.querySelector('ul[aria-label="Tailoring notes"]')?.textContent,
  ).toContain("documented Rust");
});

it.each([
  ["AI_PAGE_FIT_FAILED", "could not fit a supported resume onto one page"],
  ["AI_REVIEW_FAILED", "unresolved issues after four calls"],
  ["AI_GROUNDING_FAILED", "valid references to the published evidence"],
  ["AI_OUTPUT_INVALID", "blocked, incomplete, or unreadable response"],
  ["AI_OUTPUT_INCOMPLETE", "did not finish generating"],
  ["AI_MODEL_MISMATCH", "different model than the one selected"],
  ["AI_MATERIAL_INVALID", "did not match the required material format"],
  ["AI_PROVIDER_BAD_REQUEST", "request parameters or API key (HTTP 400)"],
  ["AI_MODEL_UNAVAILABLE", "unavailable to this API key (HTTP 404)"],
])("shows %s without saving or preparing exports", async (code, expected) => {
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_stage_one")
      return reply({
        revision: 1,
        draft: (args as Record<string, unknown>).draft,
      });
    if (name === "start_application")
      return {
        ok: false,
        error: {
          code,
          messageKey: "errors.application",
          retryable: false,
          details: {},
        },
      };
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => popup.options?.onJobChange("Rust engineer required"));
  await act(async () => button("Tailor").click());
  expect(host.textContent).toContain(expected);
  expect(host.textContent).toContain("Nothing was saved");
  expect(host.textContent).not.toContain("shorten the job description");
  expect(button("Tailor").disabled).toBe(false);
  expect(invoke).not.toHaveBeenCalledWith(
    "prepare_application_exports",
    expect.anything(),
  );
});

it("retains detailed native failures in the tailoring popup without saving", async () => {
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_stage_one")
      return reply({
        revision: 1,
        draft: (args as Record<string, unknown>).draft,
      });
    if (name === "start_application")
      return {
        ok: false,
        error: {
          code: "AI_MATERIAL_INVALID",
          messageKey: "errors.applicationMaterial",
          retryable: false,
          details: {
            model: "gemini-3.5-flash-lite",
            provider: "gemini",
            callNumber: 4,
            maximumCalls: 4,
            durationMs: 15200,
            operationId: "op-safe",
            attemptId: "attempt-safe",
            diagnostic: {
              code: "AI_MATERIAL_INVALID",
              httpStatus: null,
              finishReason: null,
              providerReason: null,
              pageCount: null,
              validationIssues: [
                "Missing required schema v6 field: reviewIssues.",
              ],
            },
          },
        },
      };
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => popup.options?.onJobChange("Rust engineer required"));
  await act(async () => button("Tailor").click());
  expect(host.textContent).toContain("4 of 4 · correction");
  expect(host.textContent).toContain("15.2 seconds");
  expect(host.textContent).toContain("gemini-3.5-flash-lite");
  expect(host.textContent).toContain(
    "Missing required schema v6 field: reviewIssues.",
  );
  expect(host.querySelector(".ai-failure-details")?.hasAttribute("open")).toBe(
    true,
  );
  expect(button("Tailor").disabled).toBe(false);
});

it("places Finish Application directly below the header when no role was found", async () => {
  const draft = workspace();
  draft.roleInfo = { company: "", title: "", location: "" };
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: draft });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  expect(button("Finish Application")).toBeTruthy();
  expect(
    host.querySelector(".application-role")?.firstElementChild?.className,
  ).toBe("application-finish-actions");
  expect(host.textContent).not.toContain("Your next opportunity");
});

it("requires an active key even when a job exists and routes connected capture + models", async () => {
  let activeContext = {
    ...context,
    aiReady: false,
    selectedKeyId: null as string | null,
    browserConnected: true,
  };
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(activeContext);
    if (name === "request_application_capture")
      return reply({
        phase: "waiting",
        sessionId: "019a0000-0000-7000-8000-000000000013",
        error: null,
      });
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
    '[aria-label="AI model"]',
  )!;
  expect(selector.options[0].text).toBe("small");
  await act(async () => {
    selector.value = "small";
    selector.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(invoke).toHaveBeenCalledWith("set_ai_key_model", {
    request: {
      credentialId: "019a0000-0000-7000-8000-000000000006",
      model: "small",
    },
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
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: latest });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return input.expectedRevision === 1
        ? reply({ revision: 1, pdfReady: true, docxReady: true, pageCount: 1 })
        : newExports.promise;
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
    expectedProfileId: context.profileId,
    expectedRevision: 2,
    workspace: latest,
  });
  expect(button("Download").disabled).toBe(true);
  await act(async () =>
    newExports.resolve(
      reply({ revision: 1, pdfReady: true, docxReady: true, pageCount: 1 }),
    ),
  );
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
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context")
      return reply({ ...context, aiBusy: true });
    if (name === "load_application_workspace")
      return reply({ revision: 1, workspace: workspace() });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_workspace")
      return {
        ok: false,
        error: {
          code: "REVISION_CONFLICT",
          messageKey: "errors.conflict",
          retryable: false,
          details: {},
        },
      };
    return reply(true);
  });
  const { host, button } = await mount();
  expect(host.querySelector("header")?.textContent).toContain("Working");
  await act(async () => button("Stop").click());
  expect(invoke).toHaveBeenCalledWith("cancel_application_generation");
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

it("uses the overlay button to start, switch to Cancel after the first corner, and revoke that generation", async () => {
  let mode = { phase: "idle", sessionId: null as string | null, error: null };
  let delayNextStatus = false;
  let resolveStaleStatus: ((value: unknown) => void) | undefined;
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_context")
      return reply({ ...context, browserConnected: true });
    if (name === "application_capture_status") {
      if (delayNextStatus) {
        delayNextStatus = false;
        return new Promise((resolve) => {
          resolveStaleStatus = resolve;
        });
      }
      return reply(mode);
    }
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "request_application_capture") {
      mode = {
        phase: "waiting",
        sessionId: "019a0000-0000-7000-8000-000000000013",
        error: null,
      };
      return reply(mode);
    }
    if (name === "cancel_application_capture") {
      mode = idleCapture;
      return reply(mode);
    }
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await act(async () => button("Capture").click());
  expect(button("Capture").getAttribute("aria-pressed")).toBe("true");
  expect(host.textContent).toContain("top-left corner");
  mode = {
    phase: "selecting",
    sessionId: "019a0000-0000-7000-8000-000000000013",
    error: null,
  };
  await act(async () =>
    listeners.get("ort:capture-mode")?.({ payload: { phase: "forged" } }),
  );
  expect(button("Cancel").disabled).toBe(false);
  expect(host.textContent).toContain("bottom-right corner");
  const staleMode = mode;
  delayNextStatus = true;
  await act(async () => listeners.get("ort:capture-mode")?.({ payload: null }));
  await act(async () => button("Cancel").click());
  expect(invoke).toHaveBeenCalledWith("cancel_application_capture", {
    sessionId: "019a0000-0000-7000-8000-000000000013",
  });
  expect(button("Capture").getAttribute("aria-pressed")).toBe("false");
  await act(async () => resolveStaleStatus!(reply(staleMode)));
  expect(button("Capture").getAttribute("aria-pressed")).toBe("false");
});

it("shows measured page fit and locally computed selection changes", async () => {
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name === "load_application_workspace")
      return reply({
        revision: 1,
        workspace: {
          ...workspace(),
          changeSummary: [
            "Selection: removed 2, restored 0, reordered 1 sections/items.",
          ],
        },
      });
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "prepare_application_exports")
      return reply({
        revision: 1,
        pdfReady: true,
        docxReady: true,
        pageCount: 1,
      });
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host } = await mount();
  expect(host.textContent).toContain("One-page PDF verified");
  expect(
    host.querySelector('[aria-label="Resume changes"]')?.textContent,
  ).toContain("removed 2");
  await act(async () =>
    popup.options?.onResumeChange({
      ...workspace().resume,
      title: "Manual edit",
    }),
  );
  expect(host.textContent).not.toContain("One-page PDF verified");
});

it("announces quality phases and call count during tailoring", async () => {
  const pending = deferred<unknown>();
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "application_capture_status") return reply(idleCapture);
    if (name === "application_context") return reply(context);
    if (name.startsWith("load_application_")) return reply(null);
    if (name === "save_application_stage_one")
      return reply({
        revision: 1,
        draft: (args as Record<string, unknown>).draft,
      });
    if (name === "start_application") return pending.promise;
    throw new Error(`Unexpected command: ${name}`);
  });
  const { host, button } = await mount();
  await editField(
    host.querySelector<HTMLTextAreaElement>(
      'textarea[aria-label="Job Description"]',
    )!,
    "Rust engineer required",
  );
  await act(async () => button("Tailor").click());
  await act(async () =>
    listeners.get("ort:tailoring-progress")?.({
      payload: {
        phase: "Checking sources and editing",
        call: 2,
        maximum: 4,
        pageCount: 2,
      },
    }),
  );
  expect(host.textContent).toContain(
    "Checking sources and editing · call 2 of 4 · PDF: 2 pages; target 1",
  );
  await act(async () =>
    pending.resolve({
      ok: false,
      error: {
        code: "AI_CANCELLED",
        messageKey: "errors.application",
        retryable: false,
        details: {},
      },
    }),
  );
});
