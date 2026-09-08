import { expect, it } from "vitest";
import { duplicateEntryGroups } from "../src/shared/duplicate-hints";
import { duplicateEntry } from "../src/shared/duplicate-entry";
import {
  createEntry,
  createSection,
  createResumeDocument,
} from "../src/shared/resume-editor";

it("finds repeated completed content across sections without changing the document", () => {
  const document = createResumeDocument();
  const section = createSection(0);
  const first = createEntry(0);
  first.heading = "Synthetic role";
  const copy = duplicateEntry(first);
  const other = createSection(1);
  section.entries = [first];
  other.entries = [copy];
  document.sections = [section, other];
  const before = JSON.stringify(document);
  expect(duplicateEntryGroups(document)[0].map((x) => x.entryId)).toEqual([
    first.id,
    copy.id,
  ]);
  expect(JSON.stringify(document)).toBe(before);
  copy.subheading = "Different organization";
  expect(duplicateEntryGroups(document)).toEqual([]);
});

it("ignores blank entries and preserves date precision in comparison", () => {
  const document = createResumeDocument();
  const section = createSection(0);
  section.entries = [createEntry(0), createEntry(1)];
  document.sections = [section];
  expect(duplicateEntryGroups(document)).toEqual([]);
  section.entries[0].heading = section.entries[1].heading = "Synthetic role";
  section.entries[0].dateRange = "2020";
  section.entries[1].dateRange = "June 2020";
  expect(duplicateEntryGroups(document)).toEqual([]);
});
