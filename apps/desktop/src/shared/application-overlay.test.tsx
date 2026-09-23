// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { createResumeDocument } from "./resume-editor";
import { ApplicationOverlay } from "./ApplicationOverlay";

const { listeners } = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ emitTo: vi.fn() }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    listen: (name: string, handler: (event: { payload: unknown }) => void) => {
      listeners.set(name, handler);
      return Promise.resolve(() => listeners.delete(name));
    },
    setSize: () => Promise.resolve(),
  }),
}));
vi.mock("./AppShell", () => ({ Brand: () => <span>ORT</span> }));
vi.mock("./ApplicationViews", () => ({
  PdfCanvas: ({ base64 }: { base64: string }) => (
    <div data-testid="pdf-preview">{base64}</div>
  ),
  ResumeFields: ({
    document,
    onChange,
  }: {
    document: { title: string };
    onChange: (value: { title: string }) => void;
  }) => (
    <button
      type="button"
      onClick={() => onChange({ ...document, title: "Edited" })}
    >
      Edit resume title
    </button>
  ),
}));

afterEach(() => {
  vi.clearAllMocks();
  listeners.clear();
});

it("saves a typed Stage 1 job before starting tailoring", async () => {
  const calls: string[] = [];
  const resume = createResumeDocument();
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    const input = args as Record<string, unknown> | undefined;
    calls.push(name);
    if (name === "application_context")
      return {
        ok: true,
        value: { publishedRevision: 1, aiLabel: "Direct AI" },
      };
    if (
      name === "load_application_workspace" ||
      name === "load_application_stage_one" ||
      name === "load_application_capture"
    )
      return { ok: true, value: null };
    if (name === "save_application_stage_one")
      return { ok: true, value: { revision: 1, draft: input?.draft } };
    if (name === "start_application")
      return {
        ok: true,
        value: {
          revision: 1,
          workspace: {
            schemaVersion: 1,
            publishedRevision: 1,
            jobDescription: input?.jobDescription,
            jobUrl: "",
            resume,
            changePoints: [],
            alerts: [],
            alertsTruncated: false,
            dismissedAlertIds: [],
            ignoreAllAlerts: false,
            coverLetter: null,
            question: "",
            answer: "",
            approvedAnswers: [],
            style: "technical",
          },
        },
      };
    throw new Error(`Unexpected command: ${name}`);
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () => root.render(<ApplicationOverlay />));
    expect(calls).not.toContain("start_application");
    const job = host.querySelector("textarea");
    if (!job) throw new Error("Job input missing");
    const setValue = Object.getOwnPropertyDescriptor(
      window.HTMLTextAreaElement.prototype,
      "value",
    )?.set;
    if (!setValue) throw new Error("Textarea setter missing");
    await act(async () => {
      setValue.call(job, "Rust engineer required");
      job.dispatchEvent(new window.InputEvent("input", { bubbles: true }));
    });
    const continueButton = [...host.querySelectorAll("button")].find((item) =>
      item.textContent?.includes("Continue and tailor resume"),
    );
    expect(continueButton?.disabled).toBe(false);
    expect(calls).not.toContain("start_application");
    await act(async () => continueButton?.click());
    expect(calls.indexOf("save_application_stage_one")).toBeGreaterThan(-1);
    expect(calls.indexOf("start_application")).toBeGreaterThan(
      calls.indexOf("save_application_stage_one"),
    );
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("start_application", {
      jobDescription: "Rust engineer required",
      jobUrl: "",
      style: "technical",
    });
    expect(host.textContent).toContain("Tailored resume");
  } finally {
    await act(async () => root.unmount());
    host.remove();
  }
});

it("keeps the editor open through an edit and refreshes the PDF after save", async () => {
  const workspace = {
    schemaVersion: 1,
    publishedRevision: 1,
    jobDescription: "Job",
    jobUrl: "",
    resume: createResumeDocument(),
    changePoints: [],
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
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    const input = args as Record<string, unknown> | undefined;
    if (name === "application_context")
      return {
        ok: true,
        value: { publishedRevision: 1, aiLabel: "Direct AI" },
      };
    if (name === "load_application_workspace")
      return { ok: true, value: { revision: 1, workspace } };
    if (
      name === "load_application_stage_one" ||
      name === "load_application_capture"
    )
      return { ok: true, value: null };
    if (name === "preview_application_pdf")
      return {
        ok: true,
        value: {
          base64: input?.expectedRevision === 2 ? "new" : "old",
          filename: "resume.pdf",
        },
      };
    if (name === "save_application_workspace")
      return { ok: true, value: { revision: 2, workspace: input?.workspace } };
    throw new Error(`Unexpected command: ${name}`);
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const button = (label: string) => {
    const found = [...host.querySelectorAll("button")].find(
      (item) => item.textContent?.trim() === label,
    );
    if (!found) throw new Error(`Missing button: ${label}`);
    return found as HTMLButtonElement;
  };
  try {
    await act(async () => {
      root.render(<ApplicationOverlay />);
    });
    await act(async () => {
      button("Preview and edit").click();
    });
    expect(host.querySelector('[data-testid="pdf-preview"]')?.textContent).toBe(
      "old",
    );
    await act(async () => {
      button("Edit resume title").click();
    });
    expect(button("Edit resume title")).toBeTruthy();
    expect(host.textContent).toContain("This PDF shows the last saved version");
    await act(async () => {
      listeners.get("ort:overlay-close-probe")?.({
        payload: { attempt: "one" },
      });
    });
    expect(vi.mocked(emitTo)).toHaveBeenCalledWith(
      "main",
      "ort:overlay-close-reply",
      { attempt: "one", dirty: true },
    );
    expect(host.querySelector("main")?.hasAttribute("inert")).toBe(true);
    await act(async () => {
      listeners.get("ort:overlay-close-cancelled")?.({ payload: {} });
    });
    await act(async () => {
      button("Save edits").click();
    });
    expect(host.querySelector('[data-testid="pdf-preview"]')?.textContent).toBe(
      "new",
    );
    expect(button("Edit resume title")).toBeTruthy();
    expect(host.textContent).not.toContain(
      "This PDF shows the last saved version",
    );
    await act(async () => {
      listeners.get("ort:overlay-close-probe")?.({
        payload: { attempt: "two" },
      });
    });
    expect(vi.mocked(emitTo)).toHaveBeenCalledWith(
      "main",
      "ort:overlay-close-reply",
      { attempt: "two", dirty: false },
    );
    await act(async () => {
      listeners.get("ort:overlay-close-cancelled")?.({ payload: {} });
    });
    await act(async () => {
      button("Finish Application").click();
    });
    expect(host.textContent).toContain("Tailored resume: Edited");
    expect(host.textContent).toContain("Approved answers: 0");
    expect(host.textContent).toContain("generated drag files");
  } finally {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  }
});
