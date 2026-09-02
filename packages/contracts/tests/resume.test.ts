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
