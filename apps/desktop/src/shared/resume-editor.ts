import type {
  Bullet,
  Link,
  ResumeDocument,
  ResumeEntry,
  ResumeSection,
  NamedField,
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

export function createNamedField(order: number): NamedField {
  return { id: createEntityId(), order, label: "", value: "", isSkill: false };
}

export function moveItem<T extends { id: string }>(
  items: T[],
  id: string,
  direction: -1 | 1,
): T[] {
  const index = items.findIndex((item) => item.id === id);
  const destination = index + direction;
  if (index < 0 || destination < 0 || destination >= items.length) return items;
  const result = [...items];
  [result[index], result[destination]] = [result[destination], result[index]];
  return result;
}

export function normalizeDocument(document: ResumeDocument): ResumeDocument {
  return {
    ...document,
    ...(document.schemaVersion === 2
      ? {
          contact: {
            ...document.contact,
            links: normalizeLinks(document.contact.links),
          },
        }
      : {}),
    sections: document.sections.map((section, sectionOrder) => ({
      ...section,
      order: sectionOrder,
      entries: section.entries.map((entry, entryOrder) => ({
        ...entry,
        ...(document.schemaVersion === 2
          ? {
              dates: (entry.dates ?? []).map((date, order) => ({
                ...date,
                order,
              })),
              links: normalizeLinks(entry.links),
            }
          : {}),
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

function normalizeLinks(links: Link[]): Link[] {
  return links.map((link, order) => ({
    ...link,
    id: link.id ?? createEntityId(),
    order,
  }));
}

// Called only by a future explicit edit/upgrade action, never during load or
// publication display. Free-text dates retain every byte; no precision is guessed.
export function upgradeDocumentV2(document: ResumeDocument): ResumeDocument {
  if (document.schemaVersion !== 1 && document.schemaVersion !== 2)
    throw new Error("Unsupported resume schema; keep the original document.");
  return normalizeDocument({ ...document, schemaVersion: 2 });
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
