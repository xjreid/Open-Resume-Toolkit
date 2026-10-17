// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { DocumentImport } from "../src/shared/DocumentImport";
import { aiImportStatus } from "../src/shared/ai-import";
import { beginAiDocumentImport } from "../src/shared/import-client";
import {
  createResumeDocument,
  upgradeDocumentV2,
} from "../src/shared/resume-editor";

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
const review = vi.hoisted(() => ({ render: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../src/shared/ImportReviewFlow", () => ({
  ImportReviewFlow: (props: { onCancelled: () => void }) => {
    review.render(props);
    return <button onClick={props.onCancelled}>Cancel shared review</button>;
  },
}));

const ready = {
  ready: true,
  busy: false,
  provider: "openai",
  model: "sample-model",
  reasoning: null,
  issue: null,
};
const snapshot = () => ({
  id: "01900000-0000-7000-8000-000000000001",
  baseRevision: 3,
  mappingVersion: 1,
  importedDocument: upgradeDocumentV2(createResumeDocument()),
  blocks: [],
  sections: [],
  contacts: { fullName: "", email: "", phone: "", location: "" },
});
let status = { ...ready } as typeof ready & {
  reasoning: string | null;
  issue: string | null;
};
let result: unknown;
let host: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
const saved = vi.fn();
const busy = vi.fn();
const operation = vi.fn();
const button = (text: string) =>
  Array.from(host.querySelectorAll("button")).find(
    (button) => button.textContent === text,
  )!;
const render = (active = true) =>
  act(async () =>
    root.render(
      <DocumentImport
        active={active}
        disabled={false}
        revision={3}
        onSaved={saved}
        onBusyChange={busy}
        onOperationChange={operation}
      />,
    ),
  );

beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.clearAllMocks();
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  status = { ...ready };
  result = { ok: true, value: null };
  native.invoke.mockImplementation(async (command: string) => {
    if (command === "ai_import_status") return { ok: true, value: status };
    if (command === "document_import_available") return false;
    if (command === "begin_ai_document_import") return result;
    if (command === "cancel_document_import") return { ok: true, value: true };
    throw new Error("Unexpected command");
  });
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
});

it("requires a ready provider, shows Codex model and reasoning, and refreshes when the page opens", async () => {
  status = {
    ...ready,
    ready: false,
    provider: "",
    model: "",
    issue: "AI_DISABLED",
  };
  await render();
  expect(button("Choose PDF").disabled).toBe(true);
  expect(button("Check connection")).toBeUndefined();
  expect(host.textContent).toContain(
    "Choose a provider and model in AI & monitoring",
  );
  status = {
    ...ready,
    ready: true,
    provider: "codex",
    reasoning: "high",
    issue: null,
  };
  await act(async () => window.dispatchEvent(new Event("focus")));
  expect(host.textContent).toContain("Codex");
  expect(host.textContent).toContain("sample-model");
  expect(host.textContent).toContain("high");
  expect(host.textContent).toContain(
    "Pages are sent to Codex using your ChatGPT plan",
  );
  expect(button("Choose PDF").disabled).toBe(false);
  status = { ...status, ready: false, issue: "AI_MODEL_IMAGE_UNSUPPORTED" };
  await act(async () => window.dispatchEvent(new Event("focus")));
  expect(host.textContent).toContain("This model cannot read PDFs");
  expect(button("Choose PDF").disabled).toBe(true);
  await render(false);
  status = { ...ready };
  await render(true);
  expect(button("Choose PDF").disabled).toBe(false);
});

it("opens the exact shared review for AI output even when the local parser is unavailable, without saving", async () => {
  const value = snapshot();
  result = { ok: true, value };
  await render();
  expect(button("Choose file").disabled).toBe(true);
  await act(async () => button("Choose PDF").click());
  expect(native.invoke).toHaveBeenCalledWith("begin_ai_document_import", {
    request: expect.objectContaining({ payload: { expectedRevision: 3 } }),
  });
  expect(review.render).toHaveBeenLastCalledWith(
    expect.objectContaining({ reviewId: value.id, initialSnapshot: value }),
  );
  expect(saved).not.toHaveBeenCalled();
  expect(busy).toHaveBeenLastCalledWith(true);
  await act(async () => button("Cancel shared review").click());
  expect(busy).toHaveBeenLastCalledWith(false);
  expect(document.activeElement).toBe(button("Choose PDF"));
});

it.each([
  ["AI_PROVIDER_TIMEOUT", "AI import timed out"],
  ["AI_PROVIDER_BAD_REQUEST", "could not accept this document"],
  ["AI_OUTPUT_INVALID", "AI returned an invalid resume"],
  ["AI_PROVIDER_UNAVAILABLE", "connection failed"],
  ["AI_PDF_RENDER_FAILED", "This PDF could not be read"],
  ["AI_PDF_PAGE_LIMIT", "exceeds the page limit"],
  ["PLAN_REQUEST_TIMEOUT", "AI import timed out"],
  ["UNEXPECTED_PROVIDER_DIAGNOSTIC", "Import failed; try again"],
])(
  "shows a concise %s error without exposing diagnostics or entering review",
  async (code, message) => {
    result = {
      ok: false,
      error: {
        code,
        messageKey: "errors.aiImportFailed",
        retryable: false,
        details: {
          provider: "openai",
          model: "sample-model",
          diagnostic: {
            code,
            httpStatus: 400,
            validationIssues: ["Missing required entry slot."],
          },
        },
      },
    };
    await render();
    await act(async () => button("Choose PDF").click());
    const error = host.querySelector('[role="alert"]')!;
    expect(error.textContent).toContain(message);
    expect(error.textContent).not.toContain(code);
    expect(error.textContent).not.toContain("400");
    expect(error.textContent).not.toContain("Missing required entry slot.");
    expect(error.querySelector("details")).toBeNull();
    expect(review.render).not.toHaveBeenCalled();
    expect(saved).not.toHaveBeenCalled();
    expect(busy).toHaveBeenLastCalledWith(false);
  },
);

it("handles picker cancellation and blocks duplicate launches while a request is pending", async () => {
  await render();
  await act(async () => button("Choose PDF").click());
  expect(busy).toHaveBeenLastCalledWith(false);
  let finish!: (value: unknown) => void;
  result = new Promise((resolve) => {
    finish = resolve;
  });
  await act(async () => {
    button("Choose PDF").click();
    button("Choose PDF").click();
  });
  expect(
    native.invoke.mock.calls.filter(
      ([command]) => command === "begin_ai_document_import",
    ),
  ).toHaveLength(2);
  expect(host.textContent).toContain("Reading your PDF with AI");
  await act(async () => button("Cancel import").click());
  expect(native.invoke).toHaveBeenCalledWith("cancel_document_import");
  await act(async () => finish({ ok: true, value: snapshot() }));
  expect(review.render).toHaveBeenCalled();
  expect(saved).not.toHaveBeenCalled();
});

it("rejects malformed native readiness and import replies instead of enabling import or entering review", async () => {
  native.invoke.mockResolvedValue({ ok: true, value: { ready: true } });
  expect((await aiImportStatus()).ok).toBe(false);
  expect((await beginAiDocumentImport(3)).ok).toBe(false);
  await render();
  expect(button("Choose PDF").disabled).toBe(true);
  expect(host.querySelector('[role="alert"]')?.textContent).toContain(
    "readiness could not be checked",
  );
});
