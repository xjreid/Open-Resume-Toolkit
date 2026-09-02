import type {
  Bullet,
  ResumeDocument,
  ResumeEntry,
  ResumeSection,
} from "@ort/contracts/resume";

export function createResumeDocument(): ResumeDocument {
  return {
    schemaVersion: 1,
    documentId: createEntityId(),
    title: "My Resume",
    contact: {
      fullName: "",
      email: "",
      phone: "",
      location: "",
      links: [],
    },
    sections: [],
  };
}

export function createSection(order: number): ResumeSection {
  return {
    id: createEntityId(),
    order,
    heading: "New section",
    entries: [],
  };
}

export function createEntry(order: number): ResumeEntry {
  return {
    id: createEntityId(),
    order,
    heading: "",
    subheading: "",
    dateRange: "",
    location: "",
    fields: [],
    bullets: [],
    links: [],
  };
}

export function createBullet(order: number): Bullet {
  return { id: createEntityId(), order, text: "" };
}

export function normalizeDocument(document: ResumeDocument): ResumeDocument {
  return {
    ...document,
    sections: document.sections.map((section, sectionOrder) => ({
      ...section,
      order: sectionOrder,
      entries: section.entries.map((entry, entryOrder) => ({
        ...entry,
        order: entryOrder,
        fields: entry.fields.map((field, fieldOrder) => ({
          ...field,
          order: fieldOrder,
        })),
        bullets: entry.bullets.map((bullet, bulletOrder) => ({
          ...bullet,
          order: bulletOrder,
        })),
      })),
    })),
  };
}

export function createEntityId(now = Date.now()): string {
  if (!Number.isSafeInteger(now) || now < 0 || now >= 2 ** 48) {
    throw new Error("Cannot create a bounded UUIDv7 timestamp");
  }
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  let timestamp = now;
  for (let index = 5; index >= 0; index -= 1) {
    bytes[index] = timestamp % 256;
    timestamp = Math.floor(timestamp / 256);
  }
  bytes[6] = 0x70 | (bytes[6] & 0x0f);
  bytes[8] = 0x80 | (bytes[8] & 0x3f);

  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0"));
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex
    .slice(6, 8)
    .join("")}-${hex.slice(8, 10).join("")}-${hex.slice(10).join("")}`;
}
