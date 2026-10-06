import type { NamedField, ResumeEntry } from "@ort/contracts/resume";
import { createNamedField } from "./resume-editor";
// Legacy labels are interpreted at this boundary; stored IDs and values never change.
import { PARAGRAPH_FIELD_LABEL } from "@ort/contracts/resume";
export { PARAGRAPH_FIELD_LABEL };
export type FieldRole = "details" | "extra" | "paragraph";
export function fieldRole(field: NamedField): FieldRole {
  if (field.label === PARAGRAPH_FIELD_LABEL) return "paragraph";
  return field.label.trim().toLowerCase() === "extra" ? "extra" : "details";
}
export function fieldsForRole(
  entry: ResumeEntry,
  role: FieldRole,
): NamedField[] {
  return entry.fields.filter((field) => fieldRole(field) === role);
}
export function updateEntryField(
  entry: ResumeEntry,
  field: NamedField | undefined,
  label: string,
  value: string,
): ResumeEntry {
  const fields = field
    ? entry.fields.map((candidate) =>
        candidate.id === field.id ? { ...candidate, value } : candidate,
      )
    : [
        ...entry.fields,
        { ...createNamedField(entry.fields.length), label, value },
      ];
  return { ...entry, fields };
}
