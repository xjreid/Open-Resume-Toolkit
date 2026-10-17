// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { expect, it } from "vitest";
import type { ContactDetails } from "@ort/contracts/resume";
import { ContactInformationEditor } from "../src/shared/ContactInformationEditor";

(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

it("shows imported profile links at the top and edits their normal link notation without losing identity", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  let contact: ContactDetails = {
    fullName: "Example Person",
    email: "person@example.com",
    phone: "",
    location: "Boston",
    links: [
      {
        id: "01900000-0000-7000-8000-000000000001",
        order: 0,
        label: "GitHub",
        url: "https://github.com/example",
      },
      {
        id: "01900000-0000-7000-8000-000000000002",
        order: 1,
        label: "LinkedIn",
        url: "https://linkedin.com/in/example",
      },
    ],
  };
  function Harness() {
    const [current, setCurrent] = useState(contact);
    return (
      <ContactInformationEditor
        contact={current}
        divider="dot"
        disabled={false}
        onDividerChange={() => {}}
        onChange={(next) => {
          contact = next;
          setCurrent(next);
        }}
      />
    );
  }
  try {
    await act(async () => root.render(<Harness />));
    const summary = host.querySelector("summary")!;
    expect(
      summary.querySelector('a[href="https://github.com/example"]')
        ?.textContent,
    ).toBe("GitHub");
    expect(
      summary.querySelector('a[href="https://linkedin.com/in/example"]')
        ?.textContent,
    ).toBe("LinkedIn");
    const input = host.querySelector<HTMLInputElement>(
      'input[aria-label="Contact link 1"]',
    )!;
    expect(input.value).toBe("[GitHub](https://github.com/example)");
    const change = async (value: string) =>
      act(async () => {
        Object.getOwnPropertyDescriptor(
          HTMLInputElement.prototype,
          "value",
        )!.set!.call(input, value);
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
    const commit = async () =>
      act(async () => {
        input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      });
    await change("[GitHub](javascript:alert(1))");
    await commit();
    expect(input.getAttribute("aria-invalid")).toBe("true");
    expect(contact.links[0].url).toBe("https://github.com/example");
    await change("[Source](https://github.com/example/new)");
    await commit();
    expect(contact.links[0]).toMatchObject({
      id: "01900000-0000-7000-8000-000000000001",
      order: 0,
      label: "Source",
      url: "https://github.com/example/new",
    });
    expect(contact.location).toBe("Boston");
    await act(async () =>
      host
        .querySelector<HTMLButtonElement>(
          '[aria-label="Delete contact link 1"]',
        )!
        .click(),
    );
    expect(contact.links).toHaveLength(1);
    expect(contact.links[0]).toMatchObject({
      id: "01900000-0000-7000-8000-000000000002",
      order: 0,
    });
  } finally {
    await act(async () => root.unmount());
    host.remove();
  }
});
