// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { ContactInformationEditor } from "../src/shared/ContactInformationEditor";
import { PublishedResume } from "../src/shared/PublishedResume";
import { useResumeSession } from "../src/shared/use-resume-session";
import { closeDisposition } from "../src/shared/close-policy";
import {
  createResumeDocument,
  upgradeDocumentV2,
} from "../src/shared/resume-editor";
import type { ResumeDocument } from "@ort/contracts/resume";

const native = vi.hoisted(() => ({
  health: vi.fn(),
  workspace: vi.fn(),
  save: vi.fn(),
  publish: vi.fn(),
  export: vi.fn(),
}));
vi.mock("../src/shared/command-client", () => ({
  requestHealth: native.health,
  requestResumeWorkspace: native.workspace,
  saveResume: native.save,
  publishResume: native.publish,
  exportResumeDocument: native.export,
  exportResumePdf: vi.fn(),
  renderResumePdf: vi.fn(),
  releaseResumePdf: vi.fn(),
}));
vi.mock("../src/shared/use-close-guard", () => ({
  useCloseGuard: () => ({ pending: false }),
}));

it("autosaves focused contact edits, guards invalid drafts, and restores saved dividers", async () => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.useFakeTimers();
  const resume = upgradeDocumentV2(createResumeDocument());
  resume.contact.links = [
    {
      id: "01900000-0000-7000-8000-000000000001",
      order: 0,
      label: "Portfolio",
      url: "https://example.com/old",
    },
  ];
  let saved = { revision: 1, document: resume };
  native.health.mockResolvedValue({
    ok: true,
    value: { storageStatus: "ready" },
  });
  native.workspace.mockImplementation(async () => ({
    ok: true,
    value: {
      profileId: "01900000-0000-7000-8000-000000000002",
      draft: saved,
      latestPublished: null,
    },
  }));
  native.save.mockImplementation(
    async (_profile: string, revision: number, document: ResumeDocument) => {
      saved = {
        revision: revision + 1,
        document: JSON.parse(JSON.stringify(document)),
      };
      return { ok: true, value: saved };
    },
  );
  let session!: ReturnType<typeof useResumeSession>;
  function Harness() {
    session = useResumeSession({
      importActive: false,
      importWorking: false,
      trackerDirty: false,
      confirmReload: false,
      documentStyle: "technical",
      exportSource: "saved_draft",
    });
    const document = session.editor.document;
    return (
      document && (
        <>
          <ContactInformationEditor
            contact={document.contact}
            disabled={false}
            onChange={(contact) =>
              session.dispatch({
                type: "edit",
                update: (current) => ({ ...current, contact }),
              })
            }
          />
          <PublishedResume document={document} />
        </>
      )
    );
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () => root.render(<Harness />));
    const input = host.querySelector<HTMLInputElement>(
      'input[aria-label="Contact link 1 address"]',
    )!;
    async function change(value: string) {
      await act(async () => {
        input.focus();
        Object.getOwnPropertyDescriptor(
          HTMLInputElement.prototype,
          "value",
        )!.set!.call(input, value);
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
    }
    await change("https://example.com/new");
    expect(closeDisposition(session.editor)).toBe("confirm");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200);
    });
    expect(document.activeElement).toBe(input);
    expect(saved.document.contact.links[0].url).toBe("https://example.com/new");
    expect(closeDisposition(session.editor)).toBe("quit");
    native.save.mockClear();

    for (const value of [
      "https://",
      "javascript:alert(1)",
      "https://example.com/" + "a".repeat(2000),
    ]) {
      await change(value);
      expect(closeDisposition(session.editor)).toBe("confirm");
      expect(input.getAttribute("aria-invalid")).toBe("true");
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1500);
        await session.save();
        await session.publish();
        await session.exportSelected("docx");
      });
      expect(native.save).not.toHaveBeenCalled();
      expect(native.publish).not.toHaveBeenCalled();
      expect(native.export).not.toHaveBeenCalled();
      expect(saved.document.contact.links[0].url).toBe(
        "https://example.com/new",
      );
      await act(async () => session.dispatch({ type: "undo" }));
      expect(input.value).toBe("https://example.com/new");
      expect(closeDisposition(session.editor)).toBe("quit");
    }
    const divider = host.querySelector("select")!;
    await act(async () => {
      divider.value = "dash";
      divider.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(closeDisposition(session.editor)).toBe("confirm");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1200);
    });
    expect(saved.document.contact.divider).toBe("dash");
    await act(async () => session.loadWorkspace());
    expect(host.querySelector("select")!.value).toBe("dash");
    expect(
      host.querySelector(".resume-document__contact--dash"),
    ).not.toBeNull();
  } finally {
    await act(async () => root.unmount());
    host.remove();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  }
});
