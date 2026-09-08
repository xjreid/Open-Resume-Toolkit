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
  it("starts one manual draft with suggested sections and focuses contact information", async () => {
    const original = native.invoke.getMockImplementation()!;
    native.invoke.mockImplementation(
      async (command: string, ...args: unknown[]) =>
        command === "load_resume"
          ? { ok: true, value: { draft: null, latestPublished: null } }
          : original(command, ...args),
    );
    const container = await render(<App surface="main" />);
    expect(container.textContent).toContain("How would you like to start?");
    expect(buttonNamed(container, "Import an existing resume").disabled).toBe(
      true,
    );
    expect(buttonNamed(container, "Save draft").disabled).toBe(true);
    expect(native.invoke.mock.calls.map(([command]) => command)).not.toContain(
      "save_resume",
    );
    const choice =
      container.querySelector<HTMLSelectElement>("#starting-profile")!;
    await act(async () => {
      choice.value = "technical";
      choice.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
    });
    expect(container.textContent).toContain(
      "Suggested sections: Experience, Projects, Skills, Education.",
    );
    await act(async () => buttonNamed(container, "Build from scratch").click());
    expect(container.querySelectorAll(".resume-start").length).toBe(0);
    expect(dom.window.document.activeElement).toBe(
      inputInLabel(container, "Full name"),
    );
    const headings = Array.from(
      container.querySelectorAll("#resume-navigation button"),
    )
      .map((button) => button.textContent?.replace(/^Edit /, ""))
      .filter((heading) =>
        ["Experience", "Projects", "Skills", "Education"].includes(
          heading ?? "",
        ),
      );
    expect(headings).toEqual(["Experience", "Projects", "Skills", "Education"]);
    expect(inputInLabel(container, "Full name").value).toBe("");
    expect(native.invoke.mock.calls.map(([command]) => command)).not.toContain(
      "publish_resume",
    );
    await expectSurfaceAccessible(container);
  });

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
    expect(container.textContent).toContain("Not saved");
    expect(container.textContent).toContain("No snapshot");
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

  it.each([false, true])(
    "keeps backup controls blocked after an edit (saved draft: %s)",
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
        await act(async () =>
          buttonNamed(container, "Build from scratch").click(),
        );
      }
      // An invalid edit cannot autosave and must remain protected from restore.
      await act(async () =>
        inputValue(inputInLabel(container, "Resume title"), ""),
      );
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
      await act(async () =>
        restore
          .querySelector("form")!
          .dispatchEvent(
            new dom.window.Event("submit", { bubbles: true, cancelable: true }),
          ),
      );
      expect(
        native.invoke.mock.calls.map(([command]) => command),
      ).not.toContain("restore_portable_backup");
      if (!hasSavedDraft) {
        await act(async () => buttonNamed(container, "Undo edit").click());
        expect(
          inputInLabel(container, "Existing backup passphrase").disabled,
        ).toBe(true);
      }
    },
  );

  it("audits the loaded editor rather than only its loading shell", async () => {
    const container = await render(<App surface="main" />);
    expect(container.textContent).toContain("Identity and contact");
    const fullName = Array.from(container.querySelectorAll("input")).find(
      (input) => input.value === "Synthetic Person",
    );
    expect(fullName).toBeTruthy();
    await expectSurfaceAccessible(container);
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
    await expectSurfaceAccessible(container);
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

it("selects export styles without saving content and labels an existing preview truthfully", async () => {
  const original = native.invoke.getMockImplementation();
  const pdfReceipt = {
    documentSha256: "a".repeat(64),
    documentSchemaVersion: 1,
    pdfSha256: "b".repeat(64),
    rendererVersion: "typst-0.15.1/ort-1",
    templateId: "technical_pdf_v1",
    templateSha256: "c".repeat(64),
    fontBundleId: "libertinus-serif/typst-assets-0.15.1",
    fontBundleSha256: "d".repeat(64),
    pageCount: 1,
    byteCount: 5,
  };
  native.invoke.mockImplementation(async (command: string, args: unknown) => {
    if (command === "render_resume_pdf")
      return {
        ok: true,
        value: {
          renderId: "019a0000-0000-7000-8000-000000000001",
          source: "saved_draft",
          revision: 1,
          generatedAtUnixMs: Date.now(),
          receipt: pdfReceipt,
          // Deliberately fails the byte hash check; this test does not emulate PDF.js.
          pdfBase64: "JVBERi0=",
        },
      };
    if (command === "export_resume_docx")
      return {
        ok: true,
        value: {
          status: "exported",
          source: "saved_draft",
          revision: 1,
          byteCount: 123,
          formatVersion: 1,
          cleanupPending: false,
          durabilityUnconfirmed: false,
          templateId: "modern_docx_v1",
        },
      };
    return original?.(command, args);
  });
  const container = await render(<App surface="main" />);
  const selectInLabel = (label: string) => {
    const element = Array.from(container.querySelectorAll("label"))
      .find((element) => element.textContent?.includes(label))
      ?.querySelector("select");
    if (!element) throw new Error(`Missing ${label}`);
    return element;
  };
  const style = selectInLabel("Document style");
  const view = selectInLabel("Resume view");
  const pdfPanel = container.querySelector(".pdf-panel")!;
  expect(pdfPanel.closest(".resume-reading-panel")).not.toBeNull();
  expect(pdfPanel.parentElement?.hidden).toBe(true);
  await act(async () => {
    view.value = "pdf";
    view.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  expect(pdfPanel.parentElement?.hidden).toBe(false);
  expect(style.value).toBe("technical");
  expect(Array.from(style.options).map((option) => option.value)).toEqual([
    "technical",
    "professional",
    "modern",
  ]);
  await act(async () => buttonNamed(container, "Preview saved draft").click());
  await settle();
  expect(native.invoke).toHaveBeenCalledWith("render_resume_pdf", {
    request: expect.objectContaining({
      payload: {
        source: "saved_draft",
        expectedRevision: 1,
        style: "technical",
      },
    }),
  });
  const renderCount = native.invoke.mock.calls.filter(
    ([command]) => command === "render_resume_pdf",
  ).length;
  await act(async () => {
    view.value = "reading";
    view.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  await act(async () => {
    view.value = "pdf";
    view.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  expect(container.querySelector(".pdf-panel")).toBe(pdfPanel);
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === "render_resume_pdf",
    ),
  ).toHaveLength(renderCount);
  for (const value of ["professional", "modern"]) {
    await act(async () => {
      style.value = value;
      style.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
    });
  }
  expect(container.textContent).toContain(
    "Preview style: Technical / Engineering.",
  );
  expect(container.textContent).toContain(
    "The selected style is Modern / Marketing & Sales.",
  );
  expect(container.textContent).toContain(
    "This existing preview keeps its original style",
  );
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === "render_resume_pdf",
    ),
  ).toHaveLength(renderCount);
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === "save_resume" || command === "publish_resume",
    ),
  ).toBe(false);
  expect(inputInLabel(container, "Full name").value).toBe("Synthetic Person");
  const format = selectInLabel("Export format");
  await act(async () => {
    format.value = "docx";
    format.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  await act(async () =>
    buttonNamed(container, "Export saved draft (.docx)").click(),
  );
  await settle();
  expect(native.invoke).toHaveBeenCalledWith("export_resume_docx", {
    request: expect.objectContaining({
      payload: { source: "saved_draft", expectedRevision: 1, style: "modern" },
    }),
  });
  expect(container.textContent).toContain(
    "DOCX (Modern / Marketing & Sales v1)",
  );
  await expectSurfaceAccessible(container);
});

it("explicitly upgrades a draft, preserves legacy dates, and edits identified dates and links", async () => {
  const source = createResumeDocument();
  source.contact.fullName = "Synthetic Date Tester";
  source.contact.links = [
    { label: "First", url: "mailto:first@example.org" },
    { label: "Second", url: "mailto:second@example.org" },
  ];
  const section = createSection(0);
  const entry = createEntry(0);
  entry.dateRange = "Circa 2020 / ongoing";
  section.entries.push(entry);
  source.sections.push(section);
  const originalBytes = JSON.stringify(source);
  let lastSaved = source;
  let revision = 1;
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(async (command: string, args: unknown) => {
    if (command === "load_resume")
      return {
        ok: true,
        value: {
          draft: { revision: 1, document: source },
          latestPublished: { revision: 1, document: source },
        },
      };
    if (command === "save_resume") {
      const payload = (
        args as { request: { payload: { document: typeof source } } }
      ).request.payload;
      lastSaved = payload.document;
      return { ok: true, value: { revision: ++revision, document: lastSaved } };
    }
    return original?.(command, args);
  });
  const container = await render(<App surface="main" />);
  await act(async () =>
    buttonNamed(container, "Enable structured dates and link ordering").click(),
  );
  expect(buttonNamed(container, "Undo edit").disabled).toBe(true);
  await act(async () => buttonNamed(container, "Save draft").click());
  await settle();
  expect(lastSaved.schemaVersion).toBe(2);
  expect(lastSaved.sections[0].entries[0].dateRange).toBe(entry.dateRange);
  expect(lastSaved.sections[0].entries[0].dates).toEqual([]);
  expect(JSON.stringify(source)).toBe(originalBytes);
  expect(
    native.invoke.mock.calls.some(([command]) => command === "publish_resume"),
  ).toBe(false);
  const linkIds = lastSaved.contact.links.map((link) => link.id);
  await act(async () =>
    buttonNamed(container, `Edit ${section.heading}`).click(),
  );
  let entryPanel = container.querySelector(".resume-entry")!;
  await act(async () =>
    buttonNamed(entryPanel, "Use date fields instead").click(),
  );
  expect(entryPanel.textContent).toContain("Circa 2020 / ongoing");
  await act(async () => buttonNamed(entryPanel, "Keep date text").click());
  expect(inputInLabel(entryPanel, "Date range").value).toBe(entry.dateRange);
  await act(async () =>
    buttonNamed(entryPanel, "Use date fields instead").click(),
  );
  await act(async () => buttonNamed(entryPanel, "Replace date text").click());
  await act(async () =>
    inputValue(inputInLabel(entryPanel, "Start or single date year"), "2025"),
  );
  const endChoice =
    Array.from(entryPanel.querySelectorAll("label"))
      .find(
        (label) =>
          label.textContent?.trim() ===
          "End dateNo end dateYear or month and yearPresent / current",
      )
      ?.querySelector("select") ??
    entryPanel.querySelector<HTMLSelectElement>(".date-record > label select")!;
  expect(endChoice).not.toBeNull();
  await act(async () => {
    endChoice.value = "present";
    endChoice.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  expect(entryPanel.textContent).toContain("2025–Present");
  await act(async () => {
    endChoice.value = "date";
    endChoice.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  expect(buttonNamed(container, "Save draft").disabled).toBe(true);
  expect(entryPanel.textContent).toContain("Enter a year from 1 to 9999");
  await act(async () =>
    inputValue(inputInLabel(entryPanel, "End year"), "2020"),
  );
  expect(entryPanel.textContent).toContain("You can still save it.");
  expect(buttonNamed(container, "Save draft").disabled).toBe(false);
  await act(async () => buttonNamed(container, "Save draft").click());
  await settle();
  const savedDate = lastSaved.sections[0].entries[0].dates![0];
  expect(savedDate.start).toEqual({ year: 2025, month: null, expected: false });
  expect(lastSaved.sections[0].entries[0].dateRange).toBe("");
  await act(async () => buttonNamed(entryPanel, "Remove date 1").click());
  await act(async () => buttonNamed(container, "Undo edit").click());
  expect(entryPanel.textContent).toContain("2025–2020");
  await act(async () => buttonNamed(container, "Edit contact").click());
  const down = container.querySelector<HTMLButtonElement>(
    'button[aria-label="Move link 1 down"]',
  )!;
  await act(async () => down.click());
  await act(async () => buttonNamed(container, "Save draft").click());
  await settle();
  expect(lastSaved.contact.links.map((link) => link.id)).toEqual(
    [...linkIds].reverse(),
  );
  expect(lastSaved.contact.links.map((link) => link.order)).toEqual([0, 1]);
  expect(lastSaved.sections[0].entries[0].dates![0].id).toBe(savedDate.id);
  await act(async () =>
    buttonNamed(container, `Edit ${section.heading}`).click(),
  );
  entryPanel = container.querySelector(".resume-entry")!;
  await act(async () => buttonNamed(entryPanel, "Add date").click());
  const secondDate = entryPanel.querySelectorAll(".date-record")[1];
  await act(async () =>
    inputValue(inputInLabel(secondDate, "Start or single date year"), "2027"),
  );
  const month = Array.from(secondDate.querySelectorAll("label"))
    .find((label) =>
      label.textContent?.startsWith("Start or single date month"),
    )!
    .querySelector("select")!;
  await act(async () => {
    month.value = "6";
    month.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
    secondDate
      .querySelector<HTMLInputElement>('input[type="checkbox"]')!
      .click();
  });
  expect(secondDate.textContent).toContain("Expected Jun 2027");
  await act(async () => buttonNamed(container, "Save draft").click());
  await settle();
  const secondId = lastSaved.sections[0].entries[0].dates![1].id;
  await act(async () =>
    entryPanel
      .querySelector<HTMLButtonElement>('button[aria-label="Move date 2 up"]')!
      .click(),
  );
  await act(async () => buttonNamed(container, "Save draft").click());
  await settle();
  expect(
    lastSaved.sections[0].entries[0].dates!.map((date) => date.id),
  ).toEqual([secondId, savedDate.id]);
  expect(
    lastSaved.sections[0].entries[0].dates!.map((date) => date.order),
  ).toEqual([0, 1]);
  expect(lastSaved.sections[0].entries[0].dates![0].start).toEqual({
    year: 2027,
    month: 6,
    expected: true,
  });
  expect(JSON.stringify(source)).toBe(originalBytes);
  await expectSurfaceAccessible(container);
});

it("adds suggested education details without saving examples or changing existing content on rename", async () => {
  const source = createResumeDocument();
  const section = createSection(0);
  section.heading = "Education";
  const entry = createEntry(0);
  section.entries.push(entry);
  source.sections.push(section);
  let saved = source;
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(
    async (command: string, ...args: unknown[]) => {
      if (command === "load_resume")
        return {
          ok: true,
          value: {
            draft: { revision: 1, document: source },
            latestPublished: null,
          },
        };
      if (command === "save_resume") {
        saved = (
          args[0] as { request: { payload: { document: typeof source } } }
        ).request.payload.document;
        return { ok: true, value: { revision: 2, document: saved } };
      }
      return original?.(command, ...args);
    },
  );
  const container = await render(<App surface="main" />);
  await act(async () => buttonNamed(container, "Edit Education").click());
  const heading = inputInLabel(container, "Institution or qualification");
  expect(heading.placeholder).toBe("e.g. University name");
  expect(heading.value).toBe("");
  const details =
    container.querySelector<HTMLDetailsElement>(".custom-fields")!;
  details.open = true;
  const select = details.querySelector("select")!;
  await act(async () => {
    select.value = "GPA";
    select.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  await act(async () => buttonNamed(details, "Add suggested detail").click());
  const value = details.querySelector<HTMLTextAreaElement>("textarea")!;
  expect(document.activeElement).toBe(value);
  expect(inputInLabel(details, "Field 1 label").value).toBe("GPA");
  expect(value.value).toBe("");
  const fieldPath = value
    .closest("label")!
    .getAttribute("data-validation-path");
  await act(async () => buttonNamed(container, "Undo edit").click());
  expect(details.querySelector("textarea")).toBeNull();
  await act(async () => buttonNamed(container, "Redo edit").click());
  const restored = details.querySelector<HTMLTextAreaElement>("textarea")!;
  expect(restored.closest("label")!.getAttribute("data-validation-path")).toBe(
    fieldPath,
  );
  await act(async () => inputValue(restored, "3.8 / 4.0"));
  await act(async () =>
    inputValue(inputInLabel(container, "Section heading"), "Learning"),
  );
  expect(inputInLabel(container, "Role or qualification").value).toBe("");
  expect(inputInLabel(details, "Field 1 label").value).toBe("GPA");
  expect(restored.value).toBe("3.8 / 4.0");
  await act(async () => buttonNamed(container, "Save draft").click());
  await settle();
  expect(saved.sections[0].entries[0].id).toBe(entry.id);
  expect(saved.sections[0].entries[0].heading).toBe("");
  expect(saved.sections[0].entries[0].subheading).toBe("");
  expect(saved.sections[0].entries[0].fields[0]).toMatchObject({
    label: "GPA",
    value: "3.8 / 4.0",
    isSkill: false,
  });
  expect(JSON.stringify(saved)).not.toContain("e.g.");
  expect(source.sections[0].heading).toBe("Education");
  expect(source.sections[0].entries[0].fields).toHaveLength(0);
  await expectSurfaceAccessible(container);
});

it("keeps a live reading view while navigating and closing focused section editors", async () => {
  const source = createResumeDocument();
  source.contact.fullName = "Synthetic Reader";
  const section = createSection(0);
  section.heading = "Experience";
  const entry = createEntry(0);
  entry.heading = "Synthetic Engineer";
  section.entries.push(entry);
  source.sections.push(section);
  const other = createSection(1);
  other.heading = "Education";
  source.sections.push(other);
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(
    async (command: string, ...args: unknown[]) =>
      command === "load_resume"
        ? {
            ok: true,
            value: {
              draft: { revision: 1, document: source },
              latestPublished: null,
            },
          }
        : original?.(command, ...args),
  );
  const container = await render(<App surface="main" />);
  const reading = container.querySelector(
    '[aria-label="Live draft resume content"]',
  )!;
  expect(reading.textContent).toContain("Synthetic Engineer");
  await act(async () => buttonNamed(container, "Edit Experience").click());
  expect(document.activeElement?.getAttribute("aria-label")).toBe(
    "Focused resume editor",
  );
  expect(container.querySelectorAll(".resume-section")).toHaveLength(1);
  expect(container.querySelectorAll(".resume-entry")).toHaveLength(1);
  await act(async () =>
    inputValue(
      inputInLabel(container, "Role or qualification"),
      "Updated Engineer",
    ),
  );
  expect(reading.textContent).toContain("Updated Engineer");
  await act(async () => buttonNamed(container, "Close editor").click());
  expect(
    container.querySelector('[aria-label="Focused resume editor"]'),
  ).toBeNull();
  expect(document.activeElement?.id).toBe("reading-title");
  expect(reading.textContent).toContain("Updated Engineer");
  await act(async () => buttonNamed(reading, "Education").click());
  expect(inputInLabel(container, "Section heading").value).toBe("Education");
  expect(container.querySelectorAll(".resume-entry")).toHaveLength(0);
  await act(async () => buttonNamed(reading, "Add entry to Education").click());
  const newHeading = inputInLabel(container, "Institution or qualification");
  expect(document.activeElement).toBe(newHeading);
  expect(newHeading.value).toBe("");
  await act(async () => buttonNamed(container, "Close editor").click());
  await act(async () => buttonNamed(reading, "Edit untitled entry 1").click());
  expect(document.activeElement).toBe(
    inputInLabel(container, "Institution or qualification"),
  );
  await act(async () => buttonNamed(container, "Undo edit").click());
  expect(reading.textContent).not.toContain("Edit untitled entry");
  await act(async () => buttonNamed(container, "Redo edit").click());
  expect(reading.textContent).toContain("Edit untitled entry 1");
  expect(source.sections[1].entries).toHaveLength(0);
  await act(async () => buttonNamed(container, "Collapse sections").click());
  expect(container.querySelector("#resume-navigation")).toBeNull();
  expect(reading.textContent).toContain("Updated Engineer");
  expect(
    native.invoke.mock.calls.some(
      ([command]) =>
        command === "publish_resume" || command === "render_resume_pdf",
    ),
  ).toBe(false);
  await expectSurfaceAccessible(container);
});

it("collapses entries, duplicates independently, and confirms removal with undo", async () => {
  const source = createResumeDocument();
  const section = createSection(0);
  section.heading = "Experience";
  const first = createEntry(0);
  first.heading = "First role";
  const second = createEntry(1);
  second.heading = "Second role";
  section.entries = [first, second];
  source.sections = [section];
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(
    async (command: string, ...args: unknown[]) =>
      command === "load_resume"
        ? {
            ok: true,
            value: {
              draft: { revision: 1, document: source },
              latestPublished: null,
            },
          }
        : original?.(command, ...args),
  );
  const container = await render(<App surface="main" />);
  await act(async () => buttonNamed(container, "Edit Experience").click());
  expect(container.querySelectorAll(".resume-entry")).toHaveLength(1);
  await act(async () => buttonNamed(container, "Second role · Edit").click());
  expect(inputInLabel(container, "Role or qualification").value).toBe(
    "Second role",
  );
  await act(async () => buttonNamed(container, "Duplicate entry").click());
  expect(container.querySelectorAll(".entry-focus-toggle")).toHaveLength(3);
  expect(document.activeElement?.getAttribute("aria-expanded")).toBe("true");
  await act(async () =>
    inputValue(inputInLabel(container, "Role or qualification"), "Copied role"),
  );
  expect(buttonNamed(container, "Second role · Edit")).toBeTruthy();
  await act(async () => buttonNamed(container, "Remove entry").click());
  await expectSurfaceAccessible(container);
  await act(async () => buttonNamed(container, "Keep it").click());
  expect(document.activeElement).toBe(buttonNamed(container, "Remove entry"));
  expect(container.querySelectorAll(".entry-focus-toggle")).toHaveLength(3);
  await act(async () => buttonNamed(container, "Remove entry").click());
  await act(async () => buttonNamed(container, "Confirm remove entry").click());
  expect(container.querySelectorAll(".entry-focus-toggle")).toHaveLength(2);
  expect(document.activeElement).toBe(
    inputInLabel(container, "Section heading"),
  );
  await act(async () => buttonNamed(container, "Undo edit").click());
  expect(buttonNamed(container, "Copied role · Edit")).toBeTruthy();
  await act(async () => buttonNamed(container, "Remove section").click());
  expect(container.querySelectorAll(".resume-section")).toHaveLength(1);
  await act(async () =>
    buttonNamed(container, "Confirm remove section").click(),
  );
  expect(container.querySelectorAll(".resume-section")).toHaveLength(0);
  expect(document.activeElement?.getAttribute("aria-label")).toBe(
    "Focused resume editor",
  );
  await act(async () => buttonNamed(container, "Undo edit").click());
  await act(async () => buttonNamed(container, "Edit Experience").click());
  expect(container.querySelectorAll(".entry-focus-toggle")).toHaveLength(3);
  expect(source.sections[0].entries).toHaveLength(2);
  await expectSurfaceAccessible(container);
});

it("opens collapsed invalid fields and supports repeated validation navigation", async () => {
  const source = createResumeDocument();
  source.title = "";
  const section = createSection(0);
  section.heading = "Experience";
  const first = createEntry(0);
  first.heading = "First role";
  const second = createEntry(1);
  second.heading = "Second role";
  second.links = [{ label: "Portfolio", url: "invalid" }];
  second.dates = [
    {
      id: createEntityId(),
      order: 0,
      label: "Employment",
      start: { year: 2020, month: null, expected: false },
      end: null,
    },
  ];
  section.entries = [first, second];
  source.sections = [section];
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(
    async (command: string, ...args: unknown[]) =>
      command === "load_resume"
        ? {
            ok: true,
            value: {
              draft: { revision: 1, document: upgradeDocumentV2(source) },
              latestPublished: null,
            },
          }
        : original?.(command, ...args),
  );
  const container = await render(<App surface="main" />);
  const jump =
    "Experience: Second role: Enter a complete HTTP, HTTPS, or mailto URL.";
  await act(async () => buttonNamed(container, jump).click());
  const year = container.querySelector<HTMLInputElement>(
    '.date-record input[type="number"]',
  )!;
  await act(async () => inputValue(year, "10000"));
  await act(async () => buttonNamed(container, "Close editor").click());
  await act(async () =>
    buttonNamed(
      container,
      "Experience: Second role: Use a year from 1 to 9999 and a month from 1 to 12, or leave the month blank.",
    ).click(),
  );
  expect(document.activeElement?.getAttribute("type")).toBe("number");
  expect(document.activeElement?.getAttribute("aria-invalid")).toBe("true");
  await act(async () => buttonNamed(container, jump).click());
  expect((document.activeElement as HTMLInputElement).value).toBe("invalid");
  expect(document.activeElement?.getAttribute("aria-invalid")).toBe("true");
  await act(async () => buttonNamed(container, "First role · Edit").click());
  await act(async () => buttonNamed(container, jump).click());
  expect((document.activeElement as HTMLInputElement).value).toBe("invalid");
  await act(async () =>
    buttonNamed(
      container,
      "Contact and resume details: Enter a resume title.",
    ).click(),
  );
  expect(document.activeElement).toBe(inputInLabel(container, "Resume title"));
  await act(async () => buttonNamed(container, "Edit Experience").click());
  expect(inputInLabel(container, "Role or qualification").value).toBe(
    "First role",
  );
  await act(async () => buttonNamed(container, "Add bullet").click());
  const bullet = container.querySelector<HTMLTextAreaElement>(
    ".bullet-row textarea",
  )!;
  await act(async () => inputValue(bullet, "Synthetic bullet"));
  await act(async () =>
    container
      .querySelector<HTMLButtonElement>('[aria-label="Duplicate bullet 1"]')!
      .click(),
  );
  expect(
    Array.from(
      container.querySelectorAll<HTMLTextAreaElement>(".bullet-row textarea"),
      (field) => field.value,
    ),
  ).toEqual(["Synthetic bullet", "Synthetic bullet"]);
  await act(async () => buttonNamed(container, "Undo edit").click());
  expect(container.querySelectorAll(".bullet-row")).toHaveLength(1);
  await expectSurfaceAccessible(container);
});

it("labels current-renderer regeneration without replacing or replaying historical bytes", async () => {
  const receipt = {
    documentSha256: "a".repeat(64),
    documentSchemaVersion: 1,
    pdfSha256: "b".repeat(64),
    rendererVersion: "historical-renderer",
    templateId: "retired_pdf_v1",
    templateSha256: "c".repeat(64),
    fontBundleId: "retired-fonts/v1",
    fontBundleSha256: "d".repeat(64),
    pageCount: 1,
    byteCount: 5,
  };
  const manifest = {
    manifestId: "019a0000-0000-7000-8000-000000000002",
    source: "published_snapshot",
    sourceRevision: 1,
    generatedAtUnixMs: 1000,
    lastGeneratedAtUnixMs: 1000,
    renderCount: 1,
    receipt,
  };
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(
    async (command: string, ...args: unknown[]) => {
      if (command === "load_pdf_render_history")
        return { ok: true, value: { manifests: [manifest] } };
      if (command === "regenerate_resume_pdf")
        return {
          ok: true,
          value: {
            accessibleText: "Synthetic retained publication\n",
            preview: {
              renderId: "019a0000-0000-7000-8000-000000000001",
              source: "published_snapshot",
              revision: 1,
              generatedAtUnixMs: Date.now(),
              pdfBase64: "JVBERi0=",
              receipt: {
                ...receipt,
                rendererVersion: "typst-0.15.1/ort-1",
                templateId: "technical_pdf_v1",
                fontBundleId: "libertinus-serif/typst-assets-0.15.1",
              },
            },
          },
        };
      return original?.(command, ...args);
    },
  );
  const container = await render(<App surface="main" />);
  await act(async () =>
    buttonNamed(container, "Regenerate with current renderer").click(),
  );
  await settle();
  expect(native.invoke).toHaveBeenCalledWith("regenerate_resume_pdf", {
    request: expect.objectContaining({
      payload: { manifestId: manifest.manifestId, style: "technical" },
    }),
  });
  expect(container.textContent).toContain(
    "Regenerated with current renderer — new output from retained revision 1.",
  );
  expect(container.textContent).toContain(
    "this preview is not an exact replay",
  );
  expect(container.textContent).toContain("historical-renderer");
  expect(
    native.invoke.mock.calls.some(([command]) =>
      ["save_resume", "publish_resume", "replay_resume_pdf"].includes(command),
    ),
  ).toBe(false);
  expect(manifest.receipt).toBe(receipt);
  await expectSurfaceAccessible(container);
});

it("opens the selected reading entry and reveals optional details for validation", async () => {
  const source = createResumeDocument();
  const section = createSection(0);
  section.heading = "Experience";
  const first = createEntry(0);
  first.heading = "First role";
  const second = createEntry(1);
  second.heading = "Second role";
  second.fields = [
    {
      id: createEntityId(),
      order: 0,
      label: "Technologies",
      value: "Rust",
      isSkill: false,
    },
  ];
  section.entries = [first, second];
  source.sections = [section];
  const original = native.invoke.getMockImplementation();
  native.invoke.mockImplementation(
    async (command: string, ...args: unknown[]) =>
      command === "load_resume"
        ? {
            ok: true,
            value: {
              draft: { revision: 1, document: source },
              latestPublished: null,
            },
          }
        : original?.(command, ...args),
  );
  const container = await render(<App surface="main" />);
  const reading = container.querySelector(
    '[aria-label="Live draft resume content"]',
  )!;
  await act(async () => buttonNamed(reading, "First role").click());
  expect(document.activeElement).toBe(
    inputInLabel(container, "Role or qualification"),
  );
  expect(inputInLabel(container, "Role or qualification").value).toBe(
    "First role",
  );
  await act(async () => buttonNamed(container, "Close editor").click());
  await act(async () => buttonNamed(reading, "Second role").click());
  expect(document.activeElement).toBe(
    inputInLabel(container, "Role or qualification"),
  );
  expect(inputInLabel(container, "Role or qualification").value).toBe(
    "Second role",
  );
  const optional = container.querySelector<HTMLDetailsElement>(
    "details.custom-fields",
  )!;
  expect(optional.open).toBe(false);
  expect(reading.textContent).toContain("Rust");
  await act(async () => optional.querySelector("summary")!.click());
  expect(optional.open).toBe(true);
  const value = optional.querySelector<HTMLTextAreaElement>("textarea")!;
  await act(async () => inputValue(value, "x".repeat(2001)));
  await act(async () => optional.querySelector("summary")!.click());
  expect(optional.open).toBe(false);
  await act(async () =>
    buttonNamed(
      container,
      "Experience: Second role: Use at most 2000 characters.",
    ).click(),
  );
  expect(optional.open).toBe(true);
  expect(document.activeElement).toBe(value);
  await act(async () => inputValue(value, "Rust"));
  await act(async () => buttonNamed(container, "Duplicate entry").click());
  expect(container.textContent).toContain("Matching entries found (1 group)");
  expect(
    container.querySelector('[aria-label="Resume validation"]'),
  ).toBeNull();
  await act(async () => buttonNamed(container, "Undo edit").click());
  expect(container.textContent).not.toContain("Matching entries found");
  expect(source.sections[0].entries).toHaveLength(2);
  await expectSurfaceAccessible(container);
});

it.each(["edit", "discard"])(
  "releases a late native preview after %s without displaying it",
  async (action) => {
    const original = native.invoke.getMockImplementation();
    let finish: ((value: unknown) => void) | undefined;
    native.invoke.mockImplementation(
      async (command: string, ...args: unknown[]) =>
        command === "render_resume_pdf"
          ? new Promise((resolve) => {
              finish = resolve;
            })
          : original?.(command, ...args),
    );
    const container = await render(<App surface="main" />);
    const view = Array.from(container.querySelectorAll("label"))
      .find((label) => label.textContent?.includes("Resume view"))!
      .querySelector("select")!;
    await act(async () => {
      view.value = "pdf";
      view.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
    });
    await act(async () =>
      buttonNamed(container, "Preview saved draft").click(),
    );
    expect(buttonNamed(container, "Preview saved draft").disabled).toBe(true);
    if (action === "edit") {
      await act(async () =>
        inputValue(
          inputInLabel(container, "Full name"),
          "Changed during render",
        ),
      );
    } else {
      await act(async () =>
        buttonNamed(container, "Discard pending preview").click(),
      );
      expect(buttonNamed(container, "Preview saved draft").disabled).toBe(true);
    }
    const renderId = "019a0000-0000-7000-8000-000000000001";
    await act(async () =>
      finish?.({
        ok: true,
        value: {
          renderId,
          source: "saved_draft",
          revision: 1,
          generatedAtUnixMs: Date.now(),
          pdfBase64: "JVBERi0=",
          receipt: {
            documentSha256: "a".repeat(64),
            documentSchemaVersion: 1,
            pdfSha256: "b".repeat(64),
            rendererVersion: "typst-0.15.1/ort-1",
            templateId: "technical_pdf_v1",
            templateSha256: "c".repeat(64),
            fontBundleId: "libertinus-serif/typst-assets-0.15.1",
            fontBundleSha256: "d".repeat(64),
            pageCount: 1,
            byteCount: 5,
          },
        },
      }),
    );
    await settle();
    expect(container.textContent).toContain(
      action === "edit"
        ? "late result was discarded"
        : "Pending preview discarded",
    );
    expect(container.querySelector("canvas")).toBeNull();
    expect(container.textContent).not.toContain("Export this preview (.pdf)");
    expect(native.invoke).toHaveBeenCalledWith("release_resume_pdf", {
      request: expect.objectContaining({ payload: { renderId } }),
    });
    expect(
      native.invoke.mock.calls.some(
        ([command]) => command === "export_resume_pdf",
      ),
    ).toBe(false);
    if (action === "discard")
      expect(buttonNamed(container, "Preview saved draft").disabled).toBe(
        false,
      );
    await expectSurfaceAccessible(container);
  },
);

it("refreshes saved pages once per identity and pauses while the reading view is selected", async () => {
  const container = await render(<App surface="main" />);
  const view = Array.from(container.querySelectorAll("label"))
    .find((label) => label.textContent?.includes("Resume view"))!
    .querySelector("select")!;
  const automatic = container.querySelector<HTMLInputElement>(
    ".pdf-auto-refresh input",
  )!;
  await act(async () => {
    view.value = "pdf";
    view.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
    automatic.click();
  });
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 750));
  });
  await settle();
  const renders = () =>
    native.invoke.mock.calls.filter(
      ([command]) => command === "render_resume_pdf",
    );
  expect(renders()).toHaveLength(1);
  // The synthetic native command fails. It must not trigger an automatic loop.
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 750));
  });
  expect(renders()).toHaveLength(1);
  await act(async () => {
    view.value = "reading";
    view.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  const style = Array.from(container.querySelectorAll("label"))
    .find((label) => label.textContent?.includes("Document style"))!
    .querySelector("select")!;
  await act(async () => {
    style.value = "modern";
    style.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 750));
  });
  expect(renders()).toHaveLength(1);
  await act(async () => {
    view.value = "pdf";
    view.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 750));
  });
  await settle();
  expect(renders()).toHaveLength(2);
  expect(renders()[1][1]).toEqual({
    request: expect.objectContaining({
      payload: {
        source: "saved_draft",
        expectedRevision: 1,
        style: "modern",
      },
    }),
  });
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === "save_resume" || command === "publish_resume",
    ),
  ).toBe(false);
});
