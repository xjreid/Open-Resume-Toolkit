import { describe, expect, it } from "vitest";
import {
  isPublishResumeCommandResponse,
  isResumeWorkspaceCommandResponse,
  isVersionedResumeCommandResponse,
} from "../generated/resume";

const document = {
  schemaVersion: 1,
  documentId: "01992187-74f7-7000-8000-000000000001",
  title: "My Resume",
  contact: {
    fullName: "Example Person",
    email: "example.invalid@example.com",
    phone: "",
    location: "",
    links: [],
  },
  sections: [],
};

describe("resume command response validation", () => {
  it("accepts exact workspace, save, and publish responses", () => {
    const versioned = { revision: 1, document };

    expect(
      isResumeWorkspaceCommandResponse({
        ok: true,
        value: { draft: versioned, latestPublished: null },
      }),
    ).toBe(true);
    expect(
      isVersionedResumeCommandResponse({ ok: true, value: versioned }),
    ).toBe(true);
    expect(
      isPublishResumeCommandResponse({
        ok: true,
        value: { draftRevision: 1, published: versioned },
      }),
    ).toBe(true);
  });

  it("rejects unknown fields and invalid revisions", () => {
    expect(
      isResumeWorkspaceCommandResponse({
        ok: true,
        value: { draft: null, latestPublished: null, secret: "unexpected" },
      }),
    ).toBe(false);
    expect(
      isVersionedResumeCommandResponse({
        ok: true,
        value: { revision: 0, document },
      }),
    ).toBe(false);
  });
});

it("reads v2 dates and stable links while rejecting mixed shapes and unsupported precision", () => {
  const value = {
    ...document,
    schemaVersion: 2,
    contact: {
      ...document.contact,
      links: [
        {
          id: "01992187-74f7-7000-8000-000000000002",
          order: 0,
          label: "Portfolio",
          url: "https://example.org",
        },
      ],
    },
    sections: [
      {
        id: "01992187-74f7-7000-8000-000000000003",
        order: 0,
        heading: "Experience",
        entries: [
          {
            id: "01992187-74f7-7000-8000-000000000004",
            order: 0,
            heading: "Synthetic role",
            subheading: "",
            dateRange: "",
            location: "",
            fields: [],
            bullets: [],
            links: [],
            dates: [
              {
                id: "01992187-74f7-7000-8000-000000000005",
                order: 0,
                label: "",
                start: { year: 2020, month: null, expected: false },
                end: { kind: "present" },
              },
            ],
          },
        ],
      },
    ],
  };
  const accepts = (document: unknown) =>
    isVersionedResumeCommandResponse({
      ok: true,
      value: { revision: 2, document },
    });
  expect(accepts(value)).toBe(true);
  for (const update of [
    (v: typeof value) => {
      v.schemaVersion = 1;
    },
    (v: typeof value) => {
      v.schemaVersion = 3;
    },
    (v: typeof value) => {
      v.contact.links[0].order = 1;
    },
    (v: typeof value) => {
      v.contact.links[0].id = "not-an-id";
    },
    (v: typeof value) => {
      v.contact.links[0].id = v.documentId;
    },

    (v: typeof value) => {
      v.sections[0].entries[0].dateRange = "conflicting text";
    },
    (v: typeof value) => {
      v.sections[0].entries[0].dates[0].start.year = 10000;
    },
  ]) {
    const invalid = structuredClone(value);
    update(invalid);
    expect(accepts(invalid)).toBe(false);
  }
  const mixed = structuredClone(value);
  Reflect.deleteProperty(mixed.contact.links[0], "id");
  expect(accepts(mixed)).toBe(false);
  const missingDates = structuredClone(value);
  Reflect.deleteProperty(missingDates.sections[0].entries[0], "dates");
  expect(accepts(missingDates)).toBe(false);
});
