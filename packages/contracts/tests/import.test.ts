import { expect, it } from "vitest";
import { isImportReviewSnapshot } from "../generated/import";
const id = "01900000-0000-7000-8000-000000000001";
function snapshot() {
  return {
    id,
    baseRevision: 1,
    mappingVersion: 1,
    blocks: [
      {
        source: "Projects",
        page: 1,
        explanation: "Recognized heading.",
        suggestedTarget: "section",
        suggestedValue: "Projects",
        proposedSection: null as number | null,
      },
      {
        source: "Synthetic contribution",
        page: 1,
        explanation: "List hint.",
        suggestedTarget: "bullet",
        suggestedValue: "Synthetic contribution",
        proposedSection: 0,
      },
    ],
    sections: [],
    contacts: { fullName: "", email: "", phone: "", location: "" },
  };
}
it("validates bounded snapshots and rejects unknown authority and malformed source", () => {
  expect(isImportReviewSnapshot(snapshot())).toBe(true);
  expect(isImportReviewSnapshot({ ...snapshot(), owner: "main" })).toBe(false);
  expect(isImportReviewSnapshot({ ...snapshot(), mappingVersion: 2 })).toBe(
    false,
  );
  const bad = snapshot();
  bad.blocks[1].proposedSection = 1;
  expect(isImportReviewSnapshot(bad)).toBe(false);
  bad.blocks[1].proposedSection = 0;
  bad.blocks[0].source = "\ud800";
  expect(isImportReviewSnapshot(bad)).toBe(false);
  bad.blocks[0].source = "x".repeat(30001);
  expect(isImportReviewSnapshot(bad)).toBe(false);
});
it("enforces aggregate extracted text and original block page bounds", () => {
  const value = snapshot();
  value.blocks[0].source = "x".repeat(30000);
  value.blocks[1].source = "x".repeat(20000);
  expect(isImportReviewSnapshot(value)).toBe(true);
  value.blocks[1].source += "x";
  expect(isImportReviewSnapshot(value)).toBe(false);
  value.blocks[1].source = "x";
  value.blocks[1].page = 11;
  expect(isImportReviewSnapshot(value)).toBe(false);
});
