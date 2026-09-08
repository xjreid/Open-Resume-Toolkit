import { expect, it } from "vitest";
import { dateText, reversedDate } from "./resume-dates";
import {
  createEntityId,
  createResumeDocument,
  createSection,
  createEntry,
  normalizeDocument,
  upgradeDocumentV2,
} from "./resume-editor";
import type { ResumeDate } from "@ort/contracts/resume";
import { validateEditorDocument } from "./resume-validation";

it("preserves date precision, expected dates, open ranges and nonblocking reversed ranges", () => {
  const date: ResumeDate = {
    id: createEntityId(),
    order: 0,
    label: "Graduation",
    start: { year: 2027, month: null, expected: true },
    end: null,
  };
  expect(dateText(date)).toBe("Graduation: Expected 2027");
  expect(dateText({ ...date, start: null })).toBe("");
  const range: ResumeDate = {
    ...date,
    label: "",
    start: { year: 2020, month: 9, expected: false },
    end: { kind: "present" },
  };
  expect(dateText(range)).toBe("Sep 2020–Present");
  range.end = {
    kind: "date",
    value: { year: 2020, month: null, expected: false },
  };
  expect(reversedDate(range)).toBe(false);
  range.end.value.year = 2019;
  expect(reversedDate(range)).toBe(true);
  const document = upgradeDocumentV2(createResumeDocument());
  const section = createSection(0);
  const entry = { ...createEntry(0), dates: [range] };
  section.entries.push(entry);
  document.sections.push(section);
  expect(validateEditorDocument(document)).toEqual([]);
  entry.dates[0].start!.month = 13;
  expect(validateEditorDocument(document)[0].path).toBe(`date.${date.id}`);
});

it("upgrades an edited copy without changing legacy dates or regenerating stable link IDs", () => {
  const original = createResumeDocument();
  original.contact.links = [
    { label: "First", url: "mailto:first@example.org" },
    { label: "Second", url: "mailto:second@example.org" },
  ];
  const section = createSection(0);
  const entry = createEntry(0);
  entry.dateRange = "Circa 2020 / ongoing";
  section.entries.push(entry);
  original.sections.push(section);
  const serialized = JSON.stringify(original);
  const upgraded = upgradeDocumentV2(original);
  expect(JSON.stringify(original)).toBe(serialized);
  expect(upgraded.sections[0].entries[0].dateRange).toBe(entry.dateRange);
  expect(upgraded.sections[0].entries[0].dates).toEqual([]);
  expect(upgradeDocumentV2(upgraded)).toEqual(upgraded);
  const ids = upgraded.contact.links.map((link) => link.id);
  upgraded.contact.links.reverse();
  upgraded.contact.links[0].label = "Renamed";
  const reordered = normalizeDocument(upgraded);
  expect(reordered.contact.links.map((link) => link.id)).toEqual(ids.reverse());
  expect(reordered.contact.links.map((link) => link.order)).toEqual([0, 1]);
});
