import axe from "axe-core";
import { JSDOM } from "jsdom";
import { act, useState } from "react";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../src/shared/App";
import { CloseDialog } from "../src/shared/CloseDialog";
import { createResumeDocument } from "../src/shared/resume-editor";

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ listen: native.listen }),
}));

const readyHealth = {
  ok: true,
  value: {
    status: "ok",
    appVersion: "0.0.0-test",
    profile: "development",
    storageStatus: "ready",
    contractVersion: 2,
  },
};

const noCloseAttempt = { ok: true, value: { pendingAttempt: null } };
const noRecovery = {
  ok: true,
  value: {
    safetyCopyAvailable: false,
    restartOperationPending: false,
    safetyCleanupPending: false,
  },
};
const unavailable = {
  ok: false,
  error: {
    code: "SYNTHETIC_UNAVAILABLE",
    messageKey: "errors.syntheticUnavailable",
    retryable: false,
    details: {},
  },
};

let dom: JSDOM;
let root: Root | null;
let saveResponse: unknown;

function installDom() {
  dom = new JSDOM(
    '<!doctype html><html lang="en"><body><div id="root"></div></body></html>',
    {
      url: "http://localhost/",
    },
  );
  vi.stubGlobal("window", dom.window);
  vi.stubGlobal("document", dom.window.document);
  vi.stubGlobal("navigator", dom.window.navigator);
  vi.stubGlobal("Node", dom.window.Node);
  vi.stubGlobal("Element", dom.window.Element);
  vi.stubGlobal("HTMLElement", dom.window.HTMLElement);
  vi.stubGlobal("HTMLInputElement", dom.window.HTMLInputElement);
  vi.stubGlobal("HTMLDialogElement", dom.window.HTMLDialogElement);
  vi.stubGlobal("Event", dom.window.Event);
  vi.stubGlobal("InputEvent", dom.window.InputEvent);
  vi.stubGlobal("MouseEvent", dom.window.MouseEvent);
  vi.stubGlobal(
    "getComputedStyle",
    dom.window.getComputedStyle.bind(dom.window),
  );
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  Object.defineProperties(dom.window.HTMLDialogElement.prototype, {
    showModal: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.open = true;
      },
    },
    close: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.open = false;
      },
    },
  });
}

async function settle() {
  await act(async () => {
    for (let turn = 0; turn < 5; turn += 1) {
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
    }
  });
}

async function render(element: React.ReactNode) {
  const container = document.getElementById("root");
  if (!container) throw new Error("Test root is missing");
  const { createRoot } = await import("react-dom/client");
  root = createRoot(container);
  await act(async () => root?.render(element));
  await settle();
  return container;
}

async function expectMediumSurfaceAccessible(container: Element) {
  const snapshot = new JSDOM(
    `<!doctype html><html lang="en"><head><title>Open Resume Toolkit</title></head><body>${container.innerHTML}</body></html>`,
  );
  for (const selector of [".backup-panel", ".storage-panel"]) {
    const elements = snapshot.window.document.querySelectorAll(selector);
    expect(elements, `one excluded HIGH surface for ${selector}`).toHaveLength(
      1,
    );
    elements[0]?.remove();
  }
  const results = await axe.run(snapshot.window.document.documentElement, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(
    results.violations.map((violation) => ({
      id: violation.id,
      targets: violation.nodes.map((node) => node.target),
    })),
  ).toEqual([]);
  snapshot.window.close();
}

function inputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    dom.window.HTMLInputElement.prototype,
    "value",
  )?.set;
  if (!setter) throw new Error("Input value setter is unavailable");
  setter.call(input, value);
  input.dispatchEvent(
    new dom.window.InputEvent("input", {
      bubbles: true,
      inputType: "insertText",
      data: value,
    }),
  );
}

beforeEach(() => {
  installDom();
  root = null;
  saveResponse = {
    ok: false,
    error: {
      code: "REVISION_CONFLICT",
      messageKey: "errors.revisionConflict",
      retryable: false,
      details: {},
    },
  };
  const document = createResumeDocument();
  document.contact.fullName = "Synthetic Person";
  native.invoke.mockImplementation(async (command: string) => {
    switch (command) {
      case "health":
        return readyHealth;
      case "load_resume":
        return {
          ok: true,
          value: {
            draft: { revision: 1, document },
            latestPublished: null,
          },
        };
      case "close_status":
        return noCloseAttempt;
      case "load_pdf_render_history":
        return { ok: true, value: { manifests: [] } };
      case "load_backup_recovery_status":
        return noRecovery;
      case "load_storage_usage":
        return unavailable;
      case "save_resume":
        return saveResponse;
      default:
        return unavailable;
    }
  });
});

afterEach(async () => {
  if (root) await act(async () => root?.unmount());
  dom.window.close();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("M2 live editor accessibility", () => {
  it("audits the loaded editor rather than only its loading shell", async () => {
    const container = await render(<App surface="main" />);
    expect(container.textContent).toContain("Identity and contact");
    const fullName = Array.from(container.querySelectorAll("input")).find(
      (input) => input.value === "Synthetic Person",
    );
    expect(fullName).toBeTruthy();
    await expectMediumSurfaceAccessible(container);
  });

  it("associates validation feedback with the invalid editor field", async () => {
    const container = await render(<App surface="main" />);
    const title = container.querySelector<HTMLInputElement>("input[required]");
    if (!title) throw new Error("Required resume title input is missing");
    await act(async () => inputValue(title, ""));

    expect(title.getAttribute("aria-invalid")).toBe("true");
    const description = title.getAttribute("aria-describedby");
    expect(description).toBeTruthy();
    expect(document.getElementById(description ?? "")?.textContent).toContain(
      "Enter a resume title",
    );
    expect(container.textContent).toContain(
      "Correct these items before saving",
    );
    await expectMediumSurfaceAccessible(container);
  });

  it("announces a revision conflict without hiding or disabling the editor", async () => {
    const container = await render(<App surface="main" />);
    const title = container.querySelector<HTMLInputElement>("input[required]");
    if (!title) throw new Error("Required resume title input is missing");
    await act(async () => inputValue(title, "Changed synthetic title"));

    const save = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent === "Save draft",
    );
    if (!save) throw new Error("Save draft button is missing");
    await act(async () => save.click());
    await settle();

    const alert = Array.from(container.querySelectorAll('[role="alert"]')).find(
      (candidate) =>
        candidate.textContent?.includes("changed after it was loaded"),
    );
    expect(alert?.textContent).toContain("changed after it was loaded");
    expect(title.disabled).toBe(false);
    await expectMediumSurfaceAccessible(container);
  });

  it("places focus in the quit dialog and restores it after cancellation", async () => {
    function FocusHarness() {
      const [open, setOpen] = useState(false);
      return (
        <main>
          <h1>Focus check</h1>
          <button type="button" onClick={() => setOpen(true)}>
            Review quit
          </button>
          <CloseDialog
            open={open}
            busy={false}
            resolving={false}
            canSave
            error={null}
            saveError={null}
            onCancel={() => setOpen(false)}
            onSave={() => {}}
            onDiscard={() => {}}
            onRetry={() => {}}
          />
        </main>
      );
    }

    const container = await render(<FocusHarness />);
    const opener = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent === "Review quit",
    );
    if (!opener) throw new Error("Quit-dialog opener is missing");
    opener.focus();
    await act(async () => opener.click());
    expect(document.activeElement?.textContent).toBe("Keep editing");

    const keep = document.activeElement as HTMLButtonElement;
    await act(async () => keep.click());
    expect(document.activeElement).toBe(opener);
  });
});
