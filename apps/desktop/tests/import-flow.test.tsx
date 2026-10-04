// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ImportReviewSnapshot } from "@ort/contracts/import";
import { ImportReviewFlow } from "../src/shared/ImportReviewFlow";
import {
  createEntry,
  createResumeDocument,
  createSection,
  upgradeDocumentV2,
} from "../src/shared/resume-editor";
const client = vi.hoisted(() => ({
  readImportReview: vi.fn(),
  mapImportReview: vi.fn(),
  cancelImportReview: vi.fn(),
}));
vi.mock("../src/shared/import-client", () => client);
let host: HTMLDivElement;
let root: Root;
let snapshot: ImportReviewSnapshot;
const saved = vi.fn();
const cancelled = vi.fn();
function button(text: string) {
  return Array.from(host.querySelectorAll("button")).find(
    (button) =>
      button.textContent?.trim() === text &&
      !button.closest("dialog:not([open])"),
  )!;
}
function pointer(target: EventTarget, type: string, x: number, y: number) {
  const event = new MouseEvent(type, {
    bubbles: true,
    button: 0,
    clientX: x,
    clientY: y,
  });
  Object.defineProperty(event, "pointerId", { value: 1 });
  target.dispatchEvent(event);
}
async function dragFirstSectionToTrash() {
  const nav = host.querySelector<HTMLElement>(".document-navigator")!;
  const card = nav.querySelector<HTMLElement>(".section-nav-card")!;
  const list = nav.querySelector<HTMLElement>(".section-sort-list")!;
  const trash = nav.querySelector<HTMLElement>(".section-trash")!;
  for (const [element, top, bottom] of [
    [nav, 0, 500],
    [list, 0, 250],
    [card, 0, 40],
    [trash, 300, 350],
  ] as const) {
    vi.spyOn(element, "getBoundingClientRect").mockReturnValue({
      left: 0,
      right: 200,
      top,
      bottom,
      width: 200,
      height: bottom - top,
    } as DOMRect);
  }
  await act(async () => {
    pointer(card, "pointerdown", 10, 10);
    pointer(window, "pointermove", 10, 320);
    pointer(window, "pointerup", 10, 320);
  });
}
beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.clearAllMocks();
  Object.defineProperties(HTMLDialogElement.prototype, {
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
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  const importedDocument = upgradeDocumentV2(createResumeDocument());
  importedDocument.contact.fullName = "Imported Person";
  importedDocument.sections = ["Experience", "Projects"].map(
    (heading, order) => ({
      ...createSection(order),
      heading,
      entries: [{ ...createEntry(0, 2), heading: `${heading} item` }],
    }),
  );
  snapshot = {
    id: crypto.randomUUID(),
    baseRevision: 1,
    mappingVersion: 1,
    importedDocument,
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
    contacts: { fullName: "Saved Person", email: "", phone: "", location: "" },
  };
  client.readImportReview.mockResolvedValue({ ok: true, value: snapshot });
  client.cancelImportReview.mockResolvedValue({ ok: true, value: true });
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
async function render(revision = 1) {
  await act(async () =>
    root.render(
      <ImportReviewFlow
        reviewId={snapshot.id}
        currentRevision={revision}
        onSaved={saved}
        onCancelled={cancelled}
      />,
    ),
  );
}
it("uses the editor with an isolated draft and submits edited, reordered v2 content only on mapping", async () => {
  client.mapImportReview.mockImplementation(async (_id, document) => ({
    ok: true,
    value: { revision: 2, document },
  }));
  await render();
  expect(host.textContent).toContain("Imported Person");
  expect(host.textContent).not.toContain("Saved Person");
  expect(client.mapImportReview).not.toHaveBeenCalled();
  await act(async () =>
    (
      host.querySelector(
        ".resume-canvas__contact .canvas-field__button",
      ) as HTMLButtonElement
    ).click(),
  );
  await act(async () => {
    const input = host.querySelector<HTMLInputElement>(
      'input[aria-label="Name"]',
    )!;
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )!.set!.call(input, "Reviewed Person");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () =>
    (
      host.querySelector('[aria-label="Rename Projects"]') as HTMLButtonElement
    ).dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "ArrowUp",
        altKey: true,
        bubbles: true,
      }),
    ),
  );
  await act(async () =>
    host
      .querySelector<HTMLButtonElement>('[aria-label="Undo import edit"]')!
      .click(),
  );
  await act(async () =>
    host
      .querySelector<HTMLButtonElement>('[aria-label="Redo import edit"]')!
      .click(),
  );
  expect(snapshot.importedDocument.contact.fullName).toBe("Imported Person");
  expect(snapshot.importedDocument.sections[0].heading).toBe("Experience");
  await act(async () => button("Map to current saved resume").click());
  const submitted = client.mapImportReview.mock.calls[0][1];
  expect(submitted.contact.fullName).toBe("Reviewed Person");
  expect(
    submitted.sections.map((section: { heading: string }) => section.heading),
  ).toEqual(["Projects", "Experience"]);
  expect(
    submitted.sections.map((section: { order: number }) => section.order),
  ).toEqual([0, 1]);
  expect(saved).toHaveBeenCalledWith({ revision: 2, document: submitted });
});
it("cancels without mapping or saving, and preserves the review when cancellation fails", async () => {
  client.cancelImportReview.mockResolvedValueOnce({
    ok: false,
    error: { code: "UNAVAILABLE" },
  });
  await render();
  await act(async () => button("Cancel").click());
  expect(host.textContent).toContain("Cancellation could not be confirmed");
  expect(cancelled).not.toHaveBeenCalled();
  await act(async () => button("Cancel").click());
  expect(cancelled).toHaveBeenCalledOnce();
  expect(client.mapImportReview).not.toHaveBeenCalled();
  expect(saved).not.toHaveBeenCalled();
});
it("keeps section deletion reviewable and undoable without saving the imported draft", async () => {
  await render();
  expect(host.querySelector(".resume-display-header")?.textContent).toContain(
    "Not saved yet",
  );
  await dragFirstSectionToTrash();
  const dialog = host.querySelector<HTMLDialogElement>("dialog[open]")!;
  expect(dialog).not.toBeNull();
  expect(dialog.closest("nav")).toBeNull();
  expect(document.activeElement).toBe(dialog.querySelector("button"));
  expect(host.querySelector(".section-trash-confirmation")).toBeNull();
  expect(host.querySelectorAll(".section-nav-card")).toHaveLength(2);
  await act(async () => button("Delete section").click());
  expect(host.querySelectorAll(".section-nav-card")).toHaveLength(1);
  await act(async () =>
    host
      .querySelector<HTMLButtonElement>('[aria-label="Undo import edit"]')!
      .click(),
  );
  expect(host.querySelectorAll(".section-nav-card")).toHaveLength(2);
  expect(snapshot.importedDocument.sections).toHaveLength(2);
  expect(client.mapImportReview).not.toHaveBeenCalled();
  expect(saved).not.toHaveBeenCalled();
});
it("opens only a rename field on title click and cancels the deletion popup without changing content", async () => {
  await render();
  await act(async () =>
    host
      .querySelector<HTMLButtonElement>('[aria-label="Rename Experience"]')!
      .click(),
  );
  const input = host.querySelector<HTMLInputElement>(
    '[aria-label="Section name Experience"]',
  )!;
  expect(document.activeElement).toBe(input);
  expect(host.querySelector(".section-nav-keyboard-actions")).toBeNull();
  expect(host.querySelectorAll(".section-nav-card button")).toHaveLength(1);
  expect(host.querySelector("dialog[open]")).toBeNull();
  await act(async () => input.blur());
  await dragFirstSectionToTrash();
  await act(async () => button("Cancel").click());
  expect(host.querySelector("dialog[open]")).toBeNull();
  expect(document.activeElement).toBe(
    host.querySelector('[aria-label="Rename Experience"]'),
  );
  expect(host.querySelectorAll(".section-nav-card")).toHaveLength(2);
  await dragFirstSectionToTrash();
  await act(async () =>
    host
      .querySelector("dialog[open]")!
      .dispatchEvent(new Event("cancel", { cancelable: true })),
  );
  expect(host.querySelector("dialog[open]")).toBeNull();
  expect(host.querySelectorAll(".section-nav-card")).toHaveLength(2);
  expect(client.mapImportReview).not.toHaveBeenCalled();
});
it("blocks stale revisions and invalid drafts before submitting", async () => {
  await render(2);
  expect(button("Map to current saved resume").disabled).toBe(true);
  expect(host.textContent).toContain("saved resume changed");
  await render();
  await act(async () =>
    (
      host.querySelector(
        '[aria-label="Rename Experience"]',
      ) as HTMLButtonElement
    ).click(),
  );
  await act(async () => {
    const input = host.querySelector<HTMLInputElement>(
      'input[aria-label="Section name Experience"]',
    )!;
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )!.set!.call(input, "");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(button("Map to current saved resume").disabled).toBe(true);
  expect(host.textContent).toContain("Enter a section heading");
  expect(client.mapImportReview).not.toHaveBeenCalled();
});
it("blocks concurrent submissions and resubmission after an uncertain save while allowing cancellation", async () => {
  let finish!: (value: unknown) => void;
  client.mapImportReview.mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  await render();
  await act(async () => {
    button("Map to current saved resume").click();
    button("Map to current saved resume").click();
  });
  expect(client.mapImportReview).toHaveBeenCalledOnce();
  expect(button("Cancel").disabled).toBe(true);
  await act(async () =>
    finish({ ok: false, error: { code: "OUTCOME_UNKNOWN" } }),
  );
  expect(button("Map to current saved resume").disabled).toBe(true);
  expect(button("Cancel").disabled).toBe(false);
  await act(async () => button("Map to current saved resume").click());
  expect(client.mapImportReview).toHaveBeenCalledOnce();
  expect(saved).not.toHaveBeenCalled();
  await act(async () => button("Cancel").click());
  expect(cancelled).toHaveBeenCalledOnce();
});
it("allows correction and retry after a confirmed validation failure", async () => {
  client.mapImportReview.mockResolvedValueOnce({
    ok: false,
    error: { code: "IMPORT_REVIEW_INVALID" },
  });
  await render();
  await act(async () => button("Map to current saved resume").click());
  expect(button("Map to current saved resume").disabled).toBe(false);
  expect(host.textContent).toContain("Correct the imported resume");
});

it("requests a saved-workspace refresh after an uncertain mapping and keeps the draft visible", async () => {
  const reloadRequired = vi.fn();
  client.mapImportReview.mockResolvedValue({
    ok: false,
    error: { code: "IMPORT_REVIEW_OUTCOME_UNKNOWN" },
  });
  await act(async () =>
    root.render(
      <ImportReviewFlow
        reviewId={snapshot.id}
        initialSnapshot={snapshot}
        currentRevision={1}
        onSaved={saved}
        onCancelled={cancelled}
        onReloadRequired={reloadRequired}
      />,
    ),
  );
  expect(client.readImportReview).not.toHaveBeenCalled();
  await act(async () => button("Map to current saved resume").click());
  expect(reloadRequired).toHaveBeenCalledOnce();
  expect(host.textContent).toContain("Imported Person");
  expect(host.textContent).toContain("save was not confirmed");
  expect(saved).not.toHaveBeenCalled();
});
