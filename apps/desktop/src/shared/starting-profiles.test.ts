import { describe, expect, it } from "vitest";
import { STARTING_PROFILES, createStartingSections } from "./starting-profiles";
import { createResumeDocument } from "./resume-editor";
import { validateEditorDocument } from "./resume-validation";

describe("starting profile suggestions", () => {
  it.each(STARTING_PROFILES)(
    "creates valid empty suggestions for $label",
    ({ id }) => {
      const document = createResumeDocument();
      document.sections = createStartingSections(id);
      expect(validateEditorDocument(document)).toEqual([]);
      expect(
        document.sections.every(
          (section, index) =>
            section.order === index && section.entries.length === 0,
        ),
      ).toBe(true);
      expect(new Set(document.sections.map((section) => section.id)).size).toBe(
        document.sections.length,
      );
      const again = createStartingSections(id);
      expect(
        again.every(
          (section) =>
            !document.sections.some((existing) => existing.id === section.id),
        ),
      ).toBe(true);
      expect(document.contact.fullName).toBe("");
    },
  );
});
