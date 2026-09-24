// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { createResumeDocument } from "./resume-editor";
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
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve()),
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
    error: { code: "POPUP_UNAVAILABLE" },
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
      new Promise<void>((resolve) => {
        show = resolve;
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
  expect(invoke).toHaveBeenLastCalledWith("hide_application_popup", {});
  await act(async () => root.unmount());
});
