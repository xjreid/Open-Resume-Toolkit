import { expect, it } from "vitest";
import { duplicateEntry } from "../src/shared/duplicate-entry";
import { createEntry, createEntityId } from "../src/shared/resume-editor";

it("duplicates factual content with independent identities and date values", () => {
  const original = createEntry(0);
  original.heading = "Synthetic engineer";
  original.fields = [
    {
      id: createEntityId(),
      order: 0,
      label: "Skill",
      value: "Testing",
      isSkill: true,
    },
  ];
  original.bullets = [
    { id: createEntityId(), order: 0, text: "Synthetic achievement" },
  ];
  original.links = [
    {
      id: createEntityId(),
      order: 0,
      label: "Portfolio",
      url: "https://example.com",
    },
  ];
  original.dates = [
    {
      id: createEntityId(),
      order: 0,
      label: "Employment",
      start: { year: 2020, month: null, expected: false },
      end: { kind: "date", value: { year: 2027, month: 6, expected: true } },
    },
  ];
  const snapshot = structuredClone(original);
  const copied = duplicateEntry(original);
  const ids = (entry: typeof original) => [
    entry.id,
    ...entry.fields.map((x) => x.id),
    ...entry.bullets.map((x) => x.id),
    ...entry.links.map((x) => x.id),
    ...entry.dates!.map((x) => x.id),
  ];
  expect(new Set([...ids(original), ...ids(copied)]).size).toBe(10);
  expect(
    JSON.stringify(copied, (key, value) => (key === "id" ? undefined : value)),
  ).toBe(
    JSON.stringify(original, (key, value) =>
      key === "id" ? undefined : value,
    ),
  );
  copied.fields[0].value = "Changed";
  copied.bullets[0].text = "Changed";
  copied.links[0].label = "Changed";
  copied.dates![0].start!.year = 2021;
  if (copied.dates![0].end?.kind === "date")
    copied.dates![0].end.value.year = 2028;
  expect(original).toEqual(snapshot);
});

it("preserves legacy entry shape without silently upgrading dates or links", () => {
  const original = createEntry(0);
  original.dateRange = "Summer term";
  original.links = [{ label: "Portfolio", url: "https://example.com" }];
  const copied = duplicateEntry(original);
  expect(copied).not.toHaveProperty("dates");
  expect(copied.links[0]).not.toHaveProperty("id");
  expect(copied.links[0]).not.toHaveProperty("order");
  expect(copied.dateRange).toBe("Summer term");
});
