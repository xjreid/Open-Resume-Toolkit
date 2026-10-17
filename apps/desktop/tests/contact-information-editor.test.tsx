// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { expect, it } from "vitest";
import type { ContactDetails } from "@ort/contracts/resume";
import { ContactInformationEditor } from "../src/shared/ContactInformationEditor";

(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

it("shows imported profile links at the top and edits labels and addresses immediately without losing identity", async () => {
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
        disabled={false}
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
      'input[aria-label="Contact link 1 address"]',
    )!;
    expect(input.value).toBe("https://github.com/example");
    const change = async (value: string) =>
      act(async () => {
        Object.getOwnPropertyDescriptor(
          HTMLInputElement.prototype,
          "value",
        )!.set!.call(input, value);
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
    await change("javascript:alert(1)");
    expect(input.getAttribute("aria-invalid")).toBe("true");
    expect(contact.links[0].url).toBe("javascript:alert(1)");
    expect(summary.querySelector('a[href^="javascript:"]')).toBeNull();
    await change("https://github.com/example/new");
    const label = host.querySelector<HTMLInputElement>(
      'input[aria-label="Contact link 1 label"]',
    )!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(
        HTMLInputElement.prototype,
        "value",
      )!.set!.call(label, "Source");
      label.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(contact.links[0]).toMatchObject({
      id: "01900000-0000-7000-8000-000000000001",
      order: 0,
      label: "Source",
      url: "https://github.com/example/new",
    });
    expect(contact.location).toBe("Boston");
    const divider = host.querySelector("select")!;
    await act(async () => {
      divider.value = "bar";
      divider.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(contact.divider).toBe("bar");
    expect(summary.textContent).toContain(" | ");
    await act(async () => {
      divider.value = "dot";
      divider.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(contact).not.toHaveProperty("divider");
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
