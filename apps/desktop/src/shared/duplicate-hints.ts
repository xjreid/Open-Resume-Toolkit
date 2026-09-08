import type { ResumeDocument, ResumeEntry } from "@ort/contracts/resume";

function content(entry: ResumeEntry) {
  return {
    heading: entry.heading.trim(),
    subheading: entry.subheading.trim(),
    location: entry.location.trim(),
    dateRange: entry.dateRange.trim(),
    fields: entry.fields
      .filter((x) => x.label.trim() || x.value.trim())
      .map((x) => ({
        label: x.label.trim(),
        value: x.value.trim(),
        isSkill: x.isSkill,
      })),
    bullets: entry.bullets.map((x) => x.text.trim()).filter(Boolean),
    links: entry.links
      .filter((x) => x.label.trim() || x.url.trim())
      .map((x) => ({ label: x.label.trim(), url: x.url.trim() })),
    dates: (entry.dates ?? [])
      .filter((x) => x.start || x.end)
      .map((x) => ({ label: x.label.trim(), start: x.start, end: x.end })),
  };
}

// Advisory exact-content matches only. Never merges, rewrites, or blocks saving.
export function duplicateEntryGroups(document: ResumeDocument) {
  const groups = new Map<
    string,
    { sectionId: string; entryId: string; label: string }[]
  >();
  for (const section of document.sections) {
    for (const entry of section.entries) {
      const value = content(entry);
      if (!Object.values(value).some((item) => item.length > 0)) continue;
      const fingerprint = JSON.stringify(value);
      const group = groups.get(fingerprint) ?? [];
      group.push({
        sectionId: section.id,
        entryId: entry.id,
        label: `${section.heading.trim() || `Section ${section.order + 1}`}: ${entry.heading.trim() || `Entry ${entry.order + 1}`}`,
      });
      groups.set(fingerprint, group);
    }
  }
  return [...groups.values()].filter((group) => group.length > 1);
}
