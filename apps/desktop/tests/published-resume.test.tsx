import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PublishedResume } from "../src/shared/App";
import {
  createEntry,
  createNamedField,
  createResumeDocument,
  createSection,
} from "../src/shared/resume-editor";

describe("published snapshot review", () => {
  it("escapes content and displays links without creating navigation authority", () => {
    const document = createResumeDocument();
    document.title = "<script>alert('synthetic')</script>";
    document.contact.links = [{ label: "Website", url: "https://example.com" }];
    const section = createSection(0);
    const entry = createEntry(0);
    entry.fields = [
      {
        ...createNamedField(0),
        label: "Language",
        value: "Rust",
        isSkill: true,
      },
    ];
    section.entries = [entry];
    document.sections = [section];
    const html = renderToStaticMarkup(<PublishedResume document={document} />);
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>");
    expect(html).not.toContain("href=");
    expect(html).toContain("https://example.com");
    expect(html).toContain("Language (skill)");
    expect(html).toContain("Rust");
  });
});
