import {
  DOCUMENT_LIMITS as limits,
  type ResumeDocument,
} from "@ort/contracts/resume";

export interface ValidationIssue {
  path: string;
  message: string;
}

export function documentUsage(document: ResumeDocument) {
  const entries = document.sections.flatMap((section) => section.entries);
  return {
    sections: document.sections.length,
    entries: entries.length,
    bullets: entries.reduce((sum, entry) => sum + entry.bullets.length, 0),
    links:
      document.contact.links.length +
      entries.reduce((sum, entry) => sum + entry.links.length, 0),
    skills: entries.reduce(
      (sum, entry) =>
        sum + entry.fields.filter((field) => field.isSkill).length,
      0,
    ),
  };
}

// Responsive UI feedback only. Rust remains the authoritative trust boundary.
export function validateEditorDocument(
  document: ResumeDocument,
): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  let characters = 0;
  function field(
    path: string,
    value: string,
    maximum: number = limits.fieldCharacters,
  ) {
    const length = Array.from(value).length;
    characters += length;
    if (length > maximum)
      issues.push({ path, message: `Use at most ${maximum} characters.` });
  }
  function links(path: string, values: ResumeDocument["contact"]["links"]) {
    values.forEach((link, index) => {
      field(`${path}.${index}.label`, link.label);
      const urlPath = `${path}.${index}.url`;
      field(urlPath, link.url);
      try {
        const url = new URL(link.url);
        if (!["http:", "https:", "mailto:"].includes(url.protocol))
          throw new Error("scheme");
      } catch {
        issues.push({
          path: urlPath,
          message: "Enter a complete HTTP, HTTPS, or mailto URL.",
        });
      }
    });
  }
  if (!document.title.trim())
    issues.push({ path: "title", message: "Enter a resume title." });
  field("title", document.title);
  for (const key of ["fullName", "email", "phone", "location"] as const)
    field(`contact.${key}`, document.contact[key]);
  links("contact.links", document.contact.links);
  for (const section of document.sections) {
    const path = `section.${section.id}`;
    if (!section.heading.trim())
      issues.push({
        path: `${path}.heading`,
        message: "Enter a section heading.",
      });
    field(`${path}.heading`, section.heading);
    for (const entry of section.entries) {
      const entryPath = `entry.${entry.id}`;
      for (const key of [
        "heading",
        "subheading",
        "dateRange",
        "location",
      ] as const)
        field(`${entryPath}.${key}`, entry[key]);
      for (const item of entry.fields) {
        field(`field.${item.id}.label`, item.label);
        field(`field.${item.id}.value`, item.value);
      }
      for (const bullet of entry.bullets)
        field(`bullet.${bullet.id}`, bullet.text, limits.bulletCharacters);
      links(`${entryPath}.links`, entry.links);
    }
  }
  const usage = documentUsage(document);
  for (const key of [
    "sections",
    "entries",
    "bullets",
    "links",
    "skills",
  ] as const) {
    if (usage[key] > limits[key])
      issues.push({
        path: "document",
        message: `Use at most ${limits[key]} ${key} across this resume.`,
      });
  }
  if (characters > limits.totalCharacters)
    issues.push({
      path: "document",
      message: `The resume exceeds ${limits.totalCharacters} total characters.`,
    });
  if (
    new TextEncoder().encode(JSON.stringify(document)).length >
    limits.serializedBytes
  ) {
    issues.push({
      path: "document",
      message: "The structured resume exceeds the storage size limit.",
    });
  }
  return issues;
}
