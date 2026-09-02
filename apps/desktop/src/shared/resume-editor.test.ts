import { describe, expect, it, vi } from "vitest";
import {
  createEntityId,
  createResumeDocument,
  normalizeDocument,
} from "./resume-editor";

describe("resume editor model", () => {
  it("creates UUIDv7 identifiers using cryptographic randomness", () => {
    vi.stubGlobal("crypto", {
      getRandomValues: (bytes: Uint8Array) => bytes.fill(0xab),
    });
    const id = createEntityId(1_725_000_000_000);

    expect(id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
    vi.unstubAllGlobals();
  });

  it("creates a bounded empty document and restores canonical order", () => {
    const document = createResumeDocument();
    document.sections = [
      { id: createEntityId(), order: 8, heading: "Skills", entries: [] },
      { id: createEntityId(), order: 9, heading: "Experience", entries: [] },
    ];

    expect(
      normalizeDocument(document).sections.map((section) => section.order),
    ).toEqual([0, 1]);
  });
});
