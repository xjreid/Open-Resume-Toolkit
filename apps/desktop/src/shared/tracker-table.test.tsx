import { createResumeDocument } from "./resume-editor";
// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { TrackerWorkspace } from "./TrackerTableWorkspace";
import { emptyTrackerEntry } from "./TrackerFields";

function summary(record: { id: string; revision: number; value: unknown }) {
  const entry = record.value as ReturnType<typeof emptyTrackerEntry>;
  return {
    id: record.id,
    revision: record.revision,
    value: {
      company: entry.company,
      title: entry.title,
      location: entry.location,
      dateApplied: entry.dateApplied,
      status: entry.status,
      customStatus: entry.customStatus,
      sourceUrl: entry.sourceUrl,
    },
    hasResume: !!entry.resume,
    hasCoverLetter: !!entry.coverLetter,
    answerCount: entry.answers?.length ?? 0,
  };
}
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("./ApplicationViews", () => ({
  PdfCanvas: ({ base64 }: { base64: string }) => <div>{base64}</div>,
}));
afterEach(() => vi.clearAllMocks());

it("edits rows directly, autosaves, and keeps content read-only", async () => {
  const older = {
    id: "older",
    revision: 1,
    value: {
      ...emptyTrackerEntry(),
      company: "Birch Labs",
      dateApplied: "2026-09-01",
    },
  };
  const newer = {
    id: "newer",
    revision: 1,
    value: {
      ...emptyTrackerEntry(),
      company: "Acme Studio",
      dateApplied: "2026-09-20",
      resume: { title: "Final resume" },
    },
  };
  let revision = 1;
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    const input = args as
      | { id?: string | null; entry?: unknown; metadata?: unknown }
      | undefined;
    if (name === "list_tracker_entries")
      return {
        ok: true,
        value: [older, newer]
          .filter((record) =>
            record.value.company
              .toLowerCase()
              .includes(
                String(
                  (args as { search?: string })?.search ?? "",
                ).toLowerCase(),
              ),
          )
          .map(summary),
      };
    if (name === "save_tracker_metadata" || name === "save_tracker_entry")
      return {
        ok: true,
        value: summary({
          id: input?.id ?? "created",
          revision: ++revision,
          value:
            input?.id === "newer"
              ? { ...newer.value, ...(input?.metadata as object) }
              : (input?.metadata ?? input?.entry),
        }),
      };
    if (name === "get_tracker_entry")
      return {
        ok: true,
        value: {
          ...newer,
          value: { ...newer.value, resume: createResumeDocument() },
        },
      };
    if (name === "preview_tracker_pdf")
      return {
        ok: true,
        value: { base64: "sample-pdf", filename: "final.pdf" },
      };
    if (name === "delete_tracker_entry") return { ok: true, value: true };
    throw new Error("Unexpected command: " + name);
  });
  const showModal = HTMLDialogElement.prototype.showModal;
  const close = HTMLDialogElement.prototype.close;
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const setInput = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  const setSelect = Object.getOwnPropertyDescriptor(
    HTMLSelectElement.prototype,
    "value",
  )?.set;
  if (!setInput || !setSelect) throw new Error("DOM setters missing");
  const companies = () =>
    [
      ...host.querySelectorAll<HTMLInputElement>("tbody td:nth-child(3) input"),
    ].map((input) => input.value);
  try {
    await act(async () =>
      root.render(<TrackerWorkspace active onDirtyChange={() => {}} />),
    );
    expect(
      [...host.querySelectorAll<HTMLTableCellElement>("thead th")]
        .slice(0, 2)
        .map((cell) => cell.textContent),
    ).toEqual(["Date applied", "Status"]);
    expect(companies()).toEqual(["Acme Studio", "Birch Labs"]);
    expect(host.textContent).not.toContain("Save changes");
    const status = host.querySelector<HTMLSelectElement>(
      'select[aria-label="Status for Acme Studio"]',
    );
    if (!status) throw new Error("Status select missing");
    await act(async () => {
      setSelect.call(status, "interview");
      status.dispatchEvent(new Event("change", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith(
      "save_tracker_metadata",
      expect.objectContaining({
        id: "newer",
        metadata: expect.objectContaining({ status: "interview" }),
      }),
    );

    const search = host.querySelector<HTMLInputElement>(
      'input[placeholder="Search all application columns"]',
    );
    if (!search) throw new Error("Search missing");
    await act(async () => {
      setInput.call(search, "Birch");
      search.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(companies()).toEqual(["Birch Labs"]);
    await act(async () => {
      setInput.call(search, "");
      search.dispatchEvent(new Event("input", { bubbles: true }));
    });

    const company = host.querySelector<HTMLInputElement>(
      'input[aria-label="Company for Acme Studio"]',
    );
    if (!company) throw new Error("Company missing");
    await act(async () => {
      setInput.call(company, "Acme Labs");
      company.dispatchEvent(new Event("input", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 450));
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith(
      "save_tracker_metadata",
      expect.objectContaining({
        id: "newer",
        metadata: expect.objectContaining({ company: "Acme Labs" }),
      }),
    );

    const resume = [
      ...host.querySelectorAll<HTMLButtonElement>("tbody button"),
    ].find((button) => button.textContent === "Final resume");
    await act(async () => resume?.click());
    expect(
      host.querySelector(".tracker-content-dialog")?.textContent,
    ).toContain("sample-pdf");
    expect(host.querySelector(".tracker-content-dialog textarea")).toBeNull();
    await act(async () =>
      host
        .querySelector<HTMLButtonElement>(".tracker-content-dialog button")
        ?.click(),
    );

    await act(async () =>
      [...host.querySelectorAll<HTMLButtonElement>("button")]
        .find((button) => button.textContent === "New entry")
        ?.click(),
    );
    expect(
      host.querySelector(".tracker-new-dialog")?.hasAttribute("open"),
    ).toBe(true);
    expect(companies()).toHaveLength(2);
    const newCompany = host.querySelector<HTMLInputElement>(
      ".tracker-new-dialog .tracker-fields input",
    );
    if (!newCompany) throw new Error("New company missing");
    await act(async () => {
      setInput.call(newCompany, "Cedar Works");
      newCompany.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () =>
      [
        ...host.querySelectorAll<HTMLButtonElement>(
          ".tracker-new-dialog button",
        ),
      ]
        .find((button) => button.textContent === "Add application")
        ?.click(),
    );
    expect(companies()).toContain("Cedar Works");

    await act(async () =>
      host
        .querySelector<HTMLButtonElement>(
          'button[aria-label="Delete Birch Labs"]',
        )
        ?.click(),
    );
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith(
      "delete_tracker_entry",
      expect.anything(),
    );
    expect(
      host.querySelector(".tracker-confirm-dialog")?.hasAttribute("open"),
    ).toBe(true);
    await act(async () =>
      [
        ...host.querySelectorAll<HTMLButtonElement>(
          ".tracker-confirm-dialog button",
        ),
      ]
        .find((button) => button.textContent === "Delete application")
        ?.click(),
    );
    expect(vi.mocked(invoke)).toHaveBeenCalledWith(
      "delete_tracker_entry",
      expect.objectContaining({ id: "older" }),
    );
    expect(companies()).not.toContain("Birch Labs");
  } finally {
    await act(async () => root.unmount());
    host.remove();
    HTMLDialogElement.prototype.showModal = showModal;
    HTMLDialogElement.prototype.close = close;
  }
});

it("keeps an unsaved row editable after a failed autosave and retries it", async () => {
  const record = {
    id: "one",
    revision: 1,
    value: { ...emptyTrackerEntry(), company: "Northstar" },
  };
  let attempts = 0;
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "list_tracker_entries")
      return { ok: true, value: [record].map(summary) };
    if (name === "save_tracker_metadata" || name === "save_tracker_entry") {
      attempts += 1;
      if (attempts === 1)
        return {
          ok: false,
          error: {
            code: "STORAGE_UNAVAILABLE",
            messageKey: "errors.storage",
            retryable: true,
            details: {},
          },
        };
      return {
        ok: true,
        value: summary({
          id: "one",
          revision: 2,
          value: (args as { metadata: unknown }).metadata,
        }),
      };
    }
    throw new Error("Unexpected command: " + name);
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const onDirtyChange = vi.fn();
  try {
    await act(async () =>
      root.render(<TrackerWorkspace active onDirtyChange={onDirtyChange} />),
    );
    const status = host.querySelector<HTMLSelectElement>(
      'select[aria-label="Status for Northstar"]',
    );
    if (!status) throw new Error("Status select missing");
    const setSelect = Object.getOwnPropertyDescriptor(
      HTMLSelectElement.prototype,
      "value",
    )?.set;
    if (!setSelect) throw new Error("Select setter missing");
    await act(async () => {
      setSelect.call(status, "interview");
      status.dispatchEvent(new Event("change", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(status.value).toBe("interview");
    expect(
      status.closest("tr")?.classList.contains("tracker-status--interview"),
    ).toBe(true);
    expect(host.textContent).toContain("Some changes could not be saved");
    expect(onDirtyChange).toHaveBeenLastCalledWith(true);
    await act(async () =>
      [...host.querySelectorAll<HTMLButtonElement>("button")]
        .find((button) => button.textContent === "Retry saving")
        ?.click(),
    );
    expect(attempts).toBe(2);
    expect(host.textContent).not.toContain("Some changes could not be saved");
    expect(onDirtyChange).toHaveBeenLastCalledWith(false);
  } finally {
    await act(async () => root.unmount());
    host.remove();
  }
});

it("serializes edits made while a previous autosave is in flight", async () => {
  const record = {
    id: "one",
    revision: 1,
    value: { ...emptyTrackerEntry(), company: "Northstar" },
  };
  let resolveFirst: ((value: { ok: true; value: unknown }) => void) | undefined;
  const saves: {
    expectedRevision: number;
    metadata: { company: string; status: string };
  }[] = [];
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "list_tracker_entries")
      return { ok: true, value: [record].map(summary) };
    if (name === "save_tracker_metadata" || name === "save_tracker_entry") {
      const input = args as {
        expectedRevision: number;
        metadata: { company: string; status: string };
      };
      saves.push(input);
      if (saves.length === 1)
        return new Promise((resolve) => {
          resolveFirst = resolve;
        });
      record.revision = 3;
      record.value = { ...record.value, ...input.metadata };
      return { ok: true, value: summary(record) };
    }
    throw new Error("Unexpected command: " + name);
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () =>
      root.render(<TrackerWorkspace active onDirtyChange={() => {}} />),
    );
    const status = host.querySelector<HTMLSelectElement>(
      'select[aria-label="Status for Northstar"]',
    );
    const company = host.querySelector<HTMLInputElement>(
      'input[aria-label="Company for Northstar"]',
    );
    if (!status || !company) throw new Error("Tracker fields missing");
    const setSelect = Object.getOwnPropertyDescriptor(
      HTMLSelectElement.prototype,
      "value",
    )?.set;
    const setInput = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )?.set;
    if (!setSelect || !setInput) throw new Error("DOM setters missing");
    await act(async () => {
      setSelect.call(status, "interview");
      status.dispatchEvent(new Event("change", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(saves).toHaveLength(1);
    await act(async () => {
      setInput.call(company, "Northstar Labs");
      company.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => {
      resolveFirst?.({
        ok: true,
        value: summary({ id: "one", revision: 2, value: saves[0]?.metadata }),
      });
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(saves).toHaveLength(2);
    expect(saves[1]?.expectedRevision).toBe(2);
    expect(saves[1]?.metadata).toMatchObject({
      company: "Northstar Labs",
      status: "interview",
    });
    expect(
      host.querySelector<HTMLInputElement>("tbody td:nth-child(3) input")
        ?.value,
    ).toBe("Northstar Labs");
  } finally {
    await act(async () => root.unmount());
    host.remove();
  }
});

it("opens a source on one click and edits arbitrary source text on a double-click", async () => {
  const record = {
    id: "one",
    revision: 1,
    value: {
      ...emptyTrackerEntry(),
      company: "Northstar",
      sourceUrl: "linkedin.com/jobs/123",
    },
  };
  vi.mocked(invoke).mockImplementation(async (name, args) => {
    if (name === "list_tracker_entries")
      return { ok: true, value: [record].map(summary) };
    if (name === "open_tracker_link") return { ok: true, value: true };
    if (name === "save_tracker_metadata" || name === "save_tracker_entry")
      return {
        ok: true,
        value: summary({
          id: "one",
          revision: 2,
          value: (args as { metadata: unknown }).metadata,
        }),
      };
    throw new Error("Unexpected command: " + name);
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () =>
      root.render(<TrackerWorkspace active onDirtyChange={() => {}} />),
    );
    const link = host.querySelector<HTMLAnchorElement>(".tracker-source-link");
    if (!link) throw new Error("Source link missing");
    expect(link.href).toBe("https:" + "//linkedin.com/jobs/123");
    await act(async () => {
      link.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 1 }));
      await new Promise((resolve) => setTimeout(resolve, 320));
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("open_tracker_link", {
      target: "https:" + "//linkedin.com/jobs/123",
    });
    vi.mocked(invoke).mockClear();
    await act(async () => {
      link.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 1 }));
      link.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 2 }));
      link.dispatchEvent(
        new MouseEvent("dblclick", { bubbles: true, detail: 2 }),
      );
      await new Promise((resolve) => setTimeout(resolve, 320));
    });
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith(
      "open_tracker_link",
      expect.anything(),
    );
    const input = host.querySelector<HTMLInputElement>(
      'input[aria-label="Edit link for Northstar"]',
    );
    if (!input) throw new Error("Link editor missing");
    const setInput = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )?.set;
    if (!setInput) throw new Error("Input setter missing");
    await act(async () => {
      setInput.call(input, "Job board reference 123");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 30));
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith(
      "save_tracker_metadata",
      expect.objectContaining({
        metadata: expect.objectContaining({
          sourceUrl: "Job board reference 123",
        }),
      }),
    );
  } finally {
    await act(async () => root.unmount());
    host.remove();
  }
});

it("expands overflowing links independently without changing their full targets or saving rows", async () => {
  const source = "example.test/jobs/" + "long-path-".repeat(40);
  const records = [
    { id: "one", company: "Northstar", sourceUrl: source },
    { id: "two", company: "Birch", sourceUrl: source + "two" },
    { id: "short", company: "Acme", sourceUrl: "example.test" },
  ].map(({ id, company, sourceUrl }) => ({
    id,
    revision: 1,
    value: { ...emptyTrackerEntry(), company, sourceUrl },
  }));
  const width = vi
    .spyOn(HTMLElement.prototype, "clientWidth", "get")
    .mockReturnValue(206);
  const scrollWidth = vi
    .spyOn(HTMLElement.prototype, "scrollWidth", "get")
    .mockImplementation(function (this: HTMLElement) {
      return this.textContent?.includes("long-path-") ? 1000 : 100;
    });
  vi.mocked(invoke).mockImplementation(async (name) => {
    if (name === "list_tracker_entries")
      return { ok: true, value: records.map(summary) };
    if (name === "open_tracker_link") return { ok: true, value: true };
    throw new Error("Unexpected command: " + name);
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () =>
      root.render(<TrackerWorkspace active onDirtyChange={() => {}} />),
    );
    const toggle = host.querySelector<HTMLButtonElement>(
      '[aria-label="Expand link for Northstar"]',
    )!;
    const link = toggle.parentElement!.querySelector("a")!;
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(
      host.querySelector('[aria-label="Expand link for Acme"]'),
    ).toBeNull();
    expect(link.textContent).toBe(source);
    expect(link.href).toBe("https:" + "//" + source);
    vi.mocked(invoke).mockClear();
    await act(async () => toggle.click());
    expect(toggle.getAttribute("aria-label")).toBe(
      "Compress link for Northstar",
    );
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(
      host
        .querySelector('[aria-label="Expand link for Birch"]')
        ?.getAttribute("aria-expanded"),
    ).toBe("false");
    expect(invoke).not.toHaveBeenCalled();
    await act(async () => link.click());
    expect(invoke).toHaveBeenCalledWith("open_tracker_link", {
      target: "https:" + "//" + source,
    });
    vi.mocked(invoke).mockClear();
    await act(async () => toggle.click());
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(link.href).toBe("https:" + "//" + source);
    expect(invoke).not.toHaveBeenCalled();
    await act(async () => link.click());
    expect(invoke).toHaveBeenCalledWith("open_tracker_link", {
      target: "https:" + "//" + source,
    });
  } finally {
    await act(async () => root.unmount());
    host.remove();
    width.mockRestore();
    scrollWidth.mockRestore();
  }
});
