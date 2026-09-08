import type { ResumeDocument } from "@ort/contracts/resume";

export function validationDestination(document: ResumeDocument, path: string) {
  if (path === "title" || path.startsWith("contact."))
    return { part: "contact", label: "Contact and resume details" };
  for (const section of document.sections) {
    const label = section.heading.trim() || `Section ${section.order + 1}`;
    if (path === `section.${section.id}.heading`)
      return { part: section.id, label };
    for (const entry of section.entries) {
      if (
        path.startsWith(`entry.${entry.id}.`) ||
        entry.bullets.some((item) => path === `bullet.${item.id}`) ||
        entry.fields.some((item) => path.startsWith(`field.${item.id}.`)) ||
        entry.dates?.some(
          (item) =>
            path === `date.${item.id}` || path === `date.${item.id}.label`,
        )
      )
        return {
          part: section.id,
          entryId: entry.id,
          label: `${label}: ${entry.heading.trim() || `Entry ${entry.order + 1}`}`,
        };
    }
  }
  return null;
}

export function focusValidationField(root: HTMLElement | null, path: string) {
  const field = Array.from(
    root?.querySelectorAll<HTMLElement>("[data-validation-path]") ?? [],
  ).find((item) => item.dataset.validationPath === path);
  // Native details stay mounted while collapsed; reveal every enclosing group
  // before moving focus so error navigation cannot target an invisible field.
  for (
    let ancestor = field?.parentElement;
    ancestor && ancestor !== root;
    ancestor = ancestor.parentElement
  ) {
    if (ancestor.tagName === "DETAILS")
      (ancestor as HTMLDetailsElement).open = true;
  }
  const input = field?.querySelector<HTMLElement>(
    '[aria-invalid="true"]:not(:disabled), input:not(:disabled), textarea:not(:disabled), select:not(:disabled)',
  );
  // Prefer the invalid endpoint within a date group over its optional label.
  (
    field?.querySelector<HTMLElement>('[aria-invalid="true"]:not(:disabled)') ??
    input
  )?.focus();
}

export interface ValidationFocusRequest {
  path: string;
  entryId?: string;
  sequence: number;
}
