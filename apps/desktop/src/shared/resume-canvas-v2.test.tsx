// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import { ResumeCanvas } from "./App";
import { createResumeDocument, createSection } from "./resume-editor";

(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => document.body.replaceChildren());

it("adds a valid blank item to a version 2 tailored resume", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  let resume = {
    ...createResumeDocument(),
    schemaVersion: 2,
    contact: { ...createResumeDocument().contact, fullName: "Alex Rivera" },
    sections: [{ ...createSection(0), heading: "Experience" }],
  };
  await act(async () =>
    root.render(
      <ResumeCanvas
        document={resume}
        style="technical"
        contactDivider="dot"
        onContactDividerChange={() => undefined}
        disabled={false}
        canAddEntry
        onChange={(update) => {
          resume = update(resume);
        }}
      />,
    ),
  );
  await act(async () =>
    host.querySelector<HTMLButtonElement>(".canvas-add-entry")!.click(),
  );
  expect(resume.sections[0].entries).toHaveLength(1);
  expect(resume.sections[0].entries[0].dates).toEqual([]);
  await act(async () => root.unmount());
});
