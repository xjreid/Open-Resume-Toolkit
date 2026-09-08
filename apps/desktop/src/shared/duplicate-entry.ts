import type { ResumeEntry } from "@ort/contracts/resume";
import { createEntityId } from "./resume-editor";

// Copy factual content exactly. Every repeated entity in the new entry receives
// its own identity; the original entry and nested values are never mutated.
export function duplicateEntry(entry: ResumeEntry): ResumeEntry {
  return {
    ...entry,
    id: createEntityId(),
    fields: entry.fields.map((field) => ({ ...field, id: createEntityId() })),
    bullets: entry.bullets.map((bullet) => ({
      ...bullet,
      id: createEntityId(),
    })),
    links: entry.links.map((link) => ({
      ...link,
      ...(link.id ? { id: createEntityId() } : {}),
    })),
    ...(entry.dates === undefined
      ? {}
      : {
          dates: entry.dates.map((date) => ({
            ...date,
            id: createEntityId(),
            start: date.start ? { ...date.start } : null,
            end:
              date.end?.kind === "date"
                ? { kind: "date" as const, value: { ...date.end.value } }
                : date.end
                  ? { kind: "present" as const }
                  : null,
          })),
        }),
  };
}
