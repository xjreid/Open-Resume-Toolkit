// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { createResumeDocument, createSection } from "./resume-editor";
import { ApplicationPopup } from "./ApplicationPopup";
import {
  useApplicationPopup,
  type ApplicationPopupKind,
} from "./application-popup";

(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

const { listeners } = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));
vi.mock("pdfjs-dist/build/pdf.worker.min.mjs?url", () => ({
  default: "fixture-worker",
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve({ ok: true, value: true })),
}));
vi.mock("@tauri-apps/api/event", () => ({
  emitTo: vi.fn(() => Promise.resolve()),
}));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    listen: (name: string, callback: (event: { payload: unknown }) => void) => {
      listeners.set(name, callback);
      return Promise.resolve(() => listeners.delete(name));
    },
  }),
}));
vi.mock("./App", () => ({
  PublishedResume: () => <div>resume view</div>,
  ResumeCanvas: () => <div>resume edit</div>,
}));

afterEach(() => {
  listeners.clear();
  vi.clearAllMocks();
  document.body.replaceChildren();
});

it.each(["resume-view", "resume-edit", "cover-view", "cover"] as const)(
  "keeps %s open on outside clicks and closes with its X",
  async (kind) => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    await act(async () => root.render(<ApplicationPopup />));
    await act(async () =>
      listeners.get("ort:application-popup-snapshot")?.({
        payload: {
          session: "persistent-popup",
          generation: 1,
          revision: 1,
          acknowledgedEditSequence: 0,
          kind,
          job: "",
          jobUrl: "",
          resume: createResumeDocument(),
          style: "technical",
          coverLetter: "Current letter",
          disabled: false,
        },
      }),
    );
    await act(async () => {
      document.body.click();
      window.dispatchEvent(new Event("blur"));
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(host.querySelector("main")).toBeTruthy();
    expect(invoke).not.toHaveBeenCalledWith("hide_application_popup");
    await act(async () =>
      host
        .querySelector<HTMLButtonElement>('[aria-label="Close popup"]')!
        .click(),
    );
    expect(invoke).toHaveBeenCalledWith("hide_application_popup");
    expect(emitTo).toHaveBeenCalledWith(
      "overlay",
      "ort:application-popup-close",
      {
        session: "persistent-popup",
        sequence: 0,
        change: undefined,
      },
    );
    await act(async () => root.unmount());
  },
);

it("sends overlay section edits through the resume change channel", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const resume = {
    ...createResumeDocument(),
    sections: [{ ...createSection(0), heading: "Education" }],
  };
  await act(async () => root.render(<ApplicationPopup />));
  await act(async () =>
    listeners.get("ort:application-popup-snapshot")?.({
      payload: {
        session: "sections",
        generation: 1,
        revision: 1,
        acknowledgedEditSequence: 0,
        kind: "resume-edit",
        job: "",
        jobUrl: "",
        resume,
        style: "technical",
        coverLetter: null,
        disabled: false,
      },
    }),
  );
  expect(
    host.querySelector(
      ".application-popup__resume-layout .application-popup__navigator",
    ),
  ).not.toBeNull();
  await act(async () =>
    host
      .querySelector<HTMLButtonElement>(".section-add-control button")!
      .click(),
  );
  expect(emitTo).toHaveBeenCalledWith(
    "overlay",
    "ort:application-popup-change",
    {
      session: "sections",
      sequence: 1,
      change: {
        field: "resume",
        value: {
          ...resume,
          sections: [
            resume.sections[0],
            expect.objectContaining({ heading: "Custom Section", order: 1 }),
          ],
        },
      },
    },
  );
  await act(async () => root.unmount());
});

it("reports a job edit with the active snapshot session and sequence", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<ApplicationPopup />));
  await act(async () =>
    listeners.get("ort:application-popup-snapshot")?.({
      payload: {
        session: "session-a",
        generation: 1,
        revision: 4,
        acknowledgedEditSequence: 0,
        kind: "job",
        job: "Old",
        jobUrl: "",
        resume: null,
        style: "technical",
        coverLetter: null,
        disabled: false,
      },
    }),
  );
  const input = host.querySelector("textarea")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLTextAreaElement.prototype,
      "value",
    )?.set?.call(input, "New job");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(vi.mocked(emitTo)).toHaveBeenCalledWith(
    "overlay",
    "ort:application-popup-change",
    {
      session: "session-a",
      sequence: 1,
      change: { field: "job", value: "New job" },
    },
  );
  await act(async () =>
    listeners.get("ort:application-popup-snapshot")?.({
      payload: {
        session: "session-a",
        generation: 1,
        revision: 5,
        acknowledgedEditSequence: 0,
        kind: "job",
        job: "Old",
        jobUrl: "",
        resume: null,
        style: "technical",
        coverLetter: null,
        disabled: true,
      },
    }),
  );
  expect((host.querySelector("textarea") as HTMLTextAreaElement).value).toBe(
    "New job",
  );
  expect((host.querySelector("textarea") as HTMLTextAreaElement).disabled).toBe(
    true,
  );
  await act(async () => root.unmount());
});

function Bridge({ onJobChange }: { onJobChange: (value: string) => void }) {
  const popup = useApplicationPopup({
    job: "Job",
    jobUrl: "",
    resume: createResumeDocument(),
    style: "technical",
    coverLetter: null,
    disabled: false,
    onJobChange,
    onUrlChange: () => {},
    onResumeChange: () => {},
    onCoverChange: () => {},
    onError: () => {},
  });
  return (
    <>
      <button
        type="button"
        onClick={() => void popup.open("job" as ApplicationPopupKind)}
      >
        Open
      </button>
      <button type="button" onClick={() => void popup.close()}>
        Close
      </button>
    </>
  );
}

it("accepts only fresh edits from the active popup session", async () => {
  const onJobChange = vi.fn();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<Bridge onJobChange={onJobChange} />));
  await act(async () => host.querySelector("button")!.click());
  const snapshot = vi
    .mocked(emitTo)
    .mock.calls.find(
      ([target, event]) =>
        target === "application-popup" &&
        event === "ort:application-popup-snapshot",
    )?.[2] as { session: string };
  await act(async () =>
    listeners.get("ort:application-popup-change")?.({
      payload: {
        session: "old",
        sequence: 1,
        change: { field: "job", value: "Ignored" },
      },
    }),
  );
  await act(async () =>
    listeners.get("ort:application-popup-change")?.({
      payload: {
        session: snapshot.session,
        sequence: 2,
        change: { field: "job", value: "Accepted" },
      },
    }),
  );
  expect(onJobChange).toHaveBeenCalledTimes(1);
  expect(onJobChange).toHaveBeenCalledWith("Accepted");
  await act(async () => root.unmount());
});

it("reports a native command envelope failure", async () => {
  const onError = vi.fn();
  vi.mocked(invoke).mockResolvedValueOnce({
    ok: false,
    error: {
      code: "POPUP_UNAVAILABLE",
      messageKey: "errors.popupUnavailable",
      retryable: false,
      details: {},
    },
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<BridgeWithError onError={onError} />));
  await act(async () => host.querySelector("button")!.click());
  expect(onError).toHaveBeenCalledWith("POPUP_UNAVAILABLE");
  await act(async () => root.unmount());
});

function BridgeWithError({ onError }: { onError: (message: string) => void }) {
  const popup = useApplicationPopup({
    job: "Job",
    jobUrl: "",
    resume: createResumeDocument(),
    style: "technical",
    coverLetter: null,
    disabled: false,
    onJobChange: () => {},
    onUrlChange: () => {},
    onResumeChange: () => {},
    onCoverChange: () => {},
    onError,
  });
  return (
    <button type="button" onClick={() => void popup.open("job")}>
      Open
    </button>
  );
}

function CoverSwitchBridge({ onEdit }: { onEdit: (value: string) => void }) {
  const [text, setText] = useState("Original letter");
  const popup = useApplicationPopup({
    job: "",
    jobUrl: "",
    resume: null,
    style: "technical",
    coverLetter: text,
    disabled: false,
    onJobChange: () => {},
    onUrlChange: () => {},
    onResumeChange: () => {},
    onCoverChange: (value) => {
      setText(value);
      onEdit(value);
    },
    onError: (error) => {
      throw new Error(error);
    },
  });
  return (
    <>
      <button onClick={() => void popup.open("cover")}>Edit</button>
      <button onClick={() => void popup.open("cover-view")}>View</button>
    </>
  );
}

it("switches Edit to View in the same popup after flushing pending text", async () => {
  const onEdit = vi.fn();
  vi.mocked(emitTo).mockImplementation(async (_target, event, payload) => {
    if (event === "ort:application-popup-flush") {
      listeners.get("ort:application-popup-flushed")?.({
        payload: {
          ...(payload as object),
          sequence: 1,
          change: { field: "coverLetter", value: "Latest typed letter" },
        },
      });
    }
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<CoverSwitchBridge onEdit={onEdit} />));
  await act(async () =>
    host.querySelectorAll<HTMLButtonElement>("button")[0]!.click(),
  );
  await act(async () =>
    host.querySelectorAll<HTMLButtonElement>("button")[1]!.click(),
  );
  expect(onEdit).toHaveBeenCalledWith("Latest typed letter");
  expect(invoke).toHaveBeenLastCalledWith("show_application_popup", {
    kind: "cover-view",
  });
  const snapshots = vi
    .mocked(emitTo)
    .mock.calls.filter(
      ([, event]) => event === "ort:application-popup-snapshot",
    );
  expect(snapshots.at(-1)?.[2]).toMatchObject({
    kind: "cover-view",
    coverLetter: "Latest typed letter",
  });
  expect(snapshots.at(-1)?.[2]).toHaveProperty(
    "session",
    expect.not.stringMatching(
      (snapshots[0]![2] as { session: string }).session,
    ),
  );
  await act(async () => root.unmount());
});

it("closes with the final edit after pending typing emissions settle", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  let deliver!: () => void;
  const delivery = new Promise<void>((resolve) => {
    deliver = resolve;
  });
  vi.mocked(emitTo).mockImplementation(async (_target, event) => {
    if (event === "ort:application-popup-change") await delivery;
  });
  await act(async () => root.render(<ApplicationPopup />));
  await act(async () =>
    listeners.get("ort:application-popup-snapshot")?.({
      payload: {
        session: "closing",
        generation: 10,
        revision: 1,
        acknowledgedEditSequence: 0,
        kind: "job",
        job: "Old",
        jobUrl: "",
        resume: null,
        style: "technical",
        coverLetter: null,
        disabled: false,
      },
    }),
  );
  const input = host.querySelector("textarea")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLTextAreaElement.prototype,
      "value",
    )?.set?.call(input, "Final edit");
    input.dispatchEvent(new Event("input", { bubbles: true }));
    host.querySelector("button")!.click();
  });
  expect(invoke).not.toHaveBeenCalledWith("hide_application_popup");
  await act(async () => deliver());
  expect(emitTo).toHaveBeenCalledWith(
    "overlay",
    "ort:application-popup-close",
    {
      session: "closing",
      sequence: 1,
      change: { field: "job", value: "Final edit" },
    },
  );
  expect(invoke).toHaveBeenCalledWith("hide_application_popup");
  await act(async () => root.unmount());
});

it("serializes a close behind a delayed show and flushes the last edit before hiding", async () => {
  const onJobChange = vi.fn();
  let show!: () => void;
  vi.mocked(invoke).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        show = () => resolve({ ok: true, value: true });
      }),
  );
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<Bridge onJobChange={onJobChange} />));
  await act(async () => {
    host.querySelectorAll("button")[0].click();
    host.querySelectorAll("button")[1].click();
  });
  expect(invoke).not.toHaveBeenCalledWith("hide_application_popup", {});
  await act(async () => show());
  const request = vi
    .mocked(emitTo)
    .mock.calls.find(
      ([, event]) => event === "ort:application-popup-flush",
    )?.[2] as { session: string; requestId: string };
  expect(request).toBeDefined();
  await act(async () =>
    listeners.get("ort:application-popup-flushed")?.({
      payload: {
        ...request,
        sequence: 2,
        change: { field: "job", value: "Final, previously undelivered edit" },
      },
    }),
  );
  expect(onJobChange).toHaveBeenCalledWith(
    "Final, previously undelivered edit",
  );
  expect(invoke).toHaveBeenLastCalledWith("hide_application_popup");
  await act(async () => root.unmount());
});
