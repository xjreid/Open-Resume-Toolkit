// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import axe from "axe-core";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ImportReviewPanel } from "../src/shared/ImportReviewPanel";
import {
  initialReview,
  reviewProblem,
  type ReviewSession,
} from "../src/shared/import-review";

const session: ReviewSession = {
  id: "synthetic-review",
  baseRevision: 4,
  contacts: { fullName: "Existing name", email: "", phone: "", location: "" },
  sections: [{ id: "experience", heading: "Experience" }],
  blocks: [
    {
      source: "<script>synthetic original</script>",
      page: 1,
      explanation: "Unclassified text; choose a destination.",
      suggestedTarget: "text",
      suggestedValue: "Synthetic reviewed value",
    },
  ],
};
let root: Root;
let host: HTMLDivElement;
const submit = vi.fn();
const cancel = vi.fn();
beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  submit.mockClear();
  cancel.mockClear();
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.unstubAllGlobals();
});
async function render(review = session, revision = 4, busy = false) {
  await act(async () =>
    root.render(
      <main>
        <h1>Resume</h1>
        <ImportReviewPanel
          session={review}
          currentRevision={revision}
          busy={busy}
          onCancel={cancel}
          onSubmit={submit}
        />
      </main>,
    ),
  );
}
async function select(label: string, value: string) {
  const node = [...host.querySelectorAll("label")].find(
    (item) => item.textContent === label,
  )!;
  const input = document.getElementById(node.htmlFor) as HTMLSelectElement;
  await act(async () => {
    input.value = value;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
}
function applyButton() {
  return host.querySelector('button[type="submit"]') as HTMLButtonElement;
}
it("requires explicit decisions and destinations, preserves original text and submits copies", async () => {
  await render();
  expect(applyButton().disabled).toBe(true);
  expect(host.querySelector("script")).toBeNull();
  expect(host.querySelector("pre")?.textContent).toBe(session.blocks[0].source);
  await select("Decision", "accept");
  expect(applyButton().disabled).toBe(true);
  await select("Destination section", "existing:experience");
  expect(applyButton().disabled).toBe(false);
  await act(async () => applyButton().click());
  expect(submit).toHaveBeenCalledWith({
    choices: [
      {
        kind: "text",
        text: "Synthetic reviewed value",
        bullet: false,
        target: { kind: "existing", id: "experience" },
      },
    ],
  });
  await select("Decision", "reject");
  expect(applyButton().disabled).toBe(true);
  expect(submit.mock.calls[0][0].choices[0].kind).toBe("text");
  expect(host.querySelector("pre")?.textContent).toBe(session.blocks[0].source);
});
it("requires an explicit contact-conflict choice and blocks stale or busy submission", async () => {
  await render();
  await select("Decision", "accept");
  await select("Use as", "fullName");
  expect(applyButton().disabled).toBe(true);
  await select("Existing contact information", "keepExisting");
  expect(applyButton().disabled).toBe(false);
  await render(session, 5);
  expect(applyButton().disabled).toBe(true);
  await act(async () =>
    host
      .querySelector("form")!
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })),
  );
  expect(submit).not.toHaveBeenCalled();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain(
    "draft changed",
  );
  await render(session, 4, true);
  expect(applyButton().disabled).toBe(true);
});
it("resets decisions for a new session and allows cancellation without submission", async () => {
  await render();
  await select("Decision", "reject");
  await render({ ...session, id: "new-session" });
  expect(host.querySelector("select")?.value).toBe("pending");
  await act(async () =>
    (host.querySelector('button[type="button"]') as HTMLButtonElement).click(),
  );
  expect(cancel).toHaveBeenCalledOnce();
  expect(submit).not.toHaveBeenCalled();
});
it("labels review controls for accessibility", async () => {
  await render();
  const result = await axe.run(host, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(result.violations.map((item) => item.id)).toEqual([]);
});
it("rejects absent destinations and blank kept content without changing proposals", () => {
  const original = JSON.stringify(session);
  const [choice] = initialReview(session);
  expect(choice.status).toBe("pending");
  choice.status = "accept";
  choice.sectionId = "absent";
  expect(reviewProblem(session, choice)).toContain("available section");
  choice.value = " ";
  expect(reviewProblem(session, choice)).toContain("Enter a value");
  choice.status = "reject";
  expect(reviewProblem(session, choice)).toBeNull();
  expect(JSON.stringify(session)).toBe(original);
});

it("invalidates a destination when its proposed section is rejected", () => {
  const [text] = initialReview(session);
  const heading = {
    ...text,
    status: "accept" as const,
    target: "section" as const,
    value: "Projects",
  };
  text.status = "accept";
  text.proposedSection = 1;
  expect(reviewProblem(session, text, [text, heading])).toBeNull();
  expect(
    reviewProblem(session, text, [text, { ...heading, status: "reject" }]),
  ).toContain("still kept");
});

it("serializes the shared native decision fixture without review source or ownership", async () => {
  const { importChoices } = await import("../src/shared/import-review");
  const { default: expected } = await import(
    "../../../fixtures/documents/import-review-choices.json"
  );
  const [initial] = initialReview(session);
  const choices = [
    {
      ...initial,
      status: "accept" as const,
      target: "section" as const,
      value: "Projects",
    },
    {
      ...initial,
      status: "accept" as const,
      target: "bullet" as const,
      value: "Synthetic contribution",
      proposedSection: 0,
    },
    {
      ...initial,
      status: "accept" as const,
      target: "fullName" as const,
      value: "Synthetic Person",
      contactMode: "replace" as const,
    },
  ];
  expect(importChoices(choices)).toEqual(expected);
  expect(importChoices([{ ...initial, status: "reject" }, initial])).toEqual({
    choices: [{ kind: "reject" }, { kind: "pending" }],
  });
});
