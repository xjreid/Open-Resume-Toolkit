// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it } from "vitest";
import { PARAGRAPH_FIELD_LABEL } from "./resume-fields";
import { ResumeCanvas } from "./ResumeCanvas";
import { PublishedResume } from "./PublishedResume";
import {
  createResumeDocument,
  createSection,
  createEntry,
  createNamedField,
} from "./resume-editor";
(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;
function sample() {
  const document = { ...createResumeDocument(), schemaVersion: 2 };
  const entry = createEntry(0, 2);
  entry.heading = "Synthetic position";
  entry.fields = [
    { ...createNamedField(0), label: "Language", value: "Rust" },
    {
      ...createNamedField(1),
      label: "Certification",
      value: "Preserve this certification",
    },
  ];
  document.sections = [
    { ...createSection(0), heading: "Experience", entries: [entry] },
  ];
  return document;
}
it("editing Extra preserves every ordinary named field", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  let resume = sample();
  await act(async () =>
    root.render(
      <ResumeCanvas
        document={resume}
        style="technical"
        contactDivider="dot"
        onContactDividerChange={() => {}}
        disabled={false}
        canAddEntry
        onChange={(update) => {
          resume = update(resume);
        }}
      />,
    ),
  );
  const button = [...host.querySelectorAll<HTMLButtonElement>("button")].find(
    (button) =>
      button.querySelector(".canvas-field__label")?.textContent === "Extra",
  )!;
  await act(async () => button.click());
  const input = host.querySelector<HTMLInputElement>(
    'input[aria-label="Extra"]',
  )!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )!.set!.call(input, "New extra");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  const fields = resume.sections[0].entries[0].fields;
  expect(fields.find((f) => f.label === "Extra")?.value).toBe("New extra");
  expect(fields.some((f) => f.value === "Preserve this certification")).toBe(
    true,
  );
  await act(async () => root.unmount());
  host.remove();
});
it("PublishedResume renders every ordinary named field", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<PublishedResume document={sample()} />));
  expect(host.textContent).toContain("Rust");
  expect(host.textContent).toContain("Preserve this certification");
  await act(async () => root.unmount());
  host.remove();
});

it("renders all paragraphs alongside retained bullets", async () => {
  const document = sample();
  const entry = document.sections[0].entries[0];
  entry.fields.push(
    ...["Paragraph one", "Paragraph two"].map((value, index) => ({
      ...createNamedField(index + 2),
      label: PARAGRAPH_FIELD_LABEL,
      value,
    })),
  );
  entry.bullets = [
    { id: crypto.randomUUID(), order: 0, text: "Retained bullet" },
  ];
  const host = window.document.createElement("div");
  const root = createRoot(host);
  try {
    await act(async () => root.render(<PublishedResume document={document} />));
    for (const value of [
      "Paragraph one",
      "Paragraph two",
      "Retained bullet",
      "Preserve this certification",
    ])
      expect(host.textContent).toContain(value);
  } finally {
    await act(async () => root.unmount());
  }
});

it("classifies a list without altering its value, identity or other metadata", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  let resume = sample();
  const original = { ...resume.sections[0].entries[0].fields[0] };
  await act(async () =>
    root.render(
      <ResumeCanvas
        document={resume}
        style="technical"
        contactDivider="dot"
        onContactDividerChange={() => {}}
        disabled={false}
        canAddEntry
        onChange={(update) => {
          resume = update(resume);
        }}
      />,
    ),
  );
  const select = host.querySelector<HTMLSelectElement>(
    'select[aria-label="List type for Language 1"]',
  )!;
  await act(async () => {
    select.value = "coursework";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(resume.sections[0].entries[0].fields[0]).toEqual({
    ...original,
    listKind: "coursework",
    isSkill: false,
  });
  expect(resume.sections[0].entries[0].fields[1].value).toBe(
    "Preserve this certification",
  );
  await act(async () => root.unmount());
  host.remove();
});
