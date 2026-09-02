import { describe, expect, it } from "vitest";
import { DOCUMENT_LIMITS } from "@ort/contracts/resume";
import {
  createEntry,
  createResumeDocument,
  createSection,
  moveItem,
  normalizeDocument,
} from "../src/shared/resume-editor";
import { validateEditorDocument } from "../src/shared/resume-validation";

describe("editor validation and ordering", () => {
  it("requires titles and section headings, and rejects executable links", () => {
    const document = createResumeDocument();
    document.title = " ";
    document.sections = [{ ...createSection(0), heading: "" }];
    document.contact.links = [{ label: "Bad", url: "javascript:alert(1)" }];
    const issues = validateEditorDocument(document);
    expect(issues).toHaveLength(3);
    expect(issues.some((issue) => issue.path === "contact.links.0.url")).toBe(
      true,
    );
  });

  it("accepts safe links and counts Unicode characters like Rust", () => {
    const document = createResumeDocument();
    document.title = "😀".repeat(DOCUMENT_LIMITS.fieldCharacters);
    document.contact.links = [
      { label: "Site", url: "https://example.com" },
      { label: "Email", url: "mailto:test@example.invalid" },
    ];
    expect(validateEditorDocument(document)).toEqual([]);
    document.title += "😀";
    expect(validateEditorDocument(document)[0]?.path).toBe("title");
  });

  it("enforces global collection limits", () => {
    const document = createResumeDocument();
    document.sections = Array.from(
      { length: DOCUMENT_LIMITS.sections + 1 },
      (_, index) => createSection(index),
    );
    expect(
      validateEditorDocument(document).some((issue) =>
        issue.message.includes("sections"),
      ),
    ).toBe(true);
  });

  it("moves stable IDs and immediately canonicalizes all ordering", () => {
    const document = createResumeDocument();
    const a = createSection(0),
      b = createSection(1);
    a.entries = [createEntry(0), createEntry(1)];
    const movedEntryId = a.entries[1].id;
    a.entries = moveItem(a.entries, movedEntryId, -1);
    document.sections = moveItem([a, b], b.id, -1);
    const normalized = normalizeDocument(document);
    expect(normalized.sections[0].id).toBe(b.id);
    expect(normalized.sections[1].entries[0].id).toBe(movedEntryId);
    expect(normalized.sections.map((section) => section.order)).toEqual([0, 1]);
    expect(normalized.sections[1].entries.map((entry) => entry.order)).toEqual([
      0, 1,
    ]);
    expect(moveItem([a, b], a.id, -1)).toEqual([a, b]);
  });
});
