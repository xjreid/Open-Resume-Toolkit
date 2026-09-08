import type { ImportChoices, ImportChoice } from "@ort/contracts/import";
// Presentation-only review state. These are not IPC contracts or save payloads.
// Native review must validate decisions against its retained proposal/revision.
export type ReviewField = "fullName" | "email" | "phone" | "location";
export type ReviewTarget = "section" | "text" | "bullet" | ReviewField;
export type ReviewChoice = {
  status: "pending" | "accept" | "reject";
  target: ReviewTarget;
  value: string;
  sectionId: string;
  proposedSection: number | null;
  newSection: string;
  contactMode: "fillEmpty" | "replace" | "keepExisting";
};
export type ReviewBlock = {
  source: string;
  page: number | null;
  explanation: string;
  suggestedTarget: ReviewTarget;
  suggestedValue: string;
  proposedSection?: number | null;
};
export type ReviewSection = { id: string; heading: string };
export type ReviewSession = {
  id: string;
  baseRevision: number;
  blocks: readonly ReviewBlock[];
  sections: readonly ReviewSection[];
  contacts: Readonly<Record<ReviewField, string>>;
};
export const REVIEW_TARGETS: readonly { id: ReviewTarget; label: string }[] = [
  { id: "section", label: "Section heading" },
  { id: "text", label: "Entry text" },
  { id: "bullet", label: "Bullet" },
  { id: "fullName", label: "Full name" },
  { id: "email", label: "Email" },
  { id: "phone", label: "Phone" },
  { id: "location", label: "Location" },
];
export function initialReview(session: ReviewSession): ReviewChoice[] {
  return session.blocks.map((block) => ({
    status: "pending",
    target: block.suggestedTarget,
    value: block.suggestedValue,
    sectionId: "",
    proposedSection: block.proposedSection ?? null,
    newSection: "",
    contactMode: "fillEmpty",
  }));
}
export function isReviewContact(target: ReviewTarget): target is ReviewField {
  return !["section", "text", "bullet"].includes(target);
}
export function reviewProblem(
  session: ReviewSession,
  choice: ReviewChoice,
  choices: readonly ReviewChoice[] = [],
): string | null {
  if (choice.status === "pending")
    return "Choose Keep or Reject for this block.";
  if (choice.status === "reject") return null;
  if (isReviewContact(choice.target) && choice.contactMode === "keepExisting")
    return null;
  if (!choice.value.trim()) return "Enter a value or reject this block.";
  if (isReviewContact(choice.target)) {
    const existing = session.contacts[choice.target];
    if (existing.trim() && choice.contactMode === "fillEmpty")
      return "Choose whether to replace or keep the existing contact value.";
  } else if (choice.proposedSection !== null) {
    const section = choices[choice.proposedSection];
    if (
      choice.target === "section" ||
      !section ||
      section.status !== "accept" ||
      section.target !== "section"
    )
      return "Choose a section heading that is still kept in this review.";
  } else if (choice.sectionId) {
    if (!session.sections.some((section) => section.id === choice.sectionId))
      return "Choose an available section.";
  } else if (choice.target !== "section" && !choice.newSection.trim()) {
    return "Name a new section or choose an existing section.";
  }
  return null;
}

// Converts presentation state without sending original text, owner or revision.
// Native validation remains authoritative; this helper grants no save authority.
export function importChoices(choices: readonly ReviewChoice[]): ImportChoices {
  return {
    choices: choices.map((choice): ImportChoice => {
      if (choice.status === "pending") return { kind: "pending" };
      if (choice.status === "reject") return { kind: "reject" };
      if (isReviewContact(choice.target))
        return {
          kind: "contact",
          field: choice.target,
          value: choice.value,
          mode: choice.contactMode,
        };
      if (choice.target === "section")
        return {
          kind: "section",
          heading: choice.value,
          target: choice.sectionId
            ? { kind: "existing", id: choice.sectionId }
            : { kind: "new" },
        };
      return {
        kind: "text",
        text: choice.value,
        bullet: choice.target === "bullet",
        target:
          choice.proposedSection !== null
            ? { kind: "proposed", index: choice.proposedSection }
            : choice.sectionId
              ? { kind: "existing", id: choice.sectionId }
              : { kind: "new", heading: choice.newSection },
      };
    }),
  };
}
