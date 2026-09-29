// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import { PopupSectionNavigator } from "./PopupSectionNavigator";
import { createResumeDocument, createSection } from "./resume-editor";

(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => document.body.replaceChildren());

it("adds, renames, reorders, and confirms removal of sections", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const first = { ...createSection(0), heading: "Education" };
  const second = { ...createSection(1), heading: "Experience" };
  let latest = { ...createResumeDocument(), sections: [first, second] };
  function Harness() {
    const [document, setDocument] = useState(latest);
    return (
      <PopupSectionNavigator
        document={document}
        disabled={false}
        onChange={(next) => {
          latest = next;
          setDocument(next);
        }}
      />
    );
  }
  await act(async () => root.render(<Harness />));
  const nav = host.querySelector("nav")!;
  const cards = () => [
    ...nav.querySelectorAll<HTMLElement>(".section-nav-card"),
  ];
  const pointer = (
    element: EventTarget,
    type: string,
    x: number,
    y: number,
  ) => {
    const event = new MouseEvent(type, {
      bubbles: true,
      button: 0,
      clientX: x,
      clientY: y,
    });
    Object.defineProperty(event, "pointerId", { value: 1 });
    element.dispatchEvent(event);
  };
  await act(async () => {
    pointer(cards()[0], "pointerdown", 10, 10);
    pointer(window, "pointerup", 10, 10);
  });
  const name = nav.querySelector<HTMLInputElement>(
    `input[aria-label="Section name Education"]`,
  )!;
  expect(name).not.toBeNull();
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )?.set?.call(name, "Training");
    name.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(latest.sections[0].heading).toBe("Training");
  await act(async () => name.blur());

  Object.defineProperty(nav, "getBoundingClientRect", {
    value: () => ({ left: 0, right: 218, top: 0, bottom: 500 }),
  });
  const list = nav.querySelector<HTMLElement>(".section-sort-list")!;
  Object.defineProperty(list, "getBoundingClientRect", {
    value: () => ({ left: 0, right: 218, top: 0, bottom: 250 }),
  });
  Object.defineProperty(cards()[0], "getBoundingClientRect", {
    value: () => ({
      left: 0,
      right: 200,
      top: 0,
      bottom: 40,
      width: 200,
      height: 40,
    }),
  });
  Object.defineProperty(cards()[1], "getBoundingClientRect", {
    value: () => ({
      left: 0,
      right: 200,
      top: 50,
      bottom: 90,
      width: 200,
      height: 40,
    }),
  });
  await act(async () => {
    pointer(cards()[0], "pointerdown", 10, 10);
    pointer(window, "pointermove", 10, 80);
    pointer(window, "pointerup", 10, 80);
  });
  expect(latest.sections.map((section) => section.heading)).toEqual([
    "Experience",
    "Training",
  ]);
  expect(latest.sections.map((section) => section.order)).toEqual([0, 1]);

  const trash = nav.querySelector<HTMLElement>(".section-trash")!;
  Object.defineProperty(trash, "getBoundingClientRect", {
    value: () => ({ left: 0, right: 218, top: 100, bottom: 150 }),
  });
  await act(async () => {
    pointer(cards()[0], "pointerdown", 10, 10);
    pointer(window, "pointermove", 10, 120);
    pointer(window, "pointerup", 10, 120);
  });
  expect(latest.sections).toHaveLength(2);
  await act(async () =>
    nav
      .querySelector<HTMLButtonElement>(
        ".section-trash-confirmation .button--danger",
      )!
      .click(),
  );
  expect(latest.sections.map((section) => section.heading)).toEqual([
    "Training",
  ]);
  await act(async () =>
    nav
      .querySelector<HTMLButtonElement>(".section-add-control button")!
      .click(),
  );
  expect(latest.sections).toHaveLength(2);
  expect(latest.sections[1].heading).toBe("Custom Section");
  await act(async () => root.unmount());
});
