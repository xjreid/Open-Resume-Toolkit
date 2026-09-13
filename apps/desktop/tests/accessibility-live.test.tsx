import axe from "axe-core";
import { JSDOM } from "jsdom";
import { act, useState } from "react";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../src/shared/App";
import { BackupPanel } from "../src/shared/BackupPanel";
import { CloseDialog } from "../src/shared/CloseDialog";
import { StoragePanel } from "../src/shared/StoragePanel";
import {
  createResumeDocument,
  createEntityId,
  upgradeDocumentV2,
  createEntry,
  createSection,
} from "../src/shared/resume-editor";

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

async function expectSurfaceAccessible(container: Element) {
  const snapshot = new JSDOM(
    `<!doctype html><html lang="en"><head><title>Open Resume Toolkit</title></head><body>${container.innerHTML}</body></html>`,
  );
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

function inputValue(
  input: HTMLInputElement | HTMLTextAreaElement,
  value: string,
) {
  const setter = Object.getOwnPropertyDescriptor(
    input.tagName === "TEXTAREA"
      ? dom.window.HTMLTextAreaElement.prototype
      : dom.window.HTMLInputElement.prototype,
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

function inputInLabel(container: Element, text: string): HTMLInputElement {
  const label = Array.from(container.querySelectorAll("label")).find((value) =>
    value.textContent?.includes(text),
  );
  const input = label?.querySelector("input");
  if (!input) throw new Error(`Input labelled ${text} is missing`);
  return input;
}

function buttonNamed(container: Element, text: string): HTMLButtonElement {
  const button = Array.from(container.querySelectorAll("button")).find(
    (value) => value.textContent?.trim() === text,
  );
  if (!button) throw new Error(`Button ${text} is missing`);
  return button;
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
  it("allows backup validation and confirmed restore before creating a first draft", async () => {
    const original = native.invoke.getMockImplementation()!;
    let finishRestore!: (result: unknown) => void;
    native.invoke.mockImplementation(
      async (command: string, ...args: unknown[]) => {
        if (command === "load_resume")
          return { ok: true, value: { draft: null, latestPublished: null } };
        if (command === "validate_portable_backup")
          return { ok: true, value: { status: "cancelled" } };
        if (command === "restore_portable_backup")
          return new Promise((resolve) => {
            finishRestore = resolve;
          });
        return original(command, ...args);
      },
    );
    const container = await render(<App surface="main" />);
    const check = container.querySelector(
      '[aria-labelledby="backup-check-heading"]',
    )!;
    const restore = container.querySelector(
      '[aria-labelledby="backup-restore-heading"]',
    )!;
    const validationPassphrase = inputInLabel(
      check,
      "Existing backup passphrase",
    );
    const restorePassphrase = inputInLabel(restore, "Backup passphrase");
    const confirmation = inputInLabel(restore, "REPLACE SAVED PROFILE");
    expect(validationPassphrase.disabled).toBe(false);
    expect(restorePassphrase.disabled).toBe(false);
    await act(async () =>
      inputValue(validationPassphrase, "synthetic test passphrase"),
    );
    const validate = buttonNamed(check, "Select and check encrypted backup");
    expect(validate.disabled).toBe(false);
    await act(async () => validate.click());
    await settle();
    expect(container.textContent).toContain("Backup validation canceled");
    expect(validationPassphrase.value).toBe("");
    await act(async () =>
      inputValue(restorePassphrase, "synthetic test passphrase"),
    );
    const submit = buttonNamed(
      restore,
      "Select backup and replace after restart",
    );
    expect(submit.disabled).toBe(true);
    await act(async () => inputValue(confirmation, "REPLACE SAVED PROFILE"));
    expect(submit.disabled).toBe(false);
    await act(async () => submit.click());
    expect(restorePassphrase.value).toBe("");
    expect(confirmation.value).toBe("");
    expect(validationPassphrase.disabled).toBe(true);
    expect(restorePassphrase.disabled).toBe(true);
    await act(async () =>
      finishRestore({
        ok: true,
        value: {
          status: "staged",
          restartRequired: true,
          safetyCopyRetained: true,
        },
      }),
    );
    await settle();
    expect(container.textContent).toContain("Restart ORT to activate it");
    expect(container.textContent).toContain("Your master resume");
    expect(container.textContent).toContain("How would you like to start?");
    expect(
      container.querySelector('[aria-labelledby="identity-heading"]'),
    ).toBeNull();
    expect(container.textContent).toContain("How would you like to start?");
    expect(restorePassphrase.disabled).toBe(true);
    const commands = native.invoke.mock.calls.map(([command]) => command);
    expect(
      commands.filter((command) => command === "restore_portable_backup"),
    ).toHaveLength(1);
    expect(commands).not.toContain("save_resume");
    expect(commands).not.toContain("publish_resume");
  });

  it("preserves inline edits while navigating between workspaces", async () => {
    const container = await render(<App surface="main" />);
    const nameField = Array.from(
      container.querySelectorAll<HTMLButtonElement>(".canvas-field__button"),
    ).find((button) => button.textContent?.includes("Synthetic Person"));
    if (!nameField) throw new Error("Name canvas field is missing");
    await act(async () => nameField.click());
    const name = container.querySelector<HTMLInputElement>(
      'input[aria-label="Name"]',
    );
    if (!name) throw new Error("Editable name input is missing");
    await act(async () => inputValue(name, "Updated Person"));
    await act(async () => buttonNamed(container, "Settings").click());
    await act(async () => buttonNamed(container, "Master resume").click());
    expect(
      container.querySelector<HTMLInputElement>('input[aria-label="Name"]')
        ?.value,
    ).toBe("Updated Person");
    await expectSurfaceAccessible(container);
  });

  it("creates an empty draft through the current guided start flow", async () => {
    const original = native.invoke.getMockImplementation()!;
    native.invoke.mockImplementation(
      async (command: string, ...args: unknown[]) =>
        command === "load_resume"
          ? { ok: true, value: { draft: null, latestPublished: null } }
          : original(command, ...args),
    );
    const container = await render(<App surface="main" />);
    expect(container.textContent).toContain("How would you like to start?");
    await expectSurfaceAccessible(container);
    const buildButtons = Array.from(
      container.querySelectorAll("button"),
    ).filter((button) => button.textContent?.trim() === "Build from scratch");
    await act(async () => buildButtons.at(-1)?.click());
    expect(
      container.querySelector('[aria-label="Editable resume"]'),
    ).not.toBeNull();
    expect(container.textContent).toContain("Contact");
  });

  it("switches between edit and view modes and exposes the selected export style", async () => {
    const container = await render(<App surface="main" />);
    const style = Array.from(container.querySelectorAll("label"))
      .find((label) => label.textContent?.includes("Resume style"))
      ?.querySelector("select");
    if (!style) throw new Error("Resume style selector is missing");
    await act(async () => {
      style.value = "modern";
      style.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
    });
    expect(style.value).toBe("modern");
    await act(async () => buttonNamed(container, "View").click());
    expect(
      container.querySelector('[aria-label="Published resume content"]'),
    ).not.toBeNull();
    expect(buttonNamed(container, "Export PDF")).not.toBeNull();
    await act(async () => buttonNamed(container, "Edit").click());
    expect(
      container.querySelector('[aria-label="Editable resume"]'),
    ).not.toBeNull();
    await expectSurfaceAccessible(container);
  });

  it("keeps import and settings workspaces navigable and accessible", async () => {
    const container = await render(<App surface="main" />);
    await act(async () => buttonNamed(container, "Import resume").click());
    expect(container.querySelector<HTMLElement>(".import-page")?.hidden).toBe(
      false,
    );
    await expectSurfaceAccessible(container);
    await act(async () => buttonNamed(container, "Settings").click());
    expect(container.querySelector<HTMLElement>(".settings-page")?.hidden).toBe(
      false,
    );
    await expectSurfaceAccessible(container);
  });

  it.each([false, true])(
    "blocks backup replacement while the current canvas is dirty (saved draft: %s)",
    async (hasSavedDraft) => {
      if (!hasSavedDraft) {
        const original = native.invoke.getMockImplementation()!;
        native.invoke.mockImplementation(
          async (command: string, ...args: unknown[]) =>
            command === "load_resume"
              ? { ok: true, value: { draft: null, latestPublished: null } }
              : original(command, ...args),
        );
      }
      const container = await render(<App surface="main" />);
      if (!hasSavedDraft) {
        const buildButtons = Array.from(
          container.querySelectorAll("button"),
        ).filter(
          (button) => button.textContent?.trim() === "Build from scratch",
        );
        await act(async () => buildButtons.at(-1)?.click());
      }
      const nameField = Array.from(
        container.querySelectorAll<HTMLButtonElement>(".canvas-field__button"),
      ).find((button) => button.textContent?.includes("Name"));
      if (!nameField) throw new Error("Name canvas field is missing");
      await act(async () => nameField.click());
      const name = container.querySelector<HTMLInputElement>(
        'input[aria-label="Name"]',
      );
      if (!name) throw new Error("Editable name input is missing");
      await act(async () =>
        inputValue(name, hasSavedDraft ? "Dirty Person" : "New Person"),
      );
      await act(async () => buttonNamed(container, "Settings").click());
      expect(
        inputInLabel(container, "Existing backup passphrase").disabled,
      ).toBe(true);
      const restore = container.querySelector(
        '[aria-labelledby="backup-restore-heading"]',
      )!;
      expect(inputInLabel(restore, "Backup passphrase").disabled).toBe(true);
      expect(
        buttonNamed(restore, "Select backup and replace after restart")
          .disabled,
      ).toBe(true);
    },
  );

  it("announces an autosave revision conflict without hiding the canvas", async () => {
    const container = await render(<App surface="main" />);
    const nameField = Array.from(
      container.querySelectorAll<HTMLButtonElement>(".canvas-field__button"),
    ).find((button) => button.textContent?.includes("Synthetic Person"));
    if (!nameField) throw new Error("Name canvas field is missing");
    await act(async () => nameField.click());
    const name = container.querySelector<HTMLInputElement>(
      'input[aria-label="Name"]',
    );
    if (!name) throw new Error("Editable name input is missing");
    await act(async () => inputValue(name, "Conflict Person"));
    await act(async () => {
      await new Promise<void>((resolve) => setTimeout(resolve, 1250));
    });
    await settle();
    const alert = Array.from(container.querySelectorAll('[role="alert"]')).find(
      (candidate) =>
        candidate.textContent?.includes("changed after it was loaded"),
    );
    expect(alert?.textContent).toContain("changed after it was loaded");
    expect(
      container.querySelector('[aria-label="Editable resume"]'),
    ).not.toBeNull();
    await expectSurfaceAccessible(container);
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

  it("labels, announces, and focuses the destructive local-data lifecycle", async () => {
    let finishDeletion: ((value: unknown) => void) | undefined;
    native.invoke.mockImplementation(async (command: string) => {
      if (command === "load_storage_usage") {
        return {
          ok: true,
          value: {
            databaseSchema: 2,
            drafts: 1,
            publishedSnapshots: 1,
            settings: 0,
            renderManifests: 1,
            diagnosticEvents: 0,
            databaseBytes: 100,
            walBytes: 0,
            sharedMemoryBytes: 0,
            manifestBytes: 10,
            recoveryMetadataBytes: 0,
            totalProfileBytes: 110,
          },
        };
      }
      if (command === "delete_all_local_data") {
        return new Promise((resolve) => {
          finishDeletion = resolve;
        });
      }
      return unavailable;
    });
    const onDeleteFinish = vi.fn();
    const container = await render(
      <StoragePanel
        enabled
        onDeleteBegin={() => true}
        onDeleteFinish={onDeleteFinish}
      />,
    );
    const confirmation = inputInLabel(container, "DELETE ALL LOCAL ORT DATA");
    await act(async () => inputValue(confirmation, "DELETE ALL"));
    expect(confirmation.getAttribute("aria-invalid")).toBe("true");
    expect(
      document.getElementById("delete-all-local-data-confirmation")
        ?.textContent,
    ).toContain("exactly as shown");

    await act(async () =>
      inputValue(confirmation, "DELETE ALL LOCAL ORT DATA"),
    );
    expect(confirmation.hasAttribute("aria-invalid")).toBe(false);
    const remove = buttonNamed(container, "Permanently delete all local data");
    expect(remove.disabled).toBe(false);
    await act(async () => remove.click());
    expect(container.querySelector("form[aria-busy='true']")).toBeTruthy();
    expect(container.textContent).toContain(
      "Closing the encrypted profile and deleting exact local records",
    );

    await act(async () => {
      finishDeletion?.({
        ok: true,
        value: { status: "cleanup_pending", restartRequired: true },
      });
      await Promise.resolve();
    });
    await settle();
    const outcome = Array.from(
      container.querySelectorAll('[role="status"]'),
    ).find((value) => value.textContent?.includes("Deletion was committed"));
    expect(document.activeElement).toBe(outcome);
    expect(onDeleteFinish).toHaveBeenCalledWith(true, false);
    await expectSurfaceAccessible(container);
  });

  it("exposes exact recovery confirmations and an announced busy state", async () => {
    let finishSafetyDeletion: ((value: unknown) => void) | undefined;
    native.invoke.mockImplementation(async (command: string) => {
      if (command === "load_backup_recovery_status") {
        return {
          ok: true,
          value: {
            safetyCopyAvailable: true,
            restartOperationPending: false,
            safetyCleanupPending: false,
          },
        };
      }
      if (command === "delete_safety_copy") {
        return new Promise((resolve) => {
          finishSafetyDeletion = resolve;
        });
      }
      return unavailable;
    });
    const onFinish = vi.fn();
    const container = await render(
      <BackupPanel
        blocked={false}
        dirty={false}
        onBegin={() => true}
        onFinish={onFinish}
      />,
    );
    const rollbackConfirmation = inputInLabel(
      container,
      "ROLL BACK SAVED PROFILE",
    );
    const restoreConfirmation = inputInLabel(
      container,
      "REPLACE SAVED PROFILE",
    );
    await act(async () => {
      inputValue(rollbackConfirmation, "ROLL BACK");
      inputValue(restoreConfirmation, "REPLACE");
    });
    expect(rollbackConfirmation.getAttribute("aria-invalid")).toBe("true");
    expect(restoreConfirmation.getAttribute("aria-invalid")).toBe("true");
    expect(
      document.getElementById("rollback-confirmation")?.textContent,
    ).toContain("exactly as shown");
    expect(
      document.getElementById("restore-confirmation")?.textContent,
    ).toContain("exactly as shown");

    const confirmation = inputInLabel(container, "DELETE SAFETY COPY");
    await act(async () => inputValue(confirmation, "DELETE"));
    expect(confirmation.getAttribute("aria-invalid")).toBe("true");
    expect(
      document.getElementById("safety-delete-confirmation")?.textContent,
    ).toContain("exactly as shown");

    await act(async () => inputValue(confirmation, "DELETE SAFETY COPY"));
    const remove = buttonNamed(container, "Permanently delete safety copy");
    expect(remove.disabled).toBe(false);
    await act(async () => remove.click());
    expect(container.querySelector("form[aria-busy='true']")).toBeTruthy();
    expect(container.textContent).toContain("Deleting safety copy");

    await act(async () => {
      finishSafetyDeletion?.({ ok: true, value: { deleted: true } });
      await Promise.resolve();
    });
    await settle();
    expect(onFinish).toHaveBeenCalledWith(
      expect.stringContaining("external exports or backups"),
    );
    expect(remove.disabled).toBe(true);
    await expectSurfaceAccessible(container);
  });
});

it("quits an idle review even after cancellation fails, but waits for actual import work", async () => {
  const original = native.invoke.getMockImplementation()!;
  let closeAttempt: string | null = null;
  let wake!: () => void;
  let finish!: (value: unknown) => void;
  const snapshot = {
    id: createEntityId(),
    baseRevision: 1,
    mappingVersion: 1,
    blocks: [
      {
        source: "Projects",
        page: 1,
        explanation: "Heading",
        suggestedTarget: "section",
        suggestedValue: "Projects",
        proposedSection: null,
      },
    ],
    sections: [],
    contacts: { fullName: "", email: "", phone: "", location: "" },
  };
  native.listen.mockImplementation(async (_event, callback) => {
    wake = callback;
    return () => {};
  });
  native.invoke.mockImplementation(async (command, ...args) => {
    if (command === "document_import_available") return true;
    if (command === "begin_document_import")
      return new Promise((resolve) => {
        finish = resolve;
      });
    if (command === "read_import_review") return { ok: true, value: snapshot };
    if (command === "cancel_import_review") return unavailable;
    if (command === "close_status")
      return { ok: true, value: { pendingAttempt: closeAttempt } };
    if (command === "resolve_close") {
      closeAttempt = null;
      return noCloseAttempt;
    }
    return original(command, ...args);
  });
  const container = await render(<App surface="main" />);
  await act(async () =>
    buttonNamed(container, "Import an existing resume").click(),
  );
  closeAttempt = createEntityId();
  await act(async () => wake());
  expect(
    native.invoke.mock.calls.filter(([command]) => command === "resolve_close"),
  ).toHaveLength(0);
  expect(
    buttonNamed(container, "Discard unsaved edits and quit").disabled,
  ).toBe(true);
  await act(async () => buttonNamed(container, "Keep editing").click());
  await act(async () => finish({ ok: true, value: snapshot }));
  await settle();
  await act(async () => buttonNamed(container, "Cancel import").click());
  expect(container.textContent).toContain(
    "Cancellation could not be confirmed",
  );
  closeAttempt = createEntityId();
  await act(async () => wake());
  await settle();
  expect(
    native.invoke.mock.calls
      .filter(([command]) => command === "resolve_close")
      .at(-1)?.[1],
  ).toMatchObject({ request: { payload: { decision: "quit" } } });
});
